/// BP07 CLEAN-G19: PL!S-bp7-005-R＋ 渡辺 曜 ab#2 (起動).
///
/// 起動：手札を2枚控え室に置く：このメンバーと自分のステージにいるほかの『Aqours』の
/// メンバー1人を選ぶ。それらが持つ登場能力それぞれ1つを発動させる。
///
/// (Activation) Discard 2 from hand: choose THIS member AND 1 other Aqours member
/// on your stage, then activate 1 of the 登場 abilities EACH of them has.
///
/// The defect (G19): the select excluded self and picked only 1 other member, so
/// this member's own 登場 ability would never fire. These tests pin that THIS
/// member is selectable (count 2, not exclude_self) and that its 登場 ability fires.
use crate::helpers::*;

const WATANABE: &str = "PL!S-bp7-005-R＋"; // 渡辺 曜 (Aqours) — has 登場 ab#0 (place discard member under)
const HANAMARU: &str = "PL!S-bp7-007-R＋"; // 国木田花丸 (Aqours) — has 登場 ab#0 (recover cost<=2 member)
const DISCARD_TARGET: &str = "PL!-sd1-001-SD"; // generic member for 渡辺曜's 登場 to place under

fn activate_self_other_debut_retrigger(game: &mut TestGame, watanabe: i16) {
    rabuka_engine::turn::TurnEngine::execute_main_phase_action_with_ability_index(
        &mut game.state,
        &rabuka_engine::game_setup::ActionType::UseAbility,
        Some(watanabe),
        None,
        None,
        None,
        Some(2),
    )
    .expect("activate ab#2 failed");
}

fn setup(game: &mut TestGame) -> (i16, i16) {
    let watanabe = game.id(WATANABE);
    let hanamaru = game.id(HANAMARU);
    // 渡辺 曜 at CENTER (ab#2 requires center); 花丸 on the left.
    game.state.player1.stage.stage = [hanamaru, watanabe, -1];
    // Cost: 2 hand cards to discard.
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    // Discard: a member card that 渡辺曜's 登場 can place under a stage member.
    let dt = game.id(DISCARD_TARGET);
    game.state.player1.waitroom.cards.push(dt);
    (watanabe, hanamaru)
}

