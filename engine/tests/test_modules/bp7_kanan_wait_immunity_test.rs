/// BP07 CLEAN-G4 (engine): PL!S-bp7-003-R＋ 松浦果南 ab#1.
///
/// 登場：以下から1つを選ぶ。
///  ・ライブ終了時まで、自分のステージにいるブレードの数が3つ以下の『Aqours』の
///    メンバーは、相手の効果によってはウェイトしない。
///  ・このメンバーを『Aqours』か『SaintSnow』のメンバーがいるエリアにポジションチェンジする。
///
/// (Debut) choose one: (1) until live end, Aqours members on your stage with blade ≤ 3
/// are NOT put to wait by the OPPONENT's effects; (2) position-change this member.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const KANAN: &str = "PL!S-bp7-003-R\u{ff0b}"; // 松浦果南 — Aqours, blade 2
const KARIN: &str = "PL!N-bp7-004-R"; // 朝香果林 — waits opponent blade≤(energy_under+1)
const ENERGY: &str = "LL-E-001-SD";

fn set_active(game: &mut TestGame, p1_active: bool) {
    game.state.player1.is_first_attacker = p1_active;
    game.state.player2.is_first_attacker = !p1_active;
}

/// Player1 plays 松浦果南 (debut) and picks choice option `opt` (1-based) on their OWN stage.
fn p1_play_kanan(game: &mut TestGame, opt: i16) -> i16 {
    let kanan = game.id(KANAN);
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(kanan);
    game.give_energy(30);
    game.play_to_stage(kanan, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let t = game.pending_choice_type();
        match t.as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            _ => game.select_choice_option((opt - 1) as usize),
        }
    }
    kanan
}

fn is_waited(game: &TestGame, id: i16) -> bool {
    game.state.mods.get_orientation_modifier(id).as_deref() == Some("wait")
}

/// 1. Choosing option 1 records protection for the Aqours member on the owner's stage.
#[test]
fn kanan_option1_records_protection_on_own_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanan = p1_play_kanan(&mut game, 1);

    assert!(
        game.state.wait_immune_members.iter().any(|(m, o)| *m == kanan && o == "p1"),
        "松浦果南 (Aqours, blade 2) should be recorded wait-immune for p1, got {:?}",
        game.state.wait_immune_members
    );
}

/// 2. Choosing option 2 (position change) records NO wait-immunity.
#[test]
fn kanan_option2_no_protection() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    p1_play_kanan(&mut game, 2);

    assert!(
        game.state.wait_immune_members.is_empty(),
        "option 2 (position change) must not grant wait-immunity"
    );
}

/// 3. OPPONENT wait blocked: player2's 朝香果林 activates to wait player1's protected
/// Aqours member (blade 2 ≤ limit) — immunity keeps it active.
#[test]
fn kanan_immunity_blocks_opponent_karin_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Player1 establishes wait-immunity on their Aqours member.
    let kanan = p1_play_kanan(&mut game, 1);
    assert!(game.state.wait_immune_members.iter().any(|(m, _)| *m == kanan));
    assert!(!is_waited(&game, kanan), "sanity: not waited yet");

    // Player2 becomes active and activates 朝香果林, whose wait targets player1's member.
    set_active(&mut game, false);
    let karin = game.id(KARIN);
    game.state.player2.stage.stage[1] = karin;
    let e = game.id(ENERGY);
    game.state.player2.energy_zone.cards.push(e);
    game.state.player2.energy_zone.add_active(1);

    game.activate_ability(karin);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let t = game.pending_choice_type();
        match t.as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            _ => game.select_choice_option(1),
        }
    }

    assert!(
        !is_waited(&game, kanan),
        "opponent 朝香果林 wait must be blocked by wait-immunity; mod={:?}",
        game.state.mods.get_orientation_modifier(kanan)
    );
}

/// 4. SELF wait still works — the owner directly waits their own protected member.
#[test]
fn kanan_immunity_does_not_block_self_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanan = p1_play_kanan(&mut game, 1);
    game.state.mods.add_orientation_modifier(kanan, "wait");

    assert!(is_waited(&game, kanan), "owner's own wait still applies");
}

/// 5. 園田海未's DEBUT cost-limit wait (cost ≤ 4) on the protected member is blocked.
#[test]
fn kanan_immunity_blocks_umis_debut_cost_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Player1 protects their 果南 (Aqours, cost 4, blade 2).
    let kanan = p1_play_kanan(&mut game, 1);
    assert!(!is_waited(&game, kanan), "sanity: not waited yet");

    // Player2 becomes active and plays 園田海未 (debut) — waits opponent cost ≤ 4.
    set_active(&mut game, false);
    let umi = game.id("PL!-bp5-013-N");
    game.state.player2.hand.cards.push(umi);
    for _ in 0..10 {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
    }
    game.state.player2.energy_zone.add_active(10);
    game.play_to_stage(umi, MemberArea::Center);

    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let t = game.pending_choice_type();
        match t.as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            _ => game.select_choice_option(1),
        }
    }

    assert!(
        !is_waited(&game, kanan),
        "園田海未's cost-limit wait must be blocked by wait-immunity"
    );
}
