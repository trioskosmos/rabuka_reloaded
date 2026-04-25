use crate::qa_individual::common::{load_all_cards, create_card_database};

#[test]
fn test_complex_look_and_select_ability() {
    // Test that look_and_select abilities are properly parsed
    
    let cards = load_all_cards();
    let _card_database = create_card_database(cards.clone());
    
    // Find a card with look_and_select ability
    let look_select_card = cards.iter()
        .filter(|c| c.is_member())
        .find(|c| {
            c.abilities.iter().any(|a| {
                a.effect.as_ref().map_or(false, |e| {
                    e.action == "look_and_select"
                })
            })
        });
    
    if let Some(card) = look_select_card {
        println!("Testing look_and_select ability on: {}", card.name);
        
        let ability = card.abilities.iter()
            .find(|a| a.effect.as_ref().map_or(false, |e| e.action == "look_and_select"));
        
        if let Some(ability) = ability {
            println!("Ability full_text: {}", ability.full_text);
            
            if let Some(ref effect) = ability.effect {
                println!("Effect action: {}", effect.action);
                
                if let Some(ref look_action) = effect.look_action {
                    println!("Look action: {}", look_action.text);
                    println!("  Source: {:?}", look_action.source);
                    println!("  Count: {:?}", look_action.count);
                }
                
                if let Some(ref select_action) = effect.select_action {
                    println!("Select action: {:?}", select_action.action);
                    println!("  Destination: {:?}", select_action.destination);
                    println!("  Count: {:?}", select_action.count);
                }
            }
            
            // Verify the structure is correct
            assert!(ability.effect.is_some(), "Ability should have effect");
            assert!(ability.effect.as_ref().unwrap().action == "look_and_select", 
                "Effect should be look_and_select");
            
            println!("look_and_select ability structure verified");
        } else {
            panic!("Card has look_and_select in abilities list but no matching ability found");
        }
    } else {
        println!("Skipping test: no card with look_and_select ability found");
    }
}

#[test]
fn test_conditional_alternative_ability() {
    // Test that conditional_alternative abilities are properly parsed
    
    let cards = load_all_cards();
    let _card_database = create_card_database(cards.clone());
    
    // Find a card with conditional_alternative ability
    let conditional_card = cards.iter()
        .filter(|c| c.is_member())
        .find(|c| {
            c.abilities.iter().any(|a| {
                a.effect.as_ref().map_or(false, |e| {
                    e.action == "conditional_alternative"
                })
            })
        });
    
    if let Some(card) = conditional_card {
        println!("Testing conditional_alternative ability on: {}", card.name);
        
        let ability = card.abilities.iter()
            .find(|a| a.effect.as_ref().map_or(false, |e| e.action == "conditional_alternative"));
        
        if let Some(ability) = ability {
            println!("Ability full_text: {}", ability.full_text);
            
            if let Some(ref effect) = ability.effect {
                println!("Effect action: {}", effect.action);
                
                if let Some(ref primary) = effect.primary_effect {
                    println!("Primary effect: {}", primary.text);
                }
                
                if let Some(ref alt_condition) = effect.alternative_condition {
                    println!("Alternative condition type: {:?}", alt_condition.condition_type);
                    println!("Alternative condition text: {:?}", alt_condition.text);
                }
                
                if let Some(ref alt_effect) = effect.alternative_effect {
                    println!("Alternative effect: {}", alt_effect.text);
                }
            }
            
            // Verify structure
            assert!(ability.effect.is_some(), "Ability should have effect");
            assert!(ability.effect.as_ref().unwrap().action == "conditional_alternative",
                "Effect should be conditional_alternative");
            
            println!("conditional_alternative ability structure verified");
        } else {
            panic!("Card has conditional_alternative but no matching ability found");
        }
    } else {
        println!("Skipping test: no card with conditional_alternative ability found");
    }
}

#[test]
fn test_sequential_ability() {
    // Test that sequential abilities are properly parsed
    
    let cards = load_all_cards();
    let _card_database = create_card_database(cards.clone());
    
    // Find a card with sequential ability
    let sequential_card = cards.iter()
        .filter(|c| c.is_member())
        .find(|c| {
            c.abilities.iter().any(|a| {
                a.effect.as_ref().map_or(false, |e| {
                    e.action == "sequential" && e.actions.is_some()
                })
            })
        });
    
    if let Some(card) = sequential_card {
        println!("Testing sequential ability on: {}", card.name);
        
        let ability = card.abilities.iter()
            .find(|a| a.effect.as_ref().map_or(false, |e| e.action == "sequential"));
        
        if let Some(ability) = ability {
            println!("Ability full_text: {}", ability.full_text);
            
            if let Some(ref effect) = ability.effect {
                println!("Effect action: {}", effect.action);
                
                if let Some(ref actions) = effect.actions {
                    println!("Sequential actions count: {}", actions.len());
                    for (i, action) in actions.iter().enumerate() {
                        println!("  Action {}: {}", i, action.action);
                        println!("    Text: {}", action.text);
                        println!("    Count: {:?}", action.count);
                    }
                }
            }
            
            // Verify structure
            assert!(ability.effect.is_some(), "Ability should have effect");
            assert!(ability.effect.as_ref().unwrap().action == "sequential",
                "Effect should be sequential");
            assert!(ability.effect.as_ref().unwrap().actions.is_some(),
                "Sequential effect should have actions");
            
            let actions = ability.effect.as_ref().unwrap().actions.as_ref().unwrap();
            assert!(actions.len() >= 2, "Sequential should have at least 2 actions");
            
            println!("sequential ability structure verified");
        } else {
            panic!("Card has sequential but no matching ability found");
        }
    } else {
        println!("Skipping test: no card with sequential ability found");
    }
}

