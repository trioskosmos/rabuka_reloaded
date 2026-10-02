use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// PL!N-pb1-015-R 桜坂しずく — 登場: optional 2E → deploy a cost≤4 桜坂しずく from hand
///
/// 「{{E}}{{E}}支払ってもよい：手札からコスト4以下の『桜坂しずく』のメンバーカードを1枚
/// ステージに登場させる。」
///
/// The deploy is the whole point, so this states where the target card ENDS UP rather
/// than that it is no longer in hand. "not in hand" is satisfied by a card that was
/// discarded, destroyed, or never in the fixture — all of which would pass while the
/// ability did nothing.
#[test]
fn shizuku_pay_2e_deploys_shizuku() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-pb1-015-R"); // cost 13
    let shizuku_hand = game.new_id("PL!N-bp1-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(shizuku);
    game.state.player1.hand.cards.push(shizuku_hand);
    game.give_energy(15);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(shizuku, MemberArea::Center);
    assert!(game.has_pending_choice(), "pay 2E prompt expected");
    game.select_option(1); // pay
    // After pay, a SelectCard asks WHICH Shizuku to deploy.
    let mut safety=0;
    while game.has_pending_choice() && safety<5 { safety+=1; game.select_indices(&[0]); }
    assert!(
        game.state.player1.stage.stage.contains(&shizuku_hand),
        "「ステージに登場させる」 — after paying, the 桜坂しずく member must be ON STAGE. \
         The old assertion was `on_stage || !in_hand`, which a discarded or destroyed card \
         satisfies just as well as a deployed one."
    );
    game.assert_energy(0, "13E to play 桜坂しずく plus the 2E optional deploy cost");
    assert!(!game.has_pending_choice());
}
#[test]
fn shizuku_skip_pay_no_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-pb1-015-R");
    let shizuku_hand = game.new_id("PL!N-bp1-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(shizuku);
    game.state.player1.hand.cards.push(shizuku_hand);
    game.give_energy(15);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(shizuku, MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_option(0); // skip
    // Bounded: an unbounded drain here would hang the suite rather than fail.
    let mut safety = 0;
    while game.has_pending_choice() && safety < 5 {
        safety += 1;
        game.select_indices(&[]);
    }
    assert!(game.state.player1.hand.cards.contains(&shizuku_hand), "skip should keep Shizuku in hand");
    game.assert_energy(2, "only the 13E to play 桜坂しずく — a declined optional cost is free");
}
/// The hand holds no eligible 桜坂しずく, so nothing can be deployed.
///
/// This test previously asserted almost nothing. Its cleanup loop was
///
/// ```ignore
/// while game.has_pending_choice() && safety<5 { let _ = game.try_select_indices(&[]); let _ = game.try_select_indices(&[0]); }
/// ```
///
/// which discarded BOTH `Result`s and attempted BOTH answers unconditionally on every
/// iteration, so it could neither report a failure nor guarantee progress — the second
/// call ran even when the first had already answered the prompt. The one assertion it
/// reached, `hand contains filler`, was already true because the filler was never a
/// candidate for the deploy.
///
/// The real content of this case is that a deploy with no candidate leaves the board
/// alone: no member staged, and the unrelated hand card untouched.
#[test]
fn shizuku_no_eligible_shizuku_no_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-pb1-015-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(shizuku);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(shizuku, MemberArea::Center);
    // Pay the optional 2E, then answer whatever the deploy asks. `filler` is not a
    // 桜坂しずく, so there is no candidate: any SelectCard it offers must be declined.
    let mut safety = 0;
    while game.has_pending_choice() && safety < 5 {
        safety += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[]),
            _ => game.select_option(0),
        }
    }
    assert!(
        game.state.player1.hand.cards.contains(&filler),
        "the unrelated hand card is not something this ability may move"
    );
    assert_eq!(
        game.state.player1.stage.stage.iter().filter(|&&id| id != -1).count(),
        1,
        "only the 桜坂しずく played this turn is on stage: with no eligible member in hand \
         the deploy adds nothing. Counted rather than compared to a literal so the card \
         just played is not mistaken for a second member."
    );
}
#[test]
fn shizuku_insufficient_energy_no_pay() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-pb1-015-R");
    let shizuku_hand = game.new_id("PL!N-bp1-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(shizuku);
    game.state.player1.hand.cards.push(shizuku_hand);
    game.give_energy(1);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    let res = game.try_play_to_stage(shizuku, MemberArea::Center);
    assert!(res.is_err(), "should fail to play with only 1 energy, need 13");
    // The refusal must be "cannot afford", not some unrelated failure: the 1
    // energy is still unspent and neither card left the hand.
    game.assert_energy_untouched_after_refusal(1, "insufficient energy to play 静月");
    assert!(
        game.state.player1.hand.cards.contains(&shizuku),
        "a refused play leaves the card in hand"
    );
    assert!(
        game.state.player1.hand.cards.contains(&shizuku_hand),
        "a refused play must not move the target out of hand either"
    );
    assert_eq!(
        game.state.player1.stage.stage,
        [-1, -1, -1],
        "a refused play must leave the stage empty"
    );
}
