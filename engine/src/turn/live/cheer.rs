use crate::card::{BaseHeart, BladeColor, CardDatabase, HeartColor};
use crate::core::game_modifiers::ModifierEntry;
use crate::game_state::GameState;
use crate::turn::TurnEngine;
use crate::types::{
    BladeSource, HeartSource, LivePerformanceData, SourceType, YellCardResult,
};
use crate::HashMap;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use super::scoring::EMPTY_H8;
pub struct YellIconOutcome {
    pub blade_hearts: [u8; 8],
    pub note_icons: u8,
    pub draw_icons: u8,
}

/// Rule 8.3.11 recolor mapping for set_blade_type effects.
pub(crate) fn blade_color_to_heart(bc: BladeColor) -> HeartColor {
    match bc {
        BladeColor::Peach => HeartColor::Heart01,
        BladeColor::Red => HeartColor::Heart02,
        BladeColor::Yellow => HeartColor::Heart03,
        BladeColor::Green => HeartColor::Heart04,
        BladeColor::Blue => HeartColor::Heart05,
        BladeColor::Purple => HeartColor::Heart06,
        // ALL blade = "any one color heart" (rule 2.1.1.3), i.e. icon_all
        // (HeartColor::All, index 7) — NOT colorless Heart00.
        BladeColor::All => HeartColor::All,
    }
}

pub(crate) fn blade_type_override_for_stage(
    stage: &[i16],
    blade_type_modifiers: &HashMap<i16, BladeColor>,
) -> Option<BladeColor> {
    stage.iter().find_map(|card_id| {
        if *card_id == -1 {
            None
        } else {
            blade_type_modifiers.get(card_id).copied()
        }
    })
}

/// Process the blade-heart and special-heart icons of ONE yell-revealed card.
///
/// Single source of truth shared by the primary yell (`player_perform_live`)
/// and the re-yell rebuild (`execute_performance_phase`). Implements:
/// - rule 8.3.15.1.1: ALL-blade counts as any one color heart (index 7 wildcard);
/// - rule 8.3.12.1: each draw icon yields 1 draw (`draw_icons`; the caller executes it);
/// - rule 8.4.2.1: each score icon adds +1 to the live total (`cheer_count`);
/// - b_heart07: colorless hearts count double but never satisfy specific colors;
/// - set_blade_type recoloring applies to colored blades only (Draw/Score pass through).
///
/// Heart contributions are added to `owned_hearts` and `total_hearts`. Callers
/// that recompute the heart pool afterwards may pass a scratch `owned_hearts`.
///
/// Public so that out-of-crate tools can compute a card's PRINTED heart
/// breakdown without reimplementing the icon decoding — `src/bin/deck_analyze.rs`
/// needs exactly that to answer "does this deck have blade hearts, and in which
/// colours". Pure: it only reads `card` and writes the three out-params.
pub fn process_yell_revealed_card_icons(
    card: &crate::card::Card,
    override_color: Option<HeartColor>,
    owned_hearts: &mut BaseHeart,
    total_hearts: &mut [u8; 8],
    cheer_count: &mut u8,
) -> YellIconOutcome {
    let mut bh_arr = EMPTY_H8;
    let mut note_icons = 0u8;
    let mut draw_icons = 0u8;

    if let Some(ref bh) = card.blade_heart {
        for (color, count) in &bh.hearts {
            // b_heart07 mechanic: the key parses to HeartColor::Heart00
            // (colorless), and `b_heart07: N` means 2×N colorless hearts.
            // A colorless heart can ONLY be used to replace heart0
            // requirements — never a specific color (heart01-heart06).
            let amount = if *color == HeartColor::Heart00 {
                count * 2
            } else {
                *count
            };
            // Draw/Score special icons are never converted by
            // set_blade_type — they pass through unchanged.
            let effective_color = if matches!(
                *color,
                HeartColor::Heart00 | HeartColor::Draw | HeartColor::Score
            ) {
                *color
            } else {
                override_color.unwrap_or(*color)
            };
            if override_color.is_some() {
                log::debug!(
                    "[BLADE_RECOLOR] original={:?} override={:?} effective={:?}",
                    color,
                    override_color,
                    effective_color
                );
            }
            // Q45: ALL-blade (BAll) can be treated as any color heart.
            // Mapped to HeartColor::All (icon_all, index 7) so the UI
            // displays icon_all.png for BAll yell hearts.
            match effective_color {
                HeartColor::BAll => {
                    *owned_hearts.hearts.entry_or_default(HeartColor::All) += amount;
                    bh_arr[7] += amount;
                    total_hearts[7] += amount;
                }
                HeartColor::Draw => draw_icons += amount,
                // Q44: Each score icon revealed during yell adds 1 to total score.
                HeartColor::Score => {
                    note_icons += amount;
                    *cheer_count += amount;
                }
                other => {
                    let idx = other.index();
                    if idx < 8 {
                        *owned_hearts.hearts.entry_or_default(other) += amount;
                        bh_arr[idx] += amount;
                        total_hearts[idx] += amount;
                    }
                }
            }
        }
    }

    // Special hearts printed on the revealed card itself (e.g. the ドロー icon
    // on Solitude Rain when it is milled back into the deck and revealed).
    if let Some(ref sh) = card.special_heart {
        for (color, count) in &sh.hearts {
            match *color {
                HeartColor::Draw => draw_icons += count,
                HeartColor::Score => {
                    note_icons += count;
                    *cheer_count += count;
                }
                _ => {}
            }
        }
    }

    YellIconOutcome {
        blade_hearts: bh_arr,
        note_icons,
        draw_icons,
    }
}

