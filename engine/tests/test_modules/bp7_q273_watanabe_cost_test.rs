/// Q273 — 渡辺 曜 PL!S-bp7-005-R＋ ab#2 (起動 センター).
///
/// 起動：手札を2枚控え室に置く：このメンバーと自分のステージにいるほかの『Aqours』の
/// メンバー1人を選ぶ。それらが持つ登場能力それぞれ1つを発動させる。
///
/// Official QA Q273: によって発動させた[登場能力]にコストが設定されている場合、そのコストは
/// 支払いますか？ → はい。支払います。
///
/// Engine support (Q273): `execute_activate_ability` now fires EVERY selected
/// member's 登場 ability (「それらが持つ…それぞれ」, not just the last), and fires it
/// through the normal ability queue so the 登場 ability's own cost is paid before
/// its effect resolves. The fired trigger is inferred as 登場 when the parser leaves
/// target_trigger null.
use crate::helpers::*;

const WATANABE: &str = "PL!S-bp7-005-R\u{ff0b}"; // 渡辺 曜 — ab#2 起動 fires 登場 abilities
// 国木田花丸 (Aqours) whose 登場 has a COST (手札を1枚控え室に置いてもよい) then looks at top 3.
const HANAMARU: &str = "PL!S-PR-019-PR";
const FILLER: &str = "PL!-sd1-010-SD";
const DISCARD_MEMBER: &str = "PL!-sd1-001-SD";

/// Activate 渡辺曜's ab#2, discard 2, select 渡辺+花丸, drive the fired 登場
/// abilities (paying 花丸's cost). Returns whether 花丸's 登場 cost was offered/paid.
fn run_ab2(game: &mut TestGame, watanabe: i16) -> bool {
    rabuka_engine::turn::TurnEngine::execute_main_phase_action_with_ability_index(
        &mut game.state,
        &rabuka_engine::game_setup::ActionType::UseAbility,
        Some(watanabe),
        None,
        None,
        None,
        Some(2),
    )
    .expect("activate ab#2");

    let pid = game.state.player1.id.clone();
    let mut saw_hana_cost = false;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 60 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            // 渡辺 ab#2 cost: discard 2 from hand.
            rabuka_engine::ability::types::Choice::SelectCard { zone, count, .. } if zone == "hand" && count == 2 => {
                game.select_indices(&[0, 1]);
            }
            // Select 渡辺 + 花丸 (this member + 1 other Aqours).
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. } if zone == "stage" => {
                game.select_indices(&[0, 1]);
            }
            // The fired 登場 abilities are queued as auto abilities — accept.
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_indices(&[]);
            }
            // 渡辺's 登場: place 1 discard member under a stage member.
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. } if zone == "discard" => {
                game.select_indices(&[0]);
            }
            // 花丸's 登場 COST: discard 1 from hand (Q273 — must be paid).
            rabuka_engine::ability::types::Choice::SelectCard { zone, allow_skip, .. } if zone == "hand" => {
                saw_hana_cost = true;
                if allow_skip {
                    game.select_indices(&[0]); // pay the cost (discard 1)
                } else {
                    game.select_indices(&[0]);
                }
            }
            // 花丸's 登場 effect: pick 1 from the looked-at top-3 to add to hand.
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. } if zone == "looked_at" => {
                game.select_indices(&[0]);
            }
            // Any other placement select (stage member under which to place) → first.
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[0]);
            }
            _ => game.select_choice_option(1),
        }
    }
    let _ = &pid;
    saw_hana_cost
}

/// The fired 登場 ability's cost is offered AND paid (Q273). 花丸's 登場 cost
/// (discard 1 from hand) must be presented and, when paid, her effect resolves.
#[test]
fn q273_fired_debut_ability_cost_is_paid() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let watanabe = game.id(WATANABE);
    let hanamaru = game.id(HANAMARU);
    game.state.player1.stage.stage = [hanamaru, watanabe, -1];
    // 3 hand cards: 2 for 渡辺's cost, 1 for 花丸's 登場 cost.
    game.state.player1.hand.cards.push(game.id(FILLER));
    game.state.player1.hand.cards.push(game.id(FILLER));
    game.state.player1.hand.cards.push(game.id(FILLER));
    // Discard member for 渡辺's 登場 to place under.
    game.state.player1.waitroom.cards.push(game.id(DISCARD_MEMBER));
    // Top-3 deck cards for 花丸's 登場 look.
    let deck_card = game.id(DISCARD_MEMBER);
    game.state.player1.main_deck.cards.push(deck_card);
    game.state.player1.main_deck.cards.push(game.id(DISCARD_MEMBER));
    game.state.player1.main_deck.cards.push(game.id(DISCARD_MEMBER));
    let deck_before = game.state.player1.main_deck.cards.len();

    let saw_hana_cost = run_ab2(&mut game, watanabe);

    assert!(
        saw_hana_cost,
        "Q273: the fired 登場 ability's cost (discard 1) must be offered and paid"
    );
    // 花丸's 登場 effect resolved: a top-3 card was added to hand (deck shrank).
    assert!(
        game.state.player1.main_deck.cards.len() < deck_before,
        "Q273: after paying the cost, the 登場 effect resolved (a card was added to hand)"
    );
}

/// 渡辺's own 登場 ability (place discard member under) also fires via ab#2.
#[test]
fn q273_watanabe_own_debut_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let watanabe = game.id(WATANABE);
    let hanamaru = game.id(HANAMARU);
    game.state.player1.stage.stage = [hanamaru, watanabe, -1];
    game.state.player1.hand.cards.push(game.id(FILLER));
    game.state.player1.hand.cards.push(game.id(FILLER));
    game.state.player1.hand.cards.push(game.id(FILLER));
    game.state.player1.waitroom.cards.push(game.id(DISCARD_MEMBER));
    let deck_card = game.id(DISCARD_MEMBER);
    game.state.player1.main_deck.cards.push(deck_card);
    game.state.player1.main_deck.cards.push(game.id(DISCARD_MEMBER));
    game.state.player1.main_deck.cards.push(game.id(DISCARD_MEMBER));

    run_ab2(&mut game, watanabe);

    // 渡辺's 登場 placed a discard member under 渡辺曜 (center).
    assert_eq!(
        game.state.player1
            .stage
            .get_under_cards(rabuka_engine::zones::MemberArea::Center)
            .len(),
        1,
        "Q273: 渡辺's own 登場 ability fired and placed a member under her"
    );
}
