/// L0 gap coverage: additional LiveStart blade gain abilities.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn drain_skips(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { allow_skip: true, .. } => game.select_indices(&[]),
            _ => break,
        }
    }
}

fn advance_live(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
        drain_skips(game);
    }
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// PL!SP-bp2-009-R+: LiveStart → per 2 hand cards, +1 blade (live_end).
/// With 4 cards in hand: +2 blade.
#[test]
fn sp_bp2_009_per_two_hand_cards_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fid = game.id_ref("PL!-sd1-010-SD");
    let member = game.id("PL!SP-bp2-009-R\u{ff0b}");
    game.state.player1.stage.stage = [-1, member, -1];
    fill_decks(&mut game, fid);

    // 4 hand cards → 4/2 = +2 blade
    for _ in 0..4 {
        game.add_to_hand(fid);
    }
    game.give_energy(15);

    advance_live(&mut game);

    let blade = game.state.mods.get_blade_modifier(member);
    assert!(blade >= 2, "4 hand cards → >= +2 blade, got {blade}");
}

/// PL!N-bp7-003-R+: ライブ開始時 ライブ終了時まで、このメンバーの下に置かれている
/// 名前の異なるメンバーカード1枚につき、ブレードを得る。
/// No cost — fires automatically.
#[test]
fn bp7_003_per_under_member_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!N-bp7-003-R\u{ff0b}");
    let under_a = game.new_id("PL!S-bp2-001-R");
    let under_b = game.new_id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [-1, member, -1];
    // Place 2 distinct members under this member
    if let Some(idx) = game.state.player1.stage.stage.iter().position(|&x| x == member) {
        game.state.player1.stage.under_cards[idx].push(under_a);
        game.state.player1.stage.under_cards[idx].push(under_b);
    }
    let fid2 = game.id_ref("PL!-sd1-010-SD");
    fill_decks(&mut game, fid2);
    game.give_energy(15);

    advance_live(&mut game);

    let blade = game.state.mods.get_blade_modifier(member);
    assert!(blade >= 2, "2 distinct under-members → >= +2 blade");
}
