/// ミア・テイラー PL!N-bp7-011-R＋ ab#1 (常時) — play-time cost reduction.
///
/// 常時：このカードをプレイする際、自分の控え室にあるすべてのメンバーカードをシャッフルし、
/// デッキの下に置いてもよい。そうしたとき、このカードのコストは２減る。
///
/// When playing ミア (base cost 13) the player is offered the optional play-time
/// choice. Accepting it shuffles all waitroom member cards to the deck bottom and
/// reduces ミア's cost by 2 (13 → 11); declining keeps the full cost.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

const MIA: &str = "PL!N-bp7-011-R\u{ff0b}"; // ミア・テイラー, cost 13
const FILLER: &str = "PL!-sd1-010-SD"; // member card (μ's member)

fn setup(game: &mut TestGame, waitroom_members: usize) -> i16 {
    let mia = game.id(MIA);
    game.add_to_hand(mia);
    for _ in 0..waitroom_members {
        game.state.player1.waitroom.cards.push(game.id(FILLER));
    }
    game.give_energy(13);
    mia
}

/// Answer a pending play-time cost-reduction choice with `accept`.
fn answer_play_choice(game: &mut TestGame, accept: bool) -> bool {
    if !game.has_pending_choice() {
        return false;
    }
    if let Choice::SelectTarget { target, options, .. } = game.get_pending_choice() {
        if target == "play_time_cost_reduction" {
            let yes = options.as_ref().map(|o| o.len() > 1).unwrap_or(false) && accept;
            game.select_choice_option(if yes { 1 } else { 0 });
            return true;
        }
    }
    false
}

/// Accept the play-time reduction → ミア's cost drops 13 → 11, waitroom members
/// are shuffled to the deck bottom.
#[test]
fn mia_ab1_accept_reduces_cost_by_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = setup(&mut game, 2);

    game.play_to_stage(mia, MemberArea::Center);
    assert!(
        answer_play_choice(&mut game, true),
        "playing ミア with waitroom members must offer the play-time cost reduction choice"
    );

    // 13 - 2 = 11 energy paid → 2 remain.
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "accepted: cost 13-2=11 paid, 2 energy remain (got {})",
        game.state.player1.energy_zone.active_count()
    );
    // The waitroom member cards were moved to the deck bottom.
    assert!(
        game.state.player1.waitroom.cards.is_empty(),
        "accepted: waitroom members are shuffled into the deck"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        2,
        "accepted: the 2 waitroom members moved to the deck bottom"
    );
}

/// Decline the play-time reduction → full cost 13 is paid, waitroom unchanged.
#[test]
fn mia_ab1_decline_pays_full_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = setup(&mut game, 2);

    game.play_to_stage(mia, MemberArea::Center);
    assert!(
        answer_play_choice(&mut game, false),
        "playing ミア must offer the play-time cost reduction choice"
    );

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "declined: full cost 13 paid, 0 energy remain (got {})",
        game.state.player1.energy_zone.active_count()
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        2,
        "declined: waitroom members stay put"
    );
}

/// Without any member cards in the waitroom the shuffle/cost-reduction choice is
/// still offered (the ability is optional), but the reduction is available.
#[test]
fn mia_ab1_no_waitroom_members_still_offers_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = setup(&mut game, 0);

    game.play_to_stage(mia, MemberArea::Center);
    // The choice is offered regardless; accept.
    assert!(
        answer_play_choice(&mut game, true),
        "the play-time choice should still be offered with an empty waitroom"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "accepted: cost reduced to 11 even with no waitroom members"
    );
}