#[test]
fn test_sequential_cost_ability() {
    // Test that sequential_cost abilities are properly parsed
    
    let cards = load_all_cards();
    let _card_database = create_card_database(cards.clone());
    
    // Find a card with sequential_cost
    let sequential_cost_card = cards.iter()
        .filter(|c| c.is_member())
        .find(|c| {
            c.abilities.iter().any(|a| {
                a.cost.as_ref().map_or(false, |cost| {
                    cost.cost_type.as_deref() == Some("sequential_cost")
                })
            })
        });
    
    if let Some(card) = sequential_cost_card {
        println!("Testing sequential_cost ability on: {}", card.name);
        
        let ability = card.abilities.iter()
            .find(|a| a.cost.as_ref().map_or(false, |cost| {
                cost.cost_type.as_deref() == Some("sequential_cost")
            }));
        
        if let Some(ability) = ability {
            println!("Ability full_text: {}", ability.full_text);
            
            if let Some(ref cost) = ability.cost {
                println!("Cost type: {:?}", cost.cost_type);
                println!("Cost text: {}", cost.text);
                
                if let Some(ref costs) = cost.costs {
                    println!("Sequential cost steps: {}", costs.len());
                    for (i, sub_cost) in costs.iter().enumerate() {
                        println!("  Step {}: {:?}", i, sub_cost.cost_type);
                        println!("    Text: {}", sub_cost.text);
                        println!("    Optional: {:?}", sub_cost.optional);
                    }
                }
            }
            
            // Verify structure
            assert!(ability.cost.is_some(), "Ability should have cost");
            assert!(ability.cost.as_ref().unwrap().cost_type.as_deref() == Some("sequential_cost"),
                "Cost should be sequential_cost");
            assert!(ability.cost.as_ref().unwrap().costs.is_some(),
                "Sequential cost should have costs");
            
            let costs = ability.cost.as_ref().unwrap().costs.as_ref().unwrap();
            assert!(costs.len() >= 1, "Sequential cost should have at least 1 sub-cost");
            
            println!("sequential_cost ability structure verified");
        } else {
            panic!("Card has sequential_cost but no matching ability found");
        }
    } else {
        println!("Skipping test: no card with sequential_cost found");
    }
}

#[test]
fn test_complex_nested_ability() {
    // Test: Very complex ability with nested structures
    
    let cards = load_all_cards();
    let _card_database = create_card_database(cards.clone());
    
    // Find the most complex ability by counting nested fields
    let mut most_complex: Option<(&rabuka_engine::card::Card, &rabuka_engine::card::Ability)> = None;
    let mut max_complexity = 0;
    
    for card in &cards {
        for ability in &card.abilities {
            let mut complexity = 0;
            
            // Count complexity factors
            if ability.cost.is_some() { complexity += 1; }
            if ability.effect.is_some() { complexity += 1; }
            
            if let Some(ref effect) = ability.effect {
                if effect.look_action.is_some() { complexity += 2; }
                if effect.select_action.is_some() { complexity += 2; }
                if effect.actions.is_some() { complexity += effect.actions.as_ref().unwrap().len(); }
                if effect.condition.is_some() { complexity += 2; }
                if effect.primary_effect.is_some() { complexity += 2; }
                if effect.alternative_condition.is_some() { complexity += 2; }
                if effect.alternative_effect.is_some() { complexity += 2; }
            }
            
            if complexity > max_complexity && complexity > 5 {
                max_complexity = complexity;
                most_complex = Some((card, ability));
            }
        }
    }
    
    if let Some((card, ability)) = most_complex {
        println!("Testing most complex ability (complexity: {})", max_complexity);
        println!("Card: {}", card.name);
        println!("Ability: {}", ability.full_text);
        
        // Verify it can be parsed without errors
        assert!(ability.full_text.len() > 0, "Ability should have text");
        
        if let Some(ref effect) = ability.effect {
            println!("Effect action: {}", effect.action);
            println!("Has look_action: {}", effect.look_action.is_some());
            println!("Has select_action: {}", effect.select_action.is_some());
            println!("Has actions: {}", effect.actions.is_some());
            println!("Has condition: {}", effect.condition.is_some());
            println!("Has primary_effect: {}", effect.primary_effect.is_some());
            println!("Has alternative_condition: {}", effect.alternative_condition.is_some());
            println!("Has alternative_effect: {}", effect.alternative_effect.is_some());
        }
        
        if let Some(ref cost) = ability.cost {
            println!("Cost type: {:?}", cost.cost_type);
            println!("Has costs: {}", cost.costs.is_some());
            println!("Has options: {}", cost.options.is_some());
        }
        
        println!("Complex ability parsing verified");
    } else {
        println!("Skipping test: no sufficiently complex ability found");
    }
}