/// Pre-trigger per-seat live results (docs/TEST_HARDENING_PLAN_2026-08-26.md
/// §4, engine bug #5): ライブ成功時 triggers fire at the TOP of
/// execute_live_victory_determination, but the authoritative verdicts and
/// scores are only computed further down — so conditions like Strawberry
/// Trapper's 「相手が余剰のハートを持たずにライブを成功させていた場合」 saw
/// stale state in every real flow (synthetic-flag tests masked it).
///
/// This records each seat's outcome BEFORE the trigger block runs, using the
/// SAME score formula as the post-trigger totals (extras = 0 at this point,
/// since trigger bonuses land in pX_extra afterwards). The later
/// authoritative writes remain as post-extras truth for display/subsequent
pub(crate) fn record_pretrigger_live_results(
    gs: &mut GameState,
    need_heart_flat: &HashMap<i16, HashMap<crate::card::HeartColor, ModifierEntry>>,
    pre_score_flat: &HashMap<i16, i16>,
) {
    // (won, no_excess) per seat, computed from finalised allocations.
    let mut results = [(false, false); 2];
    let mut recorded = [false; 2];
    for seat_index in 0..2 {
        let pid = if seat_index == 0 {
            gs.player1.id.clone()
        } else {
            gs.player2.id.clone()
        };
        let seat = if seat_index == 0 {
            &gs.player1
        } else {
            &gs.player2
        };
        let Some(snap) = gs
            .performance_snapshots
            .iter()
            .rev()
            .find(|s| s.player_id == pid)
        else {
            continue;
        };
        if snap.lives.is_empty() || seat.live_card_zone.cards.is_empty() {
            continue;
        }
        let mut all_passed = true;
        let mut filled_total = [0u8; 8];
        for (li, l) in snap.lives.iter().enumerate() {
            let Some(card) = gs.card_database.get_card(l.card_id) else {
                all_passed = false;
                break;
            };
            let Some(nh) = card.need_heart.as_ref() else {
                continue;
            };
            let mut filled = [0u8; 8];
            for alloc in &snap.breakdown.allocations {
                if alloc.target_idx == u8::try_from(li).unwrap() {
                    filled[alloc.color as usize] += alloc.amount;
                }
            }
            for (c, val) in filled.iter().enumerate() {
                filled_total[c] += *val;
            }
            let mut required = [0u8; 8];
            for (color, needed) in &nh.hearts {
                required[color.index()] = *needed;
            }
            if let Some(card_mods) = gs.mods.need_heart_modifiers.get(&l.card_id) {
                for (color, me) in card_mods {
                    if me.set != 0 {
                        required[color.index()] = u8::try_from(me.set).unwrap();
                    }
                }
                // The Q115/Q127 set-then-add rule has one definition, in
                // `apply_need_heart_modifiers`. This loop used to restate the
                // additive half of it, which is how the two drifted.
                crate::core::game_modifiers::apply_need_heart_modifiers(
                    &mut required,
                    card_mods,
                );
            }
            // Same acceptance rules as the authoritative PASS/FAIL pass:
            // total coverage, then heart0 bucket, then per-color deficits
            // coverable by icon_all. Additive modifiers apply in i32 with a
            // saturating write-back (mirroring the authoritative pass) so a
            // negative additive cannot wrap into a huge u8 requirement.
            let mut icon_all = filled[7];
            let total_filled: u16 = filled.iter().map(|&v| u16::from(v)).sum();
            let total_required: u16 = required.iter().map(|&v| u16::from(v)).sum();
            let mut ok = total_filled >= total_required;
            if ok && required[0] > 0 {
                let any_hearts: u16 =
                    filled[1..7].iter().map(|&v| u16::from(v)).sum::<u16>() + u16::from(filled[0]);
                if any_hearts + u16::from(icon_all) < u16::from(required[0]) {
                    ok = false;
                } else {
                    let used =
                        u16::from(required[0].saturating_sub(u8::try_from(any_hearts).unwrap()));
                    icon_all = icon_all.saturating_sub(u8::try_from(used).unwrap());
                }
            }
            if ok {
                for idx in 1..7 {
                    if filled[idx] < required[idx] {
                        let deficit = required[idx] - filled[idx];
                        if icon_all >= deficit {
                            icon_all -= deficit;
                        } else {
                            ok = false;
                            break;
                        }
                    }
                }
            }
            if !ok {
                all_passed = false;
            }
        }
        let no_excess = filled_total
            .iter()
            .enumerate()
            .all(|(c, &ft)| snap.total_hearts[c] >= ft && snap.total_hearts[c] - ft == 0);
        // Same score formula as the post-trigger totals (extras are zero
        // here by construction — trigger bonuses land in pX_extra after
        // this point).
        let seat_no = u8::try_from(seat_index).unwrap_or(0);
        let seat = gs.seat_player(seat_no);
        let cheer = gs.seat_cheer_count(seat_no);
        let bonus = gs.seat_score_bonus(seat_no);
        let pre_score = seat.live_card_zone.calculate_live_score(
            &gs.card_database,
            cheer,
            seat.stage_hearts.as_ref(),
            Some(need_heart_flat),
            Some(pre_score_flat),
            bonus,
        );
        let won = all_passed && pre_score > 0;
        results[seat_index] = (won, no_excess);
        recorded[seat_index] = true;
    }
    // Seats that performed no lives are LEFT untouched: nothing was learned
    // about them, and synthetic/legacy flows may have armed them explicitly.
    if recorded[0] {
        gs.p1_live_success_this_turn = results[0].0;
        gs.p1_live_success_no_excess = results[0].0 && results[0].1;
    }
    if recorded[1] {
        gs.p2_live_success_this_turn = results[1].0;
        gs.p2_live_success_no_excess = results[1].0 && results[1].1;
    }
    // Diagnostic detail: why each seat was skipped/written.
    for seat_index in 0..2 {
        let pid = if seat_index == 0 {
            gs.player1.id.clone()
        } else {
            gs.player2.id.clone()
        };
        let seat = if seat_index == 0 {
            &gs.player1
        } else {
            &gs.player2
        };
        let info = gs
            .performance_snapshots
            .iter()
            .rev()
            .find(|s| s.player_id == pid)
            .map(|s| (s.lives.len(), s.total_hearts.to_vec()))
            .unwrap_or((usize::MAX, vec![]));
        log::debug!(
            "[EARLY_SEAT_DETAIL] seat={} lives={:?} zone_cards={} total_hearts={:?}",
            pid,
            info,
            seat.live_card_zone.cards.len(),
            info.1
        );
    }
    log::debug!(
        "[EARLY_SEAT] p1(won={},no_excess={}) p2(won={},no_excess={})",
        results[0].0,
        results[0].1,
        results[1].0,
        results[1].1
    );
}
impl TurnEngine {
    /// Reveal `total_blade` yell cards into the resolution zone.
    /// Q104 / Q100 / Rule 10.2.1: refresh from waitroom when the deck runs
    /// out mid-draw. G8: 恋になりたいAQUARIUM reveals from the deck bottom.
    fn reveal_yell_cards(
        player: &mut crate::player::Player,
        resolution_zone: &mut crate::zones::ResolutionZone,
        total_blade: u8,
        owner: u8,
    ) {
        // Q40: Yell must complete ALL checks — even if hearts are already satisfied,
        // the full blade-count of yell cards is always revealed.
        let from_bottom = player.yell_from_bottom;
        for _ in 0..total_blade {
            if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                player.refresh();
            }
            let card_id = if from_bottom {
                player.main_deck.draw_bottom()
            } else {
                player.main_deck.draw()
            };
            if let Some(card_id) = card_id {
                resolution_zone.add_card_for_owner(card_id, owner);
            }
        }
    }

    /// Process yell-zone cards into results, folding blade hearts into the
    /// owned/total pools. Draw icons are only counted here (Q42 defers the
    /// actual draws until every yell card is revealed).
    /// Returns `(yell_cards, cheer_icon_count, total_draw_icons)`.
    fn process_yell_zone_cards(
        resolution_zone: &crate::zones::ResolutionZone,
        card_db: &CardDatabase,
        override_color: Option<HeartColor>,
        owned_hearts: &mut BaseHeart,
        total_hearts_arr: &mut [u8; 8],
    ) -> (Vec<YellCardResult>, u8, u8) {
        let mut yell_cards = Vec::new();
        let mut cheer_icon_count = 0u8;
        let mut total_draw_icons = 0u8;
        for card_id in &resolution_zone.cards {
            if let Some(card) = card_db.get_card(*card_id) {
                let outcome = process_yell_revealed_card_icons(
                    card,
                    override_color,
                    owned_hearts,
                    total_hearts_arr,
                    &mut cheer_icon_count,
                );
                total_draw_icons += outcome.draw_icons;

                yell_cards.push(YellCardResult {
                    card_id: *card_id,
                    blade_hearts: outcome.blade_hearts,
                    note_icons: outcome.note_icons,
                    draw_icons: outcome.draw_icons,
                    card_no: card_db
                        .get_card(*card_id)
                        .map(|c| crate::types::ArcStr::from(c.card_no.as_ref()))
                        .unwrap_or_default(),
                });
            }
        }
        (yell_cards, cheer_icon_count, total_draw_icons)
    }

    /// Q43: each draw icon revealed during yell draws 1 card (after all yell
    /// cards are revealed). Refreshes from waitroom mid-draw per Q104.
    fn draw_for_yell_icons(player: &mut crate::player::Player, total_draw_icons: u8) {
        for _ in 0..total_draw_icons {
            if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                player.refresh();
            }
            if let Some(new_card) = player.main_deck.draw() {
                player.hand.add_card(new_card);
            }
        }
    }

    pub fn player_perform_live(
        player: &mut crate::player::Player,
        resolution_zone: &mut crate::zones::ResolutionZone,
        _player_id: &str,
        owner: u8,
        card_db: &CardDatabase,
        mods: &crate::core::game_modifiers::GameModifiers,
        cannot_live: bool,
    ) -> LivePerformanceData {
        let blade_modifiers = &mods.blade_modifiers;
        let heart_override = &mods.heart_override;
        let heart_modifiers = &mods.heart_modifiers;
        let blade_type_modifiers = &mods.blade_type_modifiers;
        let orientation_modifiers = &mods.orientation_modifiers;
        let need_heart_modifiers = &mods.need_heart_modifiers;
        let heart_color_multiplier = &mods.heart_color_multiplier;
        let heart_copy = &mods.heart_copy;
        #[cfg(not(feature = "no_std"))]
        let _t = crate::timer::Timer::start("player_perform_live");
        // Q68/Rule: "cannot_live" discards live cards during performance; no yell, no live.
        if cannot_live {
            let moved = Self::discard_live_on_cannot_live(player);
            return LivePerformanceData {
                yell_count: 0,
                note_icons: 0,
                revealed_ids: Vec::new(),
                member_contributions: Vec::new(),
                yell_cards: Vec::new(),
                total_hearts: [0; 8],
                allocations: Vec::new(),
                heart_sources: Vec::new(),
                blade_sources: Vec::new(),
                draw_effects_occurred: false,
                live_card_ids: vec![],
                moved_live_card_ids: moved,
            };
        }

        // A1+A2: Use stats_pipeline for heart+blade layering (single source of truth)
        let member_contributions = Self::build_member_contributions(
            player,
            card_db,
            blade_modifiers,
            heart_override,
            heart_modifiers,
            heart_color_multiplier,
            heart_copy,
            orientation_modifiers,
        );

        // Q32/Rule 8.3.6: If the live card zone is empty, no yell, no live card processing.
        // But member contributions and stage hearts are still returned.
        if player.live_card_zone.cards.is_empty() {
            return Self::empty_live_performance(member_contributions);
        }

        let total_blade =
            player
                .stage
                .total_blades(card_db, blade_modifiers, orientation_modifiers, false);

        Self::reveal_yell_cards(player, resolution_zone, total_blade, owner);

        // Compute owned hearts from stage
        let mut owned_hearts = player.stage.get_available_hearts(
            card_db,
            heart_override,
            heart_modifiers,
            heart_color_multiplier,
            heart_copy,
        );

        let blade_to_heart = blade_color_to_heart;
        let override_color =
            blade_type_override_for_stage(&player.stage.stage, blade_type_modifiers)
                .map(blade_to_heart);

        // Process yell cards and build YellCardResult + track heart allocations
        let mut heart_sources: Vec<HeartSource> = Vec::new();
        let mut blade_sources: Vec<BladeSource> = Vec::new();
        let mut total_hearts_arr = EMPTY_H8;

        // Stage hearts source
        let stage_heart_str: crate::types::ArcStr = "Stage (base)".into();
        let mut stage_heart_arr = EMPTY_H8;
        for (color, count) in &owned_hearts.hearts {
            let idx = color.index();
            if idx < 8 {
                stage_heart_arr[idx] += count;
            }
        }
        heart_sources.push(HeartSource {
            source_type: SourceType::Stage,
            source: stage_heart_str,
            value: stage_heart_arr,
        });
        for (color, count) in &owned_hearts.hearts {
            let idx = color.index();
            if idx < 8 {
                total_hearts_arr[idx] += count;
            }
        }

        // Add stage blade source
        blade_sources.push(BladeSource {
            source_type: SourceType::Stage,
            source: crate::types::ArcStr::from("Stage members"),
            value: total_blade,
        });

        // Q42: Defer draw effects until after all yell cards have been revealed.
        let (yell_cards, cheer_icon_count, total_draw_icons) = Self::process_yell_zone_cards(
            resolution_zone,
            card_db,
            override_color,
            &mut owned_hearts,
            &mut total_hearts_arr,
        );

        // Q43: Each draw icon revealed during yell draws 1 card, after all
        // yell cards are revealed. Q104 refresh applies mid-draw.
        Self::draw_for_yell_icons(player, total_draw_icons);

        // Yell heart source
        let mut yell_heart_arr = EMPTY_H8;
        for yc in &yell_cards {
            for (i, &bh) in yc.blade_hearts.iter().enumerate() {
                yell_heart_arr[i] += bh;
            }
        }
        if yell_heart_arr.iter().any(|&v| v > 0) {
            heart_sources.push(HeartSource {
                source_type: SourceType::Yell,
                source: crate::types::ArcStr::from("Yell cards"),
                value: yell_heart_arr,
            });
        }
        blade_sources.push(BladeSource {
            source_type: SourceType::Yell,
            source: format!("{} blades", total_blade).into(),
            value: total_blade,
        });

        // NOTE: Special hearts on cards sitting in the live_card_zone do NOT
        // apply here. Both the draw note (rule 8.3.12.1) and the score note
        // (rule 8.4.2.1) are scoped to icons revealed by the yell
        // (「エールで出た」) — i.e. cards in the resolution zone only. A live
        // card in the live zone contributes its icon only when an effect has
        // put it into the deck and the yell reveals it.

        let live_card_ids: Vec<i16> = player.live_card_zone.cards.iter().copied().collect();
        let allocations =
            Self::compute_allocations(&owned_hearts, &live_card_ids, card_db, need_heart_modifiers);

        // Return yell-phase data WITHOUT draining resolution_zone or checking hearts.
        // execute_performance_phase will populate revealed_cards, trigger auto abilities
        // (8.3.13 check timing), then call check_live_success (8.3.14-8.3.16).
        let revealed_ids: Vec<i16> = resolution_zone.cards.iter().copied().collect();
        let draw_effects_occurred = yell_cards.iter().any(|yc| yc.draw_icons > 0);
        LivePerformanceData {
            yell_count: total_blade,
            note_icons: cheer_icon_count,
            revealed_ids,
            member_contributions,
            yell_cards,
            total_hearts: total_hearts_arr,
            allocations,
            heart_sources,
            blade_sources,
            draw_effects_occurred,
            live_card_ids,
            moved_live_card_ids: Vec::new(),
        }
    }
}
