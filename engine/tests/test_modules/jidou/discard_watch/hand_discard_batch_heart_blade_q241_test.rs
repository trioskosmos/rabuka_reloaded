use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

/// PL!HS-pb1-003-R 大沢瑠璃乃: Debut + Auto each_time (Q241).
///
/// ab#0 (登場): 手札の『みらくらぱーく！』のメンバーカードを好きな枚数控え室に置き、
///               その後、その枚数に1を足した枚数のカードを引く。
/// ab#1 (自動/ターン2回): 自分の手札からカードが1枚以上控え室に置かれるたび、
///               ライブ終了時まで、heart01 と ブレードを得る。
///               Q241: a batch of N discards fires the 自動 ONCE, whatever N is.
///
/// Two things this file used to get wrong, both of which let a regression through:
///
///  1. The prompt drain was keyed to the prompt ORDINAL (`if step <= 2`), so
///     adding or removing one prompt upstream silently changed which prompt got
///     the discard and the tests still passed.
///  2. Only the resulting modifier was checked, never HOW MANY cards actually
///     reached the waitroom. Since 1 card also fires the 自動 once, a test named
///     "3 cards discarded → fires once" would pass having discarded one.
///
/// Each test below now pins the batch by card identity first, then the
/// once-per-batch grant.
const RURINO: &str = "PL!HS-pb1-003-R"; // 大沢瑠璃乃
const RURINO_P: &str = "PL!HS-pb1-003-P＋"; // same card, ＋ printing
const MIRAKU: &str = "PL!HS-sd1-011-SD"; // a 『みらくらぱーく！』 member

fn heart01_mod(game: &TestGame, card_id: i16) -> i32 {
    game.state
        .mods
        .get_heart_modifier(card_id, HeartColor::Heart01)
}

fn blade_mod(game: &TestGame, card_id: i16) -> i32 {
    game.state
        .mods
        .blade_modifiers
        .get(&card_id)
        .map(|e| e.total())
        .unwrap_or(0)
}

/// `miraku_copies` 『みらくらぱーく！』 members in hand alongside `rurino`, filled
/// decks, 20 energy. Returns the candidate ids in hand order.
fn setup(rurino_no: &str, miraku_copies: usize) -> (TestGame, i16, Vec<i16>) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rurino = game.id(rurino_no);
    game.assert_card_identity(rurino, rurino_no);
    game.assert_card_cost(rurino, 15);
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(rurino);
    let mut miraku = Vec::new();
    for _ in 0..miraku_copies {
        let m = game.id(MIRAKU);
        game.assert_card_identity(m, MIRAKU);
        miraku.push(m);
        game.add_to_hand(m);
    }
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(20);
    (game, rurino, miraku)
}

/// Play the debut and answer its prompts by shape, discarding exactly `batch`
/// cards. The engine asks ONE hand SelectCard carrying the count, so this takes
/// that prompt and declines any follow-up "more?" — with no reference to how
/// many prompts preceded it.
fn play_and_discard_batch(game: &mut TestGame, rurino: i16, batch: usize) {
    game.play_to_stage(rurino, MemberArea::Center);
    let mut hand_prompts = 0usize;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[0]),
            Choice::SelectCard { zone, .. } if zone == "hand" => {
                hand_prompts += 1;
                if hand_prompts == 1 {
                    let picks: Vec<usize> = (0..batch).collect();
                    game.select_indices(&picks);
                } else {
                    game.select_indices(&[]);
                }
            }
            _ => game.select_indices(&[]),
        }
    }
    assert!(
        !game.has_pending_choice(),
        "the 登場 and its 自動 must all resolve"
    );
    assert!(
        hand_prompts >= 1,
        "the 登場 must have offered its 手札の…枚数 select"
    );
}

/// Exactly `batch` of `miraku` reached the waitroom, and each is gone from hand.
/// Without this, a test could claim a batch of 3 having discarded 1 — and 1 card
/// fires the 自動 once too, so the grant assertion would not notice.
fn assert_batch_size(game: &TestGame, miraku: &[i16], batch: usize) {
    let waited: Vec<i16> = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .copied()
        .filter(|c| miraku.contains(c))
        .collect();
    assert_eq!(
        waited.len(),
        batch,
        "好きな枚数 = {batch}: exactly {batch} of the candidates reach the waitroom \
         (waited {:?}, candidates {})",
        waited,
        miraku.len()
    );
    for c in &waited {
        assert!(
            !game.state.player1.hand.cards.contains(c),
            "a discarded card must not still be in hand"
        );
    }
}

#[test]
fn hand_discard_batch_q241_two_discarded_auto_fires_once() {
    let (mut game, rurino, miraku) = setup(RURINO, 3);

    play_and_discard_batch(&mut game, rurino, 2);

    assert_batch_size(&game, &miraku, 2);
    // Q241: 2 cards discarded simultaneously → each_time fires ONCE per batch
    assert_eq!(
        heart01_mod(&game, rurino),
        1,
        "Q241: 2 cards discarded in one batch → auto fires once (heart01=1)"
    );
    assert_eq!(
        blade_mod(&game, rurino),
        1,
        "Q241: 2 cards discarded in one batch → auto fires once (blade=1)"
    );
}

/// Same batch of 2 on the ＋ printing, so the rarity cannot be the reason the
/// first case behaves differently.
#[test]
fn hand_discard_batch_q241_p_plus_two_discarded_auto_fires_once() {
    let (mut game, rurino, miraku) = setup(RURINO_P, 2);
    game.assert_card_identity(rurino, RURINO_P);

    play_and_discard_batch(&mut game, rurino, 2);

    assert_batch_size(&game, &miraku, 2);
    assert_eq!(heart01_mod(&game, rurino), 1);
    assert_eq!(blade_mod(&game, rurino), 1);
}

/// Q241 edge: 1 card discarded → fires once (control).
#[test]
fn hand_discard_batch_q241_one_discarded_fires_once() {
    let (mut game, rurino, miraku) = setup(RURINO, 2);

    play_and_discard_batch(&mut game, rurino, 1);

    assert_batch_size(&game, &miraku, 1);
    assert_eq!(
        heart01_mod(&game, rurino),
        1,
        "Q241: 1 card discarded → fires once (heart01=1)"
    );
}

/// Q241 edge: 3 cards discarded → still fires once. This is the case the old
/// version could not distinguish from the 1-card one.
#[test]
fn hand_discard_batch_q241_three_discarded_fires_once() {
    let (mut game, rurino, miraku) = setup(RURINO, 4);

    play_and_discard_batch(&mut game, rurino, 3);

    assert_batch_size(&game, &miraku, 3);
    assert_eq!(
        heart01_mod(&game, rurino),
        1,
        "Q241: 3 cards discarded in one batch → still fires once (heart01=1)"
    );
}

/// Q241 edge: discard 0 → no trigger.
#[test]
fn hand_discard_batch_q241_zero_discarded_no_trigger() {
    let (mut game, rurino, miraku) = setup(RURINO, 1);
    let waitroom_before = game.state.player1.waitroom.cards.len();

    play_and_discard_batch(&mut game, rurino, 0);

    assert_batch_size(&game, &miraku, 0);
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before,
        "0 chosen → the waitroom is untouched"
    );
    assert_eq!(
        heart01_mod(&game, rurino),
        0,
        "Q241: 0 cards discarded → no trigger"
    );
}
