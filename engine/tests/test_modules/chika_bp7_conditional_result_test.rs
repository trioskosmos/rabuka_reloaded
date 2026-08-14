/// Tests for 高海千歌 (PL!S-bp7-001-R) ab#0 — 登場: conditional_on_result character check.
///
/// Card text:
///   登場 手札を1枚控え室に置いてもよい：自分の控え室からコスト10以上のメンバーカードを
///   1枚手札に加える。これにより「桜内梨子」か「渡辺曜」を手札に加えた場合、ライブ終了時まで、
///   ブレード+2 を得る。
///
/// The followup (gain 2 blades) must ONLY fire when the card moved to hand by
/// this effect is 桜内梨子 or 渡辺曜. A bug previously granted blades regardless
/// of which cost≥10 member was added, because the result_condition's `characters`
/// filter was ignored and only the raw hand size was counted.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const CHIKA: &str = "PL!S-bp7-001-R";
const RIKO: &str = "PL!S-bp3-002-R"; // 桜内梨子, cost 11 — matches
const YO: &str = "PL!S-bp7-005-R＋"; // 渡辺曜, cost 15 — matches
const HONOKA: &str = "PL!-sd1-001-SD"; // 高坂穂乃果, cost 11 — does NOT match

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for p in [&mut game.state.player1, &mut game.state.player2] {
        p.main_deck.cards.clear();
        for _ in 0..50 {
            p.main_deck.cards.push(filler);
        }
    }
}

/// Play 千歌, pay the optional discard-1 cost, and recover `target` (the
/// sole cost≥10 member in discard) to hand. With only one eligible member the
/// recover resolves automatically (no selection prompt). Returns after the
/// followup blade check resolves.
fn play_chika_add_to_hand(game: &mut TestGame, chika: i16, target: i16) {
    game.state.player1.hand.cards.push(chika);
    // Give the player more than one hand card so the optional discard-1 cost
    // always prompts (with a single card it may auto-resolve and shift the flow).
    let discard_me = game.id("PL!-sd1-010-SD");
    let discard_me2 = game.id("PL!-sd1-011-SD");
    game.state.player1.hand.cards.push(discard_me);
    game.state.player1.hand.cards.push(discard_me2);
    game.state.player1.waitroom.cards.push(target); // the cost≥10 member to recover
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(chika, MemberArea::Center);

    // Optional discard-1 cost: select one hand card to discard.
    assert!(
        game.has_pending_choice(),
        "Should prompt for the optional discard-1 cost"
    );
    game.select_indices(&[0]);

    // The recover (move 1 cost≥10 member from discard to hand) auto-selects the
    // single eligible member, so no further prompt is expected.
    assert!(
        !game.has_pending_choice(),
        "Single discard candidate should auto-resolve without a prompt"
    );
    game.drain_auto_ability_choices();
}

#[test]
fn chika_adds_riko_gets_2_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id(CHIKA);
    let riko = game.id(RIKO);
    fill_decks(&mut game);
    game.give_energy(9);

    play_chika_add_to_hand(&mut game, chika, riko);

    assert!(
        game.state.player1.hand.cards.contains(&riko),
        "Riko should be added to hand"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(chika),
        2,
        "Adding 桜内梨子 should grant 2 blades"
    );
}

#[test]
fn chika_adds_yo_gets_2_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id(CHIKA);
    let yo = game.id(YO);
    fill_decks(&mut game);
    game.give_energy(9);

    play_chika_add_to_hand(&mut game, chika, yo);

    assert!(
        game.state.player1.hand.cards.contains(&yo),
        "Yo should be added to hand"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(chika),
        2,
        "Adding 渡辺曜 should grant 2 blades"
    );
}

#[test]
fn chika_adds_non_matching_member_gets_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id(CHIKA);
    let honoka = game.id(HONOKA);
    fill_decks(&mut game);
    game.give_energy(9);

    play_chika_add_to_hand(&mut game, chika, honoka);

    assert!(
        game.state.player1.hand.cards.contains(&honoka),
        "Honoka should be added to hand"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(chika),
        0,
        "Adding a non-桜内梨子/渡辺曜 member should NOT grant blades"
    );
}
