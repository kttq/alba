use alba::weather::Condition;

#[test]
fn wmo_codes_map_to_the_nine_conditions() {
    let table = [
        (&[0, 1][..], Condition::Clear),
        (&[2], Condition::PartlyCloudy),
        (&[3], Condition::Cloudy),
        (&[45, 48], Condition::Fog),
        (&[51, 53, 55, 56, 57], Condition::Drizzle),
        (&[61, 63, 66, 80, 81], Condition::Rain),
        (&[65, 67, 82], Condition::HeavyRain),
        (&[71, 73, 75, 77, 85, 86], Condition::Snow),
        (&[95, 96, 97, 99], Condition::Thunderstorm),
    ];

    for (codes, condition) in table {
        for &code in codes {
            assert_eq!(Condition::from_wmo(code), Some(condition), "code {code}");
        }
    }
}

#[test]
fn unknown_wmo_codes_have_no_condition() {
    for code in [4, 10, 44, 50, 60, 68, 70, 90, 98, 100, 300] {
        assert_eq!(Condition::from_wmo(code), None, "code {code}");
    }
}
