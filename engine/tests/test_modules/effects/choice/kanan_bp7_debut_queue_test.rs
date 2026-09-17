/// Tests for PL!S-bp7-003-P＋ 松浦果南 (Kanan Matsuura, P+):
///
/// The card has TWO [登場] (debut) abilities:
///   ab#0: {{toujyou|登場}}/{{live_start|ライブ開始時}} 自分のデッキの一番上のカードを見る。それをデッキの一番下に置いてもよい。
///   ab#1: {{toujyou|登場}} 以下から1つを選ぶ。(wait immunity / position change)
///
/// Both debut abilities must be visible in the ability queue immediately when
/// the card debuts — not just the first, with the second appearing only after
/// the first resolves.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const KANAN: &str = "PL!S-bp7-003-P＋";
const FILLER: &str = "PL!-sd1-010-SD";

/// Both debut abilities should be enqueued from the start of the debut.
#[test]
fn kanan_bp7_both_debut_abilities_in_queue_from_start() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanan = game.id(KANAN);

    game.state.player1.hand.cards.push(kanan);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }
    game.give_energy(20);

    game.play_to_stage(kanan, MemberArea::Center);

    game.dump_queue();

    // Both debut abilities should be present in the queue at the same time.
    let total = game.state.ability_queue.len();
    let pending = game.state.ability_queue.pending_entries().len();
    assert_eq!(
        total, 2,
        "Expected both debut abilities enqueued from the start, got {}",
        total
    );
    assert_eq!(
        pending, 2,
        "Expected both debut abilities pending from the start, got {}",
        pending
    );
}