/// The printed COST is 手札を2枚控え室に置く — a SelectCard over the HAND, count
/// 2 — and it must actually move 2 cards to the waitroom. This pins the cost
/// itself; the cascade test below pins the member select that follows it.
#[test]
fn discard_two_cost_is_a_hand_select_of_two_and_moves_two_cards() {
    use rabuka_engine::ability::types::Choice;

    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let (watanabe, _) = setup(&mut game);
    let hand_before = game.state.player1.hand.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();
    assert!(
        hand_before >= 2,
        "the cost needs 2 cards in hand, setup gave {hand_before}"
    );

    activate_self_other_debut_retrigger(&mut game, watanabe);

    match game.get_pending_choice() {
        Choice::SelectCard { zone, count, .. } => {
            assert_eq!(
                zone, "hand",
                "手札を2枚控え室に置く — the cost selects out of the HAND, not the \
                 stage"
            );
            assert_eq!(*count, 2, "…2枚 — the cost is exactly 2 cards");
        }
        other => panic!("expected the printed cost prompt, got {other:?}"),
    }

    // Pay it, however the engine splits the two picks.
    for _ in 0..4 {
        match game.get_pending_choice() {
            Choice::SelectCard { zone, .. } if zone == "hand" => game.select_indices(&[0]),
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 2,
        "the two cost cards must land in the waitroom"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 2,
        "the cost must actually leave the hand"
    );
    // …and only then does the member select arrive.
    assert!(
        matches!(
            game.get_pending_choice(),
            Choice::SelectCard { zone, .. } if zone == "stage"
        ),
        "this メンバーと…を選ぶ select must follow the paid cost, got {:?}",
        game.pending_choice_summary()
    );
}

/// The stronger half of the G19 pin: the two picked 登場 really resolve.
/// 渡辺 曜 ab#0 places a member from the waitroom under a stage member;
/// 花丸 ab#0 recovers a cost<=2 member from the waitroom to hand. If the
/// select excluded self, 渡辺 曜's own 登場 could not run and the waitroom
/// would only ever lose a card to 花丸.
#[test]
fn discard_two_debut_retrigger_activates_both_chosen_debut_abilities() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (watanabe, hanamaru) = setup(&mut game);
    // 花丸 ab#0 recovers a member of cost 2 or less from the waitroom, so the
    // recoverable target must actually be a cost<=2 member (PL!-sd1-001-SD is
    // cost 11 and would leave her with no legal candidate at all).
    // 渡辺 曜 ab#0 tucks any waitroom member under a stage member.
    let recoverable = game.id("PL!-sd1-002-SD"); // cost 2
    let tuckable = game.new_id("PL!-sd1-005-SD"); // cost 2
    assert_eq!(
        game.db.get_card(recoverable).unwrap().cost,
        Some(2),
        "花丸 ab#0 recovers コスト2以下のメンバーカード — the target must qualify"
    );
    game.state.player1.waitroom.cards.push(recoverable);
    game.state.player1.waitroom.cards.push(tuckable);
    let waitroom_before = game.state.player1.waitroom.cards.len();
    let hand_before = game.state.player1.hand.cards.len();
    let under_before: usize = game.state.player1.stage.under_cards.iter().map(|c| c.len()).sum();

    activate_self_other_debut_retrigger(&mut game, watanabe);

    // The printed cost comes first: 手札を2枚控え室に置く. The engine may ask
    // for the two cards in one prompt or one at a time, so pay it over
    // hand-zone prompts only, then pin the member select that follows.
    for _ in 0..4 {
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. } if zone == "hand" => {
                game.select_indices(&[0]);
            }
            _ => break,
        }
    }

    match game.get_pending_choice() {
        rabuka_engine::ability::types::Choice::SelectCard {
            count,
            filtered_indices,
            zone,
            ..
        } => {
            assert_eq!(*count, 2, "select should target 2 members (this + 1 other)");
            assert_eq!(
                zone, "stage",
                "このメンバーと自分のステージにいるほかの『Aqours』のメンバー1人 — \
                 both candidates are stage members"
            );
            // None = no filter applied, i.e. every stage member stays selectable.
            // A filter that dropped 渡辺 曜's own slot is the G19 defect.
            assert!(
                filtered_indices.is_none()
                    || filtered_indices
                        .as_deref()
                        .is_some_and(|ix| ix.contains(&0) && ix.contains(&1)),
                "both stage members must be selectable — 渡辺 曜 herself \
                 (このメンバー) is slot 1, which is exactly what G19 excluded; \
                 got {filtered_indices:?}"
            );
        }
        other => panic!("expected SelectCard select, got {:?}", other),
    }
    game.select_indices(&[0, 1]);

    // Each chosen 登場 now asks for its own target; take the first candidate of
    // each prompt. The assertions below do not depend on which 進展 got which
    // card: two distinct 登場 must have resolved either way.
    for _ in 0..6 {
        if !game.has_pending_choice() {
            break;
        }
        game.select_indices(&[0]);
    }

    assert!(!game.has_pending_choice(), "both 登場 must resolve cleanly");
    assert_eq!(
        game.state.player1.stage.stage[0], hanamaru,
        "花丸 stays on stage"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], watanabe,
        "渡辺 曜 stays on stage (起動 only picks, it does not move)"
    );
    // The cost puts 2 cards into the waitroom and each 登場 then takes one out
    // (渡辺 曜 tucks it under a stage member, 花丸 recovers it to hand), so the
    // waitroom nets out unchanged from the pre-activation count.
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before,
        "2 cost discards in, one card per 登場 out (渡辺 曜 tucks one under a \
         stage member, 花丸 recovers one to hand)"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "the cost discarded 2 from hand and 花丸 ab#0 recovered 1 back into it"
    );
    assert!(
        game.state.player1.hand.cards.contains(&recoverable),
        "the cost<=2 member 花丸 recovered must be in hand"
    );
    assert!(
        recoverable != tuckable,
        "the two waitroom candidates must be distinct card instances, otherwise \
         'the cost<=2 member 花丸 recovered' proves nothing"
    );
    let under_after: usize = game.state.player1.stage.under_cards.iter().map(|c| c.len()).sum();
    assert_eq!(
        under_after,
        under_before + 1,
        "渡辺 曜 ab#0 must place a member from the waitroom under a stage member"
    );
    // Which of the two candidates 渡辺 曜 takes is not pinned (each 進展 takes
    // the first legal one), but it must be a card that really was in the
    // waitroom, and the two 登場 must not have shared one card.
    let tucked: Vec<i16> = game
        .state
        .player1
        .stage
        .under_cards
        .iter()
        .flatten()
        .copied()
        .collect();
    assert!(
        tucked.len() == 1,
        "exactly one card was tucked under a stage member, got {:?}",
        tucked
    );
    // Which member 渡辺 曜 takes is deliberately not pinned: her 登場 offers any
    // waitroom member, and by then the 2 cost discards are in the waitroom too.
    // What must hold is that the card really left the waitroom and is now under
    // a stage member rather than duplicated anywhere.
    assert!(
        !game.state.player1.waitroom.cards.contains(&tucked[0])
            && !game.state.player1.hand.cards.contains(&tucked[0]),
        "the tucked card must have MOVED under a stage member, not been copied"
    );
    assert!(
        tucked[0] != recoverable,
        "the card 花丸 recovered to hand cannot also be the tucked card"
    );
}
