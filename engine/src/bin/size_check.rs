fn main() {
    macro_rules! sz {
        ($t:ty) => {
            println!("{:<35} {}", stringify!($t), std::mem::size_of::<$t>())
        };
    }

    println!("=== The REAL offenders ===");
    sz!(rabuka_engine::card::Condition);
    sz!(Option<rabuka_engine::card::Condition>);
    sz!(Box<rabuka_engine::card::Condition>);
    sz!(Option<Box<rabuka_engine::card::Condition>>);
    println!();
    sz!(rabuka_engine::card::CompoundBranch);
    sz!(rabuka_engine::card::AbilityEffect);
    sz!(rabuka_engine::card::EffectKind);
    println!();
    sz!(Option<String>);
    sz!(Option<Box<str>>);
    sz!(Box<str>);
    sz!(Vec<String>);
    sz!(Option<Vec<String>>);
    sz!(Box<Vec<String>>);
}
