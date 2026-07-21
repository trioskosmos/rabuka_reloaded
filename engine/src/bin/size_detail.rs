fn main() {
    macro_rules! sz {
        ($t:ty) => {
            println!("{:<45} {}", stringify!($t), std::mem::size_of::<$t>())
        };
    }

    println!("=== Core types ===");
    sz!(rabuka_engine::card::EffectKind);
    sz!(rabuka_engine::card::Condition);
    sz!(rabuka_engine::card::AbilityEffect);
    println!();

    println!("=== Inner types (what bloats the variants) ===");
    sz!(rabuka_engine::card::PositionInfo);
    sz!(Option<rabuka_engine::card::PositionInfo>);
    sz!(rabuka_engine::card::DynamicCount);
    sz!(Option<rabuka_engine::card::DynamicCount>);
    sz!(rabuka_engine::card::QuotedText);
    sz!(Option<rabuka_engine::card::QuotedText>);
    println!();

    println!("=== Enums / small types ===");
    sz!(rabuka_engine::card::DistinctType);
    sz!(rabuka_engine::card::Operator);
    sz!(rabuka_engine::card::ComparisonType);
    sz!(rabuka_engine::card::ComparisonTarget);
    sz!(rabuka_engine::card::ConditionCardType);
    sz!(rabuka_engine::card::CardType);
    sz!(rabuka_engine::card::PlacementOrder);
    sz!(rabuka_engine::ability::enums::EffectState);
    sz!(Option<rabuka_engine::ability::enums::EffectState>);
    sz!(rabuka_engine::card::CardProperty);
    sz!(Option<rabuka_engine::card::CardProperty>);
    sz!(rabuka_engine::card::DistinctInfo);
    sz!(Option<rabuka_engine::card::DistinctInfo>);
    sz!(rabuka_engine::card::CardState);
    sz!(Option<rabuka_engine::card::CardState>);
    println!();

    println!("=== Filter types ===");
    sz!(rabuka_engine::card::AbilityFilter);
    sz!(Option<rabuka_engine::card::AbilityFilter>);
    sz!(rabuka_engine::card::AbilityFilterBranch);
    sz!(Option<rabuka_engine::card::AbilityFilterBranch>);
    println!();

    println!("=== LocationSubChecks ===");
    sz!(rabuka_engine::card::LocationSubChecks);
    sz!(Option<Box<rabuka_engine::card::LocationSubChecks>>);
    println!();

    println!("=== PositionCharacter ===");
    sz!(rabuka_engine::card::PositionCharacter);
    sz!(Option<Vec<rabuka_engine::card::PositionCharacter>>);
}
