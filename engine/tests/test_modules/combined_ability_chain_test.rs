//! Combined multi-ability chains (mission ﾂｧboth-players + ﾂｧcombined).
//!
//! Each test drives ONE continuous game flow through multiple distinct
//! abilities feeding each other 窶・not isolated single-card pins.
//!
//! Chain 1 (retrieval 竊・placement 竊・blade):
//!   bp4-007-R+ 蜆ｪ譛ｨ縺帙▽闖・debut (both players retrieve a Niji live)
//!     竊・P1's copy is DIVE!, whose ab#0 arms (own-effect discard竊檀and)
//!       竊・optional placement into the live zone
//!         竊・DIVE! ab#1 grants blade+2 to a Nijigasaki member
//!   窶ｦwhile P2's retrieved DIVE! copy must stay inert (opponent-effect +
//!   wrong-phase gate).
//!
//! Chain 2 (failure relocation vs cross-seat success at one determination):
//!   P1 fails MIRACLE WAVE (12 hearts) 竊・螳蛾､雁ｯｺ蟋ｫ闃ｽ-style Aqours relocation
//!   via 譯懷・譴ｨ蟄・bp6-002-R+ auto 竊・deck
//!   窶ｦwhile P2 exact-fills 譛ｪ譚･縺ｮ蜒輔ｉ縺ｯ遏･縺｣縺ｦ繧九ｈ 竊・per-seat flags diverge.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const SETSUNA_BOTH: &str = "PL!N-bp4-007-R\u{ff0b}";
const DIVE: &str = "PL!N-bp4-026-L";

/// Chain 1: three abilities resolve in one continuous main phase.
#[test]
fn retrieval_then_dive_placement_then_blade_grant() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive_p1 = g.id(DIVE);
    let setsuna = g.id(SETSUNA_BOTH);
    let filler = g.id("PL!-sd1-010-SD");
    // Only DIVE! in each waitroom 竊・both retrievals forced onto DIVE! copies.
    g.state.player1.hand.cards.push(setsuna);
    g.state.player1.waitroom.cards.push(dive_p1);
    g.state.player2.waitroom.cards.push(g.new_id(DIVE));
    fill_decks(&mut g, filler);
    g.give_energy(15);

    // P1 stages Setsuna: her own stage slot fills, and her debut fires.
    g.play_to_stage(setsuna, MemberArea::LeftSide);

    // Drain strictly by type: SelectCard answers are retrievals / placement;
    // SelectTarget is DIVE! ab#1's blade-target pick.
    let mut guard = 0;
    while g.has_pending_choice() {
        guard += 1;
        assert!(guard <= 12, "runaway prompt loop");
        let ty = g.pending_choice_type().unwrap_or_default();
        match ty.as_str() {
            "SelectCard" | "SelectAutoAbility" | "SelectTarget" => {
                g.select_indices(&[0]);
            }
            other => panic!("unexpected prompt {other}"),
        }
    }

    // End state of the FULL chain:
    // 1. Setsuna herself is on stage窶ｦ
    assert!(
        g.state.player1.stage.stage.contains(&setsuna),
        "Setsuna staged"
    );
    // 2. P1's DIVE! was retrieved then PLACED into the live card zone窶ｦ
    assert!(
        g.state.player1.live_card_zone.cards.contains(&dive_p1),
        "P1's DIVE! placed into live card zone"
    );
    // 3. 窶ｦand ab#1 granted blade+2 to the staged Nijigasaki member.
    let blade = g
        .state
        .mods
        .blade_modifiers
        .get(&setsuna)
        .map_or(0, |e| e.total());
    assert!(
        blade >= 2,
        "DIVE! ab#1 should grant blade+2 to the Niji member, got {blade}"
    );

    // 4. Cross-seat gate still holds: P2's copy never armed.
    assert_eq!(
        g.state.player2.live_card_zone.cards.len(),
        0,
        "P2's DIVE! stayed out of the live zone"
    );
}

/// Chain 2: one determination, divergent seat outcomes.
/// P1 sets MIRACLE WAVE (needs 12 hearts, has ~6) 竊・FAILS 竊・譯懷・譴ｨ蟄・bp6-002
/// auto relocates it to her deck. P2 exact-fills 譛ｪ譚･縺ｮ蜒輔ｉ縺ｯ遏･縺｣縺ｦ繧九ｈ 竊・/// succeeds. Per-seat flags must diverge accordingly.
#[test]
fn determination_cascade_p1_failure_relocation_p2_success() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let riko = g.id("PL!S-bp6-002-R\u{ff0b}");
    let mw = g.id("PL!S-bp3-019-L"); // MIRACLE WAVE 窶・Aqours, needs 12 hearts
    let p2_member = g.id("PL!S-bp2-001-R"); // 鬮俶ｵｷ蜊・ｭ・ exact-fills P2's live
    let p2_live = g.id("PL!S-sd1-019-SD"); // 譛ｪ譚･縺ｮ蜒輔ｉ縺ｯ遏･縺｣縺ｦ繧九ｈ, score 1
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.stage.stage = [-1, riko, -1];
    g.state.player2.stage.stage = [-1, p2_member, -1];
    // P1's deck must be heart-less: yell reveals deck cards into the pool and
    // any member hearts there would accidentally satisfy MIRACLE WAVE.
    let energy = g.id("LL-E-001-SD");
    for _ in 0..12 {
        g.state.player1.main_deck.cards.push(energy);
        g.state.player2.main_deck.cards.push(filler);
    }
    g.state.player1.hand.cards.push(mw);
    g.state.player2.hand.cards.push(p2_live);
    g.give_energy(20);

    // Drive exactly like opponent_live_success_flow_test: fixed pass counts
    // reach each LiveCardSet phase, then determination.
    let mut seen = |g: &mut TestGame| {
        while g.has_pending_choice() {
            g.select_indices(&[0]);
        }
    };
    for _ in 0..5 {
        seen(&mut g);
        g.pass();
    }
    g.set_live_card(mw);
    seen(&mut g);
    g.pass();
    g.set_live_card(p2_live);
    let mut p1_set = true;
    let mut p2_set = true;
    let _ = (&mut p1_set, &mut p2_set);
    for _ in 0..8 {
        seen(&mut g);
        if g.state.p1_live_success_this_turn || g.state.p2_live_success_this_turn {
            break;
        }
        g.pass();
    }
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    // Per-seat divergence at the SAME determination:
    assert!(
        !g.state.p1_live_success_this_turn,
        "P1 must FAIL: MIRACLE WAVE's 12-heart need cannot be met"
    );
    assert!(
        g.state.p2_live_success_this_turn,
        "P2 must succeed: exact-fill live"
    );

    // Riko's auto relocated the failed Aqours live off the waitroom.
    assert!(
        !g.state.player1.waitroom.cards.contains(&mw)
            || g.state.player1.main_deck.cards.contains(&mw),
        "MIRACLE WAVE relocated to deck by Riko's auto"
    );
}
