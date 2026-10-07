// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
use rabuka_engine::game_setup::*;
use rabuka_engine::game::match_runner::*;
use rabuka_engine::game::sav::*;
use rabuka_engine::game_state::*;


use rabuka_engine::game::link::*;

    use rabuka_engine::game::platform_ui::PlatformUi;
    use std::cell::Cell;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    struct Hub {
        to_a: VecDeque<Vec<u8>>,
        to_b: VecDeque<Vec<u8>>,
        budget: usize,
        alive: bool,
        sent_a: usize,
        sent_b: usize,
    }

    /// Cross-connected loopback pair: A's sends land in B's inbox and back.
    /// `budget` bounds total packets so a desync bug aborts instead of
    /// hanging the suite (libtest has no per-test timeout).
    #[derive(Clone)]
    struct TestTransport {
        hub: Arc<Mutex<Hub>>,
        is_a: bool,
    }

    impl TestTransport {
        fn pair(budget: usize) -> (Self, Self) {
            let hub = Arc::new(Mutex::new(Hub {
                to_a: VecDeque::new(),
                to_b: VecDeque::new(),
                budget,
                alive: true,
                sent_a: 0,
                sent_b: 0,
            }));
            (
                TestTransport { hub: hub.clone(), is_a: true },
                TestTransport { hub, is_a: false },
            )
        }
    }

    impl LinkTransport for TestTransport {
        fn is_up(&mut self) -> bool {
            let hub = self.hub.lock().unwrap();
            hub.alive && hub.budget > 0
        }
        fn send_packet(&mut self, packet: &[u8]) -> bool {
            let mut hub = self.hub.lock().unwrap();
            if !hub.alive || hub.budget == 0 {
                hub.alive = false;
                return false;
            }
            hub.budget -= 1;
            if self.is_a {
                hub.to_b.push_back(packet.to_vec());
                hub.sent_a += 1;
            } else {
                hub.to_a.push_back(packet.to_vec());
                hub.sent_b += 1;
            }
            true
        }
        fn recv_packet(&mut self) -> Option<Vec<u8>> {
            let mut hub = self.hub.lock().unwrap();
            if self.is_a {
                hub.to_a.pop_front()
            } else {
                hub.to_b.pop_front()
            }
        }
    }

    /// Auto-picking console: first row always, except RPS where side 1
    /// takes the second row (Paper beats Rock: no draw loop). First-row
    /// picks are the progress options everywhere (Confirm / Pass), so a
    /// tiny-deck game mills to terminal in dozens of decisions, not
    /// thousands. The decision count (not frames or headers) would drive
    /// fancier policies; first-row needs no counting at all.
    struct TestUi {
        local: u8,
        lang: rabuka_engine::game::language::Lang,
        frame: Cell<usize>,
        rows: Vec<String>,
    }

    impl TestUi {
        fn new(local: u8) -> Self {
            TestUi {
                local,
                lang: rabuka_engine::game::language::Lang::Japanese,
                frame: Cell::new(0),
                rows: Vec::new(),
            }
        }
        fn downs(&self) -> usize {
            if self.rows.iter().any(|r| r.contains("Rock") || r.contains("グー")) {
                return self.local as usize;
            }
            0
        }
    }

    impl PlatformUi for TestUi {
        fn clear_screen(&mut self) {
            // NOTE: no frame reset here — select_action clears every
            // frame, so the frame counts polls since the last A press.
            self.rows.clear();
        }
        fn println(&mut self, text: &str) {
            if self.rows.len() < 12 {
                self.rows.push(text.to_string());
            }
        }
        fn swap_buffers(&mut self) {}
        fn poll_input(&mut self) {
            self.frame.set(self.frame.get() + 1);
        }
        fn just_pressed_a(&self) -> bool {
            // One A press consumes one menu; the frame restarts so the
            // next menu's policy counts from zero.
            if self.frame.get() == self.downs() + 1 {
                self.frame.set(0);
                true
            } else {
                false
            }
        }
        fn just_pressed_b(&self) -> bool {
            false
        }
        fn just_pressed_up(&self) -> bool {
            false
        }
        fn just_pressed_down(&self) -> bool {
            let f = self.frame.get();
            f >= 1 && f <= self.downs()
        }
        fn just_pressed_start(&self) -> bool {
            false
        }
        fn wait_vblank(&mut self) {
            // Frame pacing for the loopback harness only: send_reliable's
            // retransmit cadence (every 60 spins) assumes 60fps vblank waits.
            // A no-op here makes spins burn CPU and hub budget at millions
            // per second — a peer stuck in one reschedule can then strand
            // its counterpart mid-exchange (±1 action break asymmetry).
            std::thread::sleep(std::time::Duration::from_micros(120));
        }
        // Each console keeps its own language: multiplayer must work across a
        // mixed-language link (P1 English vs P2 Japanese). The wire carries
        // numeric action tags, never text, so the languages need not agree —
        // this test exercises exactly that by giving side 1 English and the
        // other side Japanese. downs() matches both spellings because the
        // scripted Paper tiebreak must work in EITHER rendering.
        fn ui_lang(&self) -> rabuka_engine::game::language::Lang {
            self.lang
        }
    }

    #[test]
    fn link_action_codec_roundtrip() {
        let la = LinkAction {
            tag: 6,
            card_id: Some(42),
            card_indices: vec![1, 2, 3],
            stage_area: 2,
            use_baton_touch: true,
            ability_index: Some(7),
            seq: 0xDEAD_BEEF,
        };
        let bytes = la.encode();
        assert_eq!(bytes[0], MSG_ACTION);
        assert_eq!(LinkAction::decode(&bytes), Some(la));
        let bare = LinkAction {
            tag: 22,
            card_id: None,
            card_indices: vec![],
            stage_area: 0,
            use_baton_touch: false,
            ability_index: None,
            seq: 0,
        };
        assert_eq!(LinkAction::decode(&bare.encode()), Some(bare));
        assert_eq!(LinkAction::decode(&[]), None);
        assert_eq!(LinkAction::decode(&[0x09]), None);
    }

    #[test]
    fn setup_codec_roundtrip() {
        let deck = SavDeck {
            name: String::from(" 体力測定* "),
            cards: vec![String::from("PL!S-bp2-022-L"), String::from("LL-bp2-001-R＋")],
        };
        let bytes = encode_setup(0x1234_5678_9ABC_DEF0, &deck);
        assert_eq!(bytes[0], MSG_SETUP);
        let (seed, back) = decode_setup(&bytes).expect("setup decodes");
        assert_eq!(seed, 0x1234_5678_9ABC_DEF0);
        assert_eq!(back, deck);
        assert_eq!(decode_setup(&bytes[..4]), None);
    }

    #[test]
    fn find_local_action_matches_and_rejects() {
        use rabuka_engine::game::game_setup::{ActionParameters, ActionType};
        let mk = |t: ActionType, cid: Option<i16>| Action {
            description: String::from("row"),
            description_ja: None,
            action_type: t,
            parameters: Some(ActionParameters {
                card_id: cid,
                card_index: None,
                card_indices: None,
                stage_area: None,
                stage_area_index: None,
                use_baton_touch: None,
                card_name: None,
                card_no: None,
                ability_index: None,
                source_ability: None,
                base_cost: None,
                final_cost: None,
                available_areas: None,
                double_baton_pairs: None,
                disabled: None,
            }),
            selected: None,
        };
        let acts = vec![
            mk(ActionType::RockChoice, None),
            mk(ActionType::PaperChoice, None),
            mk(ActionType::ChoiceDecision, Some(1)),
        ];
        let la = LinkAction::from_action(&acts[2], 9);
        assert_eq!(find_local_action(&acts, &la), Some(2));
        let mut wrong = la.clone();
        wrong.card_id = Some(0);
        assert_eq!(find_local_action(&acts, &wrong), None);
    }

    #[test]
    fn loopback_transport_moves_packets_both_ways() {
        let (mut ta, mut tb) = TestTransport::pair(10);
        assert!(ta.send_packet(&[1, 2, 3]));
        assert!(tb.send_packet(&[4, 5]));
        assert_eq!(tb.recv_packet(), Some(vec![1, 2, 3]));
        assert_eq!(ta.recv_packet(), Some(vec![4, 5]));
        assert_eq!(ta.recv_packet(), None);
        assert!(ta.is_up() && tb.is_up());
    }

    /// Full lockstep spot-check over loopback: both sides run concurrently
    /// (threads — sequential runs could never exchange a packet) through a
    /// small packet budget (~25 decisions: RPS, first-attacker pick,
    /// mulligans, live sets, passes), then their full state digests must
    /// match exactly. Any lockstep divergence (routing, codec, ordering)
    /// fails this test. Tiny decks (6 members + 3 lives a side;
    /// composition warnings are warnings only) keep every decision cheap,
    /// so this runs in seconds even in debug profile.
    #[test]
    fn loopback_full_match_stays_in_lockstep() {
        for lang_a in rabuka_engine::game::language::Lang::SUPPORTED {
            for lang_b in rabuka_engine::game::language::Lang::SUPPORTED {
                assert_loopback_lockstep(lang_a, lang_b);
            }
        }
    }

    /// The 18 cards this test plays with, parsed and ability-attached ONCE.
    ///
    /// `load_cards_from_strs` parses the whole ~2280-card `cards.json`, and
    /// this used to run inside `assert_loopback_lockstep` — i.e. once per
    /// language pair, so 2x2 = 4 full database parses for a single test. The
    /// matches themselves are cheap next to that; hoisting the parse is what
    /// makes this test tolerable.
    fn test_deck_cards() -> &'static [rabuka_engine::card::Card] {        static DECK_CARDS: std::sync::OnceLock<Vec<rabuka_engine::card::Card>> =
            std::sync::OnceLock::new();
        DECK_CARDS.get_or_init(|| {
            let json = include_str!("../../../cards/cards.json");
            let all_json = rabuka_engine::card_loader::CardLoader::load_cards_from_strs(json)
                .expect("cards load");
            let mut members: Vec<&rabuka_engine::card::Card> =
                all_json.iter().filter(|c| c.is_member()).collect();
            members.sort_by(|a, b| a.card_no.as_ref().cmp(b.card_no.as_ref()));
            let mut lives: Vec<&rabuka_engine::card::Card> =
                all_json.iter().filter(|c| c.is_live()).collect();
            lives.sort_by(|a, b| a.card_no.as_ref().cmp(b.card_no.as_ref()));
            assert!(
                members.len() >= 12 && lives.len() >= 6,
                "test needs 12 members and 6 lives, found {} and {}",
                members.len(),
                lives.len()
            );
            let mut picked: Vec<rabuka_engine::card::Card> = members[..12]
                .iter()
                .chain(lives[..6].iter())
                .map(|c| (*c).clone())
                .collect();
            rabuka_engine::card_loader::CardLoader::attach_abilities(&mut picked);
            picked
        })
    }

    fn assert_loopback_lockstep(
        lang_a: rabuka_engine::game::language::Lang,
        lang_b: rabuka_engine::game::language::Lang,
    ) {
        let deck_cards = test_deck_cards();
        let all_cards: Vec<rabuka_engine::card::Card> = deck_cards.to_vec();
        // `picked` is members[..12] then lives[..6], so the two decks are
        // contiguous slices of it: P1 = members 0..6 + lives 0..3,
        // P2 = members 6..12 + lives 3..6.
        let card_no = |i: usize| deck_cards[i].card_no.as_ref().to_string();
        let p1deck: Vec<String> = (0..6).map(card_no).chain((12..15).map(card_no)).collect();
        let p2deck: Vec<String> = (6..12).map(card_no).chain((15..18).map(card_no)).collect();
        let p1: Vec<&str> = p1deck.iter().map(|s| s.as_str()).collect();
        let p2: Vec<&str> = p2deck.iter().map(|s| s.as_str()).collect();
        let p1b: Vec<&str> = p1deck.iter().map(|s| s.as_str()).collect();
        let p2b: Vec<&str> = p2deck.iter().map(|s| s.as_str()).collect();

        let (mut ta, mut tb) = TestTransport::pair(20_000);
        let hub = ta.hub.clone();
        let all_cards_b = all_cards.clone();

        // Both sides run concurrently: sequential runs could never
        // exchange a packet (side A would time out waiting for an ack
        // side B hasn't started sending yet).
        //
        // States are built SERIALLY before the concurrent section:
        // build_match_state consumes the shared global RNG (deck shuffles),
        // and two threads drawing from one Mutex-guarded generator
        // nondeterministically desync the two decks. Seeding + building
        // each side serially gives both the same deterministic stream.
        // Languages intentionally differ (P1 Japanese vs P2 English):
        // multiplayer must work across a mixed-language link — the wire
        // carries numeric action tags, never text, so the languages need
        // not agree. downs() matches both spellings of the RPS row so the
        // scripted Paper tiebreak works in either rendering.
        rabuka_engine::rng::seed(0x5EED);
        let gs_a = build_match_state(&p1, &p2, all_cards);
        rabuka_engine::rng::seed(0x5EED);
        let gs_b = build_match_state(&p1b, &p2b, all_cards_b);

        let (ra, rb) = std::thread::scope(|s| {
            let hb = s.spawn(move || {
                let mut ub = TestUi::new(1);
                ub.lang = lang_b;
                run_link_match_prebuilt(&mut ub, &mut tb, 1, gs_b)
            });
            let mut ua = TestUi::new(0);
            ua.lang = lang_a;
            let ra = run_link_match_prebuilt(&mut ua, &mut ta, 0, gs_a);
            (ra, hb.join().expect("side B finishes"))
        });

        assert_eq!(ra.result, rb.result, "both sides agree on the result");
        assert_ne!(ra.result, GameResult::Ongoing, "match must reach a terminal result");
        assert!(ra.final_state.game_ended && rb.final_state.game_ended);

        fn digest(gs: &GameState) -> String {
            format!(
                "{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}|{:?}|{}",
                gs.current_phase,
                gs.turn_number,
                gs.player1.hand.cards.len(),
                gs.player1.main_deck.cards.len(),
                gs.player1.energy_zone.cards.len(),
                gs.player1.live_card_zone.cards.len(),
                gs.player1.success_live_card_zone.cards.len(),
                gs.player1.waitroom.cards.len(),
                gs.player2.hand.cards.len(),
                gs.player2.main_deck.cards.len(),
                gs.player2.energy_zone.cards.len(),
                gs.player1_rps_choice,
                gs.player2_rps_choice,
                gs.game_result,
                gs.rule_log.len()
            )
        }
        assert_eq!(
            digest(&ra.final_state),
            digest(&rb.final_state),
            "both sides hold identical states"
        );
        let hub = hub.lock().unwrap();
        assert!(hub.sent_a > 20, "a real exchange happened, not a stub");
    }
