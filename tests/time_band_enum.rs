use proj_xs::models::common::TimeBandEnum;

#[test]
fn get_enum_from_str_valid_values() {
    assert_eq!(
        TimeBandEnum::get_enum_from_str(Some("11:00am - 12:00pm")),
        Some(TimeBandEnum::ElevenAM)
    );
    assert_eq!(
        TimeBandEnum::get_enum_from_str(Some("12:00pm - 01:00pm")),
        Some(TimeBandEnum::TwevlvePM)
    );
}

#[test]
fn get_enum_from_str_invalid_and_none() {
    assert_eq!(TimeBandEnum::get_enum_from_str(Some("invalid")), None);
    assert_eq!(TimeBandEnum::get_enum_from_str(None), None);
    assert_eq!(TimeBandEnum::get_enum_from_str(Some("")), None);
}

#[test]
fn human_readable_returns_expected_strings() {
    assert_eq!(TimeBandEnum::ElevenAM.human_readable(), "11:00am - 12:00pm");
    assert_eq!(
        TimeBandEnum::TwevlvePM.human_readable(),
        "12:00pm - 01:00pm"
    );
}

#[test]
fn round_trip_str_to_enum_to_str() {
    let inputs = ["11:00am - 12:00pm", "12:00pm - 01:00pm"];
    for input in inputs {
        let variant = TimeBandEnum::get_enum_from_str(Some(input))
            .unwrap_or_else(|| panic!("should parse '{input}'"));
        assert_eq!(variant.human_readable(), input);
    }
}
