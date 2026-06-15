mod helpers;
mod test_modules;

#[test]
fn test_parse_heart_color() {
    use rabuka_engine::card::{parse_heart_color, HeartColor};
    assert_eq!(parse_heart_color("heart00"), HeartColor::Heart00);
    assert_eq!(parse_heart_color("heart01"), HeartColor::Heart01);
    assert_eq!(parse_heart_color("heart06"), HeartColor::Heart06);
    assert_eq!(parse_heart_color("b_heart01"), HeartColor::Heart01);
    assert_eq!(parse_heart_color("b_heart03"), HeartColor::Heart03);
    assert_eq!(parse_heart_color("b_heart06"), HeartColor::Heart06);
    assert_eq!(parse_heart_color("b_all"), HeartColor::BAll);
    assert_eq!(parse_heart_color("draw"), HeartColor::Draw);
    assert_eq!(parse_heart_color("score"), HeartColor::Score);
    assert_eq!(parse_heart_color("bogus"), HeartColor::Heart00);
}
