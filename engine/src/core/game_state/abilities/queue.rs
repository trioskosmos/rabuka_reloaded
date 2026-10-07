use crate::core::game_state::GameState;
use crate::core::constants::CountCast;
use smallvec::SmallVec;

impl GameState {
    fn process_player_abilities(&mut self, raw_player_id: &str) {
        self.process_player_abilities_depth(raw_player_id)
    }

    /// Recursive auto-ability resolution with a bounded re-entry depth.
    ///
    /// `process_player_abilities` re-enters itself from its post-loop batch
    /// scan (§9.5.3.1 loopback) whenever a watcher enqueues further abilities.
    /// The per-ability `reprocess_counts` guard is local to each invocation, so
    /// runaway re-triggering would otherwise recurse without bound and overflow
    /// the stack. `max_auto_recursion` caps the depth as a last-resort safety
    /// net; well-formed games resolve in a handful of levels.
    fn process_player_abilities_depth(&mut self, raw_player_id: &str) {
        let player_id = match raw_player_id {
            "player1" => "p1",
            "player2" => "p2",
            other => other,
        };
        // §4.14: the guard's key space is the abilities
        // in one drain pass — a handful — so a SmallVec
        // with a linear scan beats a HashMap (and its
        // first-insert allocation) per invocation.
        let mut reprocess_counts: SmallVec<[((i16, usize), u8); 8]> =
            SmallVec::new();
        let mut batch_rerun = true;
        while batch_rerun {
            batch_rerun = false;
            loop {
            if !self.ability_queue.is_idle() {
                break;
            }

            // Snapshot queue length before resolution. Entries at indices >= pre_len
            // are freshly triggered (each_time watchers) by the current resolution
            // and must be drained depth-first (§9.5.3.2→§9.5.3.1 loopback).
            let pre_len =
                self.depth_first_cutoff
                    .unwrap_or_else(|| self.ability_queue.len().u16_count())
            as usize;
            self.depth_first_cutoff = None;

            let mut available_indices = (0..pre_len).filter(|&i| {
                self.ability_queue.is_entry_available(i)
                    && self.ability_queue.entry_player_id(i) == Some(player_id)
            });
            let first = available_indices.next();
            let second = available_indices.next();

            if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                let debug_indices: Vec<usize> = (0..pre_len)
                    .filter(|&i| {
                        self.ability_queue.is_entry_available(i)
                            && self.ability_queue.entry_player_id(i) == Some(player_id)
                    })
                    .collect();
                log::trace!("[QUEUE_SCAN] owner={} cutoff={} available_indices={:?}", player_id, pre_len, debug_indices);
            }

            let Some(idx) = first else {
                break;
            };

            if let Some(second) = second {
                let options = core::iter::once(idx)
                    .chain(core::iter::once(second))
                    .chain(available_indices)
                    .filter_map(|idx| {
                        let entry = self.ability_queue.get_entry(idx)?;
                        let cid = entry.card_id.unwrap_or(0);
                        let card_name = self
                            .card_database
                            .get_card(cid)
                            .map(|c| c.name.to_string())
                            .unwrap_or_else(|| entry.card_no.to_string());
                        Some(crate::ability::types::AutoAbilityOption {
                            card_name,
                            ability_text: entry.ability.full_text.clone(),
                            queue_index: idx,
                            card_id: entry.card_id,
                        })
                    })
                    .collect();

                let choice = crate::ability::types::Choice::SelectAutoAbility {
                    player_id: player_id.to_string(),
                    options,
                    description:
                        "複数の自動能力が同時に発動しました。使用する順番を選択してください。"
                            .to_string(),
                    description_en: Some("Multiple auto abilities triggered simultaneously. Choose the order to use them.".to_string()),
                    description_ja: Some("複数の自動能力が同時に発動しました。使用する順番を選択してください。".to_string()),
                };

                self.ability_queue.pause_for_auto_ability_choice(choice);
                break;
            }
            self.ability_queue.promote_entry_by_abs(idx);
            if !self.ability_queue.start_next() {
                break;
            }

            // Infinite loop guard: track how many times the same ability is processed
            if let Some(entry) = self.ability_queue.current_entry() {
                let key = (entry.card_id.unwrap_or(-1), entry.ability_index);
                let idx = match reprocess_counts
                    .iter()
                    .position(|(k, _)| *k == key)
                {
                    Some(idx) => idx,
                    None => {
                        reprocess_counts.push((key, 0));
                        reprocess_counts.len() - 1
                    }
                };
                reprocess_counts[idx].1 += 1;
                if reprocess_counts[idx].1 > 5 {
                    let card_name = entry
                        .card_id
                        .and_then(|id| self.card_database.get_card(id))
                        .map(|c| c.name.to_string())
                        .unwrap_or_default();
                    log::error!(
                        "[PCA_INFINITE_LOOP] card={} ({}) ability=\"{}\" processed {} times",
                        card_name,
                        entry.card_no,
                        entry.ability.full_text,
                        reprocess_counts[idx].1
                    );
                    break;
                }
            }

            self.process_current_ability();
            let had_recent_moves = self.recently_moved_cards.is_some();
            let had_recent_appearances = !self.recently_appeared_cards.is_empty();
            self.clear_recently_moved_batch();
            self.recently_appeared_cards.clear();
            // Deferred so a cost paid here (a state change itself) survives
            // until the watcher behind the pending choice reads it.
            self.clear_state_change_window_deferred("after process_current_ability");
            // Save flag for the post-loop batch scan below;
            // process_current_ability's internal scan (line 742) already ran
            // before the clear above, so each_time watchers from the
            // just-resolved effect were caught. This post-loop scan catches
            // batch movements (look_and_select, etc.) that finalize card
            // movement outside individual ability resolution.
            if had_recent_moves {
                self.set_recently_moved_batch(SmallVec::new(), None);
            }
            if had_recent_appearances {
                self.recently_appeared_cards.push(-1);
            }
            if self.has_pending_choice() {
                break;
            }

            // Depth-first drain: newly-triggered each_time watchers at indices >= cutoff
            // (§9.5.3.2→§9.5.3.1 loopback) resolve immediately before the next stale entry.
            // The range widens dynamically as sub-resolutions queue deeper entries.
            let cutoff = pre_len;
            let mut drain_iters = 0;
            while !self.has_pending_choice() && self.ability_queue.is_idle() {
                drain_iters += 1;
                if drain_iters > 50 {
                    log::error!(
                        "[PCA_DRAIN_LIMIT] each_time drain exceeded 50 iterations player={}",
                        player_id
                    );
                    break;
                }
                let new_idx = (cutoff..self.ability_queue.len()).find(|&i| {
                    self.ability_queue.is_entry_available(i)
                        && self.ability_queue.entry_player_id(i) == Some(player_id)
                });
                match new_idx {
                    Some(idx) => {
                        self.ability_queue.set_current_entry(idx);
                        if !self.ability_queue.start_next() {
                            break;
                        }
                        self.process_current_ability();
                        // Sub-resolution may queue deeper entries -- while loop catches them
                        // on next iteration (widening range from pre_len..len).
                        if self.has_pending_choice() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
        // Q86 / Q252 / Rule 9.5.3.1: Post-loop scan for batch-triggered abilities
        //
        // After the main ability loop, we scan for auto-abilities triggered by
        // batch movements from look_and_select or other compound effects that
        // finalize card movement outside individual ability resolution.
        //
        // Trigger types: each_time:discard, each_time:hand_to_discard,
        //   each_time:any_to_discard, each_time:energy_placed
        //
        // Q86: After look-and-select resolves, if all looked-at cards go to
        //   discard and deck becomes empty, the batch movement triggers
        //   each_time:discard watchers. The post-loop scan catches these.
        //
        // Q252: "When multiple live cards go to discard simultaneously, can
        //   I put all of them back with one trigger?" -- No, only 1 card per
        //   trigger instance. The batch scan fires the ability once per batch,
        //   and the ability selects 1 card from the batch.
        let moved_marker = self.recently_moved_cards.is_some();
        let appeared_marker = !self.recently_appeared_cards.is_empty();
        if moved_marker || self.last_energy_placed_by_effect() || appeared_marker {
            // Built from `batch_movements`, NOT the `recently_moved_cards` shadow
            // field — which is why the batch is a parameter and not read from
            // `self`. Collected before the `&mut self` call so the borrow ends.
            let moved: smallvec::SmallVec<[i16; 4]> = self
                .batch_movements
                .iter()
                .map(|m| m.moved_card_id)
                .collect();
            self.trigger_auto_abilities_for_player_with_event(player_id, &moved);
            self.clear_recently_moved_batch();
            self.recently_appeared_cards.clear();
            self.clear_state_change_window("after post-loop TAS scan");
            // Re-enter the loop to process any abilities just enqueued
            // by the watcher scan (e.g. Hazuki Ren each_time after discard).
            // Keep this_batch_triggered_ability_ids alive through the recursive
            // call so the same ability isn't enqueued twice from stale events.
            if !self.has_pending_choice() {
                batch_rerun = true;
            }
            self.batch_movements.clear();
            self.position_change_events.clear();
            self.this_batch_triggered_ability_ids.clear();
        }
        } // end while batch_rerun
        // Bug B: the re-scan guard's scope is ONE batch. Once drained, the key
        // must not survive into the next player action, or it vetoes that
        // action's fresh event. See docs/JIDOU_COMBINATION_WORK.md §6.
        self.just_completed_ability_key = None;
        self.just_completed_moved.clear();
    }

    pub fn process_pending_auto_abilities(&mut self, raw_player_id: &str) {
        #[cfg(not(feature = "no_std"))]
        let _timer = crate::timer::Timer::start("trig::process_pending");
        // Nothing queued means nothing to resolve. This function is reached ~3.8x per
        // action (every phase boundary, every gate re-check, every resume) and the
        // overwhelming majority of those calls find an empty queue — each one otherwise
        // entered `process_player_abilities_depth` TWICE, walked its re-entry loop, and
        // ran `clear_completed` over nothing.
        //
        // Both `process_player_abilities` calls are no-ops on an empty queue: the
        // depth-first loop finds no available index, and `clear_completed` has nothing to
        // clear. So this is behaviour-preserving, not a shortcut past work.
        // Both the empty-queue short-circuit below and the 自動 index were written, A/B'd, and
        // REMOVED as losses. The record is here because both were large-sounding wins that
        // measurement killed, and because the reason is the useful part.
        //
        // The short-circuit looked like a 49% win (5.05 s -> 3.42 s, 39,050 -> 58,103
        // aps) and passed the full suite. It was never A/B'd at the time, because the only
        // instrument available compared across BUILDS, which cannot work here:
        // `CardDatabase.cards` is a `HashMap` on `RandomState`, so game trajectories differ
        // between PROCESSES — the same binary and seed gave 175,441 and then 169,393 total
        // actions. Sound A/B (one process, interleaved, `total_actions` verified equal)
        // measures 25,908 aps with the short-circuit against 26,402 without.
        //
        // The 自動 index is the same story: 23,674 aps with, 26,402 without.
        //
        // The cause of both false wins was a bug in the ENGINE, not the benchmark:
        // `PCA_CALLS`, the runaway-loop guard in `process_current_ability`, was a
        // process-global `static` nothing ever reset, so after 200k abilities resolved
        // cumulatively every later call cleared the queue and returned. Warmup alone
        // resolves far past that, so the entire measured sweep ran with ability resolution
        // OFF — and an engine that resolves nothing has very little 自動 scanning to do.
        // Both "optimisations" were being credited for work the broken guard had already
        // deleted. The guard is now per-chain; see `process_current_ability`.
        if self.ability_queue.is_empty() {
            return;
        }
        // A fresh settle is a fresh runaway-loop budget. See `process_current_ability`.
        self.pca_calls_in_chain = 0;
        let active_player_id = match raw_player_id {
            "player1" => "p1",
            "player2" => "p2",
            other => other,
        };
        log::trace!("[QUEUE_SCAN] raw={} active={} queue_len_before={}", raw_player_id, active_player_id, self.ability_queue.len());
        // Rule 9.5.3.2: Active player resolves ALL their standby abilities first
        // (one at a time, back to rule processing between each)
        self.process_player_abilities(active_player_id);
        log::trace!("[QUEUE_SCAN] after active: queue_len={} has_choice={}", self.ability_queue.len(), self.has_pending_choice());
        if self.has_pending_choice() {
            return;
        }

        // Rule 9.5.3.3: Then non-active player resolves ALL theirs
        let non_active_id = self
            .ability_queue
            .pending_entries_iter()
            .find(|e| e.player_id != active_player_id)
            .map(|e| e.player_id.clone())
            .unwrap_or_default();
        if !non_active_id.is_empty() {
            self.process_player_abilities(&non_active_id);
        }
        self.ability_queue.clear_completed();
    }
}
