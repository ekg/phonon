use phonon::phonon_lang::PhononParser;

#[test]
fn test_dsl_pattern_gain() {
    let code = r#"
tempo: 2.0
out: s "bd bd" # gain "1.0 0.5"
"#;

    let mut parser = PhononParser::new();
    parser.parse(code).expect("Parse failed");

    // Render 0.5 seconds (1 cycle at tempo 2.0)
    let buffer = parser.graph_mut().render(22050);

    // Split into halves
    let first_half = &buffer[0..11025];
    let second_half = &buffer[11025..22050];

    let first_peak = first_half.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let second_peak = second_half.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("DSL Pattern Gain Test:");
    println!("First BD (gain=1.0):  {:.6}", first_peak);
    println!("Second BD (gain=0.5): {:.6}", second_peak);
    println!("Ratio: {:.3} (expected 2.0)", first_peak / second_peak);

    // The first BD should be roughly 2x louder than the second
    assert!((first_peak / second_peak - 2.0).abs() < 0.2,
            "DSL pattern gain not working: ratio = {:.3}, expected 2.0",
            first_peak / second_peak);
}
