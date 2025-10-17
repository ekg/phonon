/// Test pattern querying at exact boundaries
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn test_query_pattern_at_boundaries() {
    let pattern = parse_mini_notation("1 2");

    // Query at position 0.0 (first event)
    let state0 = State {
        span: TimeSpan::new(
            Fraction::from_float(0.0),
            Fraction::from_float(0.001),
        ),
        controls: HashMap::new(),
    };
    let events0 = pattern.query(&state0);
    println!("Query at 0.0: {:?}", events0.iter().map(|e| &e.value).collect::<Vec<_>>());

    // Query at position 0.5 (boundary between first and second)
    let state05 = State {
        span: TimeSpan::new(
            Fraction::from_float(0.5),
            Fraction::from_float(0.501),
        ),
        controls: HashMap::new(),
    };
    let events05 = pattern.query(&state05);
    println!("Query at 0.5: {:?}", events05.iter().map(|e| &e.value).collect::<Vec<_>>());

    // Query at position 0.75 (second event)
    let state075 = State {
        span: TimeSpan::new(
            Fraction::from_float(0.75),
            Fraction::from_float(0.751),
        ),
        controls: HashMap::new(),
    };
    let events075 = pattern.query(&state075);
    println!("Query at 0.75: {:?}", events075.iter().map(|e| &e.value).collect::<Vec<_>>());

    // Verify we get the right values
    assert_eq!(events0.len(), 1);
    assert_eq!(events0[0].value, "1");

    assert_eq!(events05.len(), 1);
    assert_eq!(events05[0].value, "2", "At boundary 0.5, should get second value");

    assert_eq!(events075.len(), 1);
    assert_eq!(events075[0].value, "2");
}
