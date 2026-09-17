use crate::helpers::*;
use rabuka_engine::ability::resolver::AbilityResolver;
use rabuka_engine::card::{AbilityEffect, DistinctType, EffectFilter, EffectKind};
use rabuka_engine::core::pool::EkBox;

fn draw_effect(distinct: Option<DistinctType>) -> AbilityEffect {
    AbilityEffect {
        kind: Some(EkBox::new(EffectKind::DrawCards {
            filter: Some(Box::new(EffectFilter {
                distinct: distinct.map(Box::new),
                ..Default::default()
            })),
        })),
        ..Default::default()
    }
}

#[test]
fn distinct_draw_preserves_duplicates_and_scans_for_another_name() {
    for source in ["deck", "discard"] {
        let mut game = TestGame::new(load_real_database());
        let first = game.id("PL!-sd1-010-SD");
        let duplicate = game.id("PL!-sd1-010-SD");
        let other = game.id("PL!HS-bp1-005-P");
        assert_ne!(first, duplicate);
        assert_eq!(
            game.db.get_card(first).unwrap().name,
            game.db.get_card(duplicate).unwrap().name
        );
        assert_ne!(
            game.db.get_card(first).unwrap().name,
            game.db.get_card(other).unwrap().name
        );
        if source == "deck" {
            game.state
                .player1
                .main_deck
                .cards
                .extend([first, duplicate, other]);
        } else {
            game.state
                .player1
                .waitroom
                .cards
                .extend([other, duplicate, first]);
        }
        let mut resolver = AbilityResolver::new(game.state.card_database.clone(), None);
        resolver
            .execute_draw(
                &mut game.state,
                &draw_effect(Some(DistinctType::CardName)),
                2,
                "self",
                source,
                "hand",
                None,
                false,
                1,
                None,
            )
            .unwrap();
        assert_eq!(game.state.player1.hand.cards.as_slice(), &[first, other]);
        let remaining = if source == "deck" {
            game.state.player1.main_deck.cards.as_slice()
        } else {
            game.state.player1.waitroom.cards.as_slice()
        };
        assert_eq!(remaining, &[duplicate]);
    }
}

#[test]
fn distinct_draw_stops_when_only_duplicates_remain() {
    for source in ["deck", "discard"] {
        let mut game = TestGame::new(load_real_database());
        let first = game.id("PL!-sd1-010-SD");
        let duplicate = game.id("PL!-sd1-010-SD");
        if source == "deck" {
            game.state
                .player1
                .main_deck
                .cards
                .extend([first, duplicate]);
        } else {
            game.state.player1.waitroom.cards.extend([duplicate, first]);
        }
        let mut resolver = AbilityResolver::new(game.state.card_database.clone(), None);
        resolver
            .execute_draw(
                &mut game.state,
                &draw_effect(Some(DistinctType::True)),
                3,
                "self",
                source,
                "hand",
                None,
                false,
                1,
                None,
            )
            .unwrap();
        assert_eq!(game.state.player1.hand.cards.as_slice(), &[first]);
        let remaining = if source == "deck" {
            game.state.player1.main_deck.cards.as_slice()
        } else {
            game.state.player1.waitroom.cards.as_slice()
        };
        assert_eq!(remaining, &[duplicate]);
    }
}

#[test]
fn ordinary_draw_keeps_same_name_cards() {
    for source in ["deck", "discard"] {
        let mut game = TestGame::new(load_real_database());
        let first = game.id("PL!-sd1-010-SD");
        let duplicate = game.id("PL!-sd1-010-SD");
        if source == "deck" {
            game.state.player1.main_deck.cards.extend([first, duplicate]);
        } else {
            game.state.player1.waitroom.cards.extend([duplicate, first]);
        }
        let mut resolver = AbilityResolver::new(game.state.card_database.clone(), None);
        resolver
            .execute_draw(
                &mut game.state,
                &draw_effect(None),
                2,
                "self",
                source,
                "hand",
                None,
                false,
                1,
                None,
            )
            .unwrap();
        assert_eq!(game.state.player1.hand.cards.as_slice(), &[first, duplicate]);
        assert!(game.state.player1.main_deck.cards.is_empty());
        assert!(game.state.player1.waitroom.cards.is_empty());
    }
}

#[test]
fn filtered_draw_stops_after_scanning_nonmatching_deck() {
    for distinct in [None, Some(DistinctType::CardName)] {
        let mut game = TestGame::new(load_real_database());
        let member = game.id("PL!-sd1-010-SD");
        game.state.player1.main_deck.cards.push(member);
        let mut resolver = AbilityResolver::new(game.state.card_database.clone(), None);
        resolver
            .execute_draw(
                &mut game.state,
                &draw_effect(distinct),
                2,
                "self",
                "deck",
                "hand",
                Some("live_card"),
                false,
                1,
                None,
            )
            .unwrap();
        assert!(game.state.player1.hand.cards.is_empty());
        assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[member]);
    }
}
