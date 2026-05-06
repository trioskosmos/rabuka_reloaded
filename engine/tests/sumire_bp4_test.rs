/// Tests for PL!SP-bp4-004-R+ (平安名すみれ) ab#1 — Q193, Q194
///
/// ab#0 (常時): このカードのプレイに際し、2人のメンバーとバトンタッチしてよい
/// ab#1 (登場)[Center]: 2人バトンタッチで登場した場合、
///   2枚引き、控え室からコスト4以下のLiella!を空きエリアに登場させる
///
/// Q193: 2人バトンタッチの出現エリアは？
/// Answer: バトンタッチした2人のエリアのいずれか。プレイヤーが選ぶ。
///
/// Q194: 今ターン登場したメンバーをバトンタッチ元にできる？
/// Answer: いいえ。2人とも前のターンに登場している必要あり。

mod helpers;
use helpers::*;

/// Verify the parser extracted the constant ability (2-member baton touch).
#[test]
fn sumire_bp4_q193_parser_constant_ability() {
    let db = load_real_database();
    let card = db.get_card_by_no("PL!SP-bp4-004-R\u{ff0b}")
        .or_else(|| db.get_card_by_no("PL!SP-bp4-004-R+"))
        .expect("Sumire bp4 should exist");

    // ab#0: constant ability for 2-member baton touch
    let ab0 = card.abilities.iter()
        .find(|a| a.triggers.as_deref() == Some("常時"))
        .expect("Should have 常時 ability");
    assert!(ab0.full_text.contains("バトンタッチ"),
        "Constant ability should mention baton touch");
}

/// Verify the parser extracted the debut ability with draw + place from discard.
#[test]
fn sumire_bp4_q194_parser_debut_ability() {
    let db = load_real_database();
    let card = db.get_card_by_no("PL!SP-bp4-004-R\u{ff0b}")
        .or_else(|| db.get_card_by_no("PL!SP-bp4-004-R+"))
        .expect("Sumire bp4 should exist");

    // ab#1: debut ability
    let ab1 = card.abilities.iter()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .expect("Should have 登場 ability");
    assert!(ab1.full_text.contains("Liella!"),
        "Debut ability should mention Liella!");
    assert!(ab1.full_text.contains("バトンタッチ"),
        "Debut ability should mention baton touch");
}
