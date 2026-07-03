//! Three-Level Verification Tests for `splice` and `stitch`
//!
//! - `splice n indices` — like `slice`, but time-stretches each slice (via playback
//!   `speed`) so it fills its event duration (beat-locked slicing). Contrast with
//!   `slice`, which plays each slice at natural speed (leaving gaps).
//! - `stitch bool a b` — boolean pattern interleaves two value patterns, taking
//!   STRUCTURE from the boolean (the complement of `sew`).

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

fn cycle_state(cycle: i64) -> State {
    State {
        span: TimeSpan::new(
            Fraction::from_float(cycle as f64),
            Fraction::from_float((cycle + 1) as f64),
        ),
        controls: HashMap::new(),
    }
}

// ============================================================================
// LEVEL 1: Pattern Query Verification — splice
// ============================================================================

#[test]
fn test_splice_level1_event_count() {
    // splice 8 "0 1 2 3" -> 4 events (one per index)
    let base = parse_mini_notation("break");
    let spliced = base.splice_pattern(8, parse_mini_notation("0 1 2 3"));

    let haps = spliced.query(&cycle_state(0));
    assert_eq!(haps.len(), 4, "splice 8 \"0 1 2 3\" should produce 4 events, got {}", haps.len());
    println!("✅ splice L1: 4 events");
}

#[test]
fn test_splice_level1_speed_fills_event() {
    // Each 1/8 slice must be stretched to fill its 1/4-cycle slot:
    //   speed = slice_size / event_duration = (1/8) / (1/4) = 0.5
    let base = parse_mini_notation("break");
    let spliced = base.splice_pattern(8, parse_mini_notation("0 1 2 3"));

    let haps = spliced.query(&cycle_state(0));
    assert_eq!(haps.len(), 4);
    for hap in &haps {
        let speed: f64 = hap
            .context
            .get("speed")
            .expect("splice must set speed context")
            .parse()
            .unwrap();
        assert!(
            (speed - 0.5).abs() < 1e-6,
            "splice speed should be 0.5 (fills 1/4 slot with 1/8 slice), got {}",
            speed
        );
    }
    println!("✅ splice L1: speed = 0.5 fills each event");
}

#[test]
fn test_splice_level1_speed_varies_with_event_duration() {
    // "0 1" -> 2 events of 1/2 cycle each. slice_size=1/4 -> speed=(1/4)/(1/2)=0.5
    let base = parse_mini_notation("break");
    let spliced = base.clone().splice_pattern(4, parse_mini_notation("0 1"));
    let haps = spliced.query(&cycle_state(0));
    assert_eq!(haps.len(), 2);
    for hap in &haps {
        let speed: f64 = hap.context.get("speed").unwrap().parse().unwrap();
        assert!((speed - 0.5).abs() < 1e-6, "speed should be 0.5, got {}", speed);
    }

    // "0 1 2 3 4 5 6 7" -> 8 events of 1/8 each, slice_size=1/8 -> speed=1.0
    let spliced8 = base.splice_pattern(8, parse_mini_notation("0 1 2 3 4 5 6 7"));
    let haps8 = spliced8.query(&cycle_state(0));
    assert_eq!(haps8.len(), 8);
    for hap in &haps8 {
        let speed: f64 = hap.context.get("speed").unwrap().parse().unwrap();
        assert!((speed - 1.0).abs() < 1e-6, "full-grid speed should be 1.0, got {}", speed);
    }
    println!("✅ splice L1: speed tracks event duration");
}

#[test]
fn test_splice_level1_begin_end_matches_slice() {
    // splice must set the SAME begin/end slice boundaries as slice.
    let base = parse_mini_notation("break");
    let spliced = base.splice_pattern(8, parse_mini_notation("0 1 2 3"));
    let haps = spliced.query(&cycle_state(0));

    // Sort by time
    let mut sorted = haps.clone();
    sorted.sort_by(|a, b| a.part.begin.to_float().partial_cmp(&b.part.begin.to_float()).unwrap());

    let expected = [(0.0, 0.125), (0.125, 0.25), (0.25, 0.375), (0.375, 0.5)];
    for (hap, (eb, ee)) in sorted.iter().zip(expected.iter()) {
        let begin: f64 = hap.context.get("begin").unwrap().parse().unwrap();
        let end: f64 = hap.context.get("end").unwrap().parse().unwrap();
        assert!((begin - eb).abs() < 1e-6, "begin: expected {}, got {}", eb, begin);
        assert!((end - ee).abs() < 1e-6, "end: expected {}, got {}", ee, end);
    }
    println!("✅ splice L1: begin/end slice boundaries correct");
}

#[test]
fn test_splice_vs_slice_speed_context() {
    // The distinguishing feature: slice sets NO speed context, splice DOES.
    let base = parse_mini_notation("break");
    let sliced = base.clone().slice_pattern(8, parse_mini_notation("0 1 2 3"));
    let spliced = base.splice_pattern(8, parse_mini_notation("0 1 2 3"));

    let slice_haps = sliced.query(&cycle_state(0));
    let splice_haps = spliced.query(&cycle_state(0));

    assert!(slice_haps.iter().all(|h| h.context.get("speed").is_none()),
        "plain slice must NOT set speed context");
    assert!(splice_haps.iter().all(|h| h.context.get("speed").is_some()),
        "splice MUST set speed context on every event");
    println!("✅ splice vs slice: speed context present only for splice");
}

#[test]
fn test_splice_level1_multicycle() {
    let base = parse_mini_notation("break");
    let spliced = base.splice_pattern(8, parse_mini_notation("0 1 2 3"));
    let mut total = 0;
    for c in 0..4 {
        total += spliced.query(&cycle_state(c)).len();
    }
    assert_eq!(total, 16, "4 cycles x 4 events = 16");
    println!("✅ splice L1: multicycle consistent");
}

#[test]
fn test_splice_index_wraps() {
    // index >= n wraps via modulo (matching slice_pattern)
    let base = parse_mini_notation("break");
    let spliced = base.splice_pattern(4, parse_mini_notation("0 4 5 7"));
    let haps = spliced.query(&cycle_state(0));
    assert_eq!(haps.len(), 4);
    let mut sorted = haps.clone();
    sorted.sort_by(|a, b| a.part.begin.to_float().partial_cmp(&b.part.begin.to_float()).unwrap());
    // 0%4=0, 4%4=0, 5%4=1, 7%4=3
    let expected_begin = [0.0, 0.0, 0.25, 0.75];
    for (hap, eb) in sorted.iter().zip(expected_begin.iter()) {
        let begin: f64 = hap.context.get("begin").unwrap().parse().unwrap();
        assert!((begin - eb).abs() < 1e-6, "begin: expected {}, got {}", eb, begin);
    }
    println!("✅ splice L1: index wraps modulo n");
}

// ============================================================================
// LEVEL 1: Pattern Query Verification — stitch
// ============================================================================

#[test]
fn test_stitch_level1_selects_from_true_false() {
    // stitch "t f t f" "1 2 3" "4 5 6"
    // bool events at 0(t), 1/4(f), 1/2(t), 3/4(f)
    //   t@0   -> a@0   = 1
    //   f@1/4 -> b@1/4 = 4
    //   t@1/2 -> a@1/2 = 2
    //   f@3/4 -> b@3/4 = 6
    let bool_pat = parse_mini_notation("t f t f");
    let a = parse_mini_notation("1 2 3");
    let b = parse_mini_notation("4 5 6");
    let stitched = Pattern::stitch(bool_pat, a, b);

    let mut haps = stitched.query(&cycle_state(0));
    haps.sort_by(|x, y| x.part.begin.to_float().partial_cmp(&y.part.begin.to_float()).unwrap());

    assert_eq!(haps.len(), 4, "stitch should produce 4 events (structure from bool)");
    let values: Vec<String> = haps.iter().map(|h| h.value.clone()).collect();
    assert_eq!(values, vec!["1", "4", "2", "6"], "got {:?}", values);
    println!("✅ stitch L1: selects a on true, b on false -> {:?}", values);
}

#[test]
fn test_stitch_level1_structure_from_bool() {
    // The number/timing of events comes from the boolean pattern, not the values.
    // "t t t" -> 3 events all from a; "f f" -> 2 events all from b.
    let all_true = Pattern::stitch(
        parse_mini_notation("t t t"),
        parse_mini_notation("x"),
        parse_mini_notation("y"),
    );
    let haps = all_true.query(&cycle_state(0));
    assert_eq!(haps.len(), 3, "3 true events -> 3 events");
    assert!(haps.iter().all(|h| h.value == "x"), "all true -> all from a");

    let all_false = Pattern::stitch(
        parse_mini_notation("f f"),
        parse_mini_notation("x"),
        parse_mini_notation("y"),
    );
    let haps2 = all_false.query(&cycle_state(0));
    assert_eq!(haps2.len(), 2, "2 false events -> 2 events");
    assert!(haps2.iter().all(|h| h.value == "y"), "all false -> all from b");
    println!("✅ stitch L1: structure from boolean pattern");
}

#[test]
fn test_stitch_is_complement_of_sew_structure() {
    // sew keeps value-pattern structure; stitch imposes boolean structure.
    // With a bool "t f" and dense value patterns, event counts differ.
    let bool_pat = parse_mini_notation("t f");
    let a = parse_mini_notation("1 2 3 4");
    let b = parse_mini_notation("5 6 7 8");

    let stitched = Pattern::stitch(bool_pat.clone(), a.clone(), b.clone());
    let sewn = Pattern::sew(bool_pat, a, b);

    let stitch_count = stitched.query(&cycle_state(0)).len();
    let sew_count = sewn.query(&cycle_state(0)).len();

    // stitch: 2 events (one per bool event). sew: keeps 4-event value structure.
    assert_eq!(stitch_count, 2, "stitch structure from bool = 2 events");
    assert!(sew_count > stitch_count,
        "sew keeps denser value structure ({} > {})", sew_count, stitch_count);
    println!("✅ stitch/sew: stitch={}, sew={}", stitch_count, sew_count);
}

// ============================================================================
// LEVEL 2 & 3: Audio rendering
// ============================================================================

mod audio_test_utils;
mod pattern_verification_utils;
use pattern_verification_utils::{calculate_peak, calculate_rms, detect_audio_events};

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn render_dsl(code: &str, cycles: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Parse failed");
    let sample_rate = 44100.0;
    let mut graph = compile_program(statements, sample_rate, None).expect("Compile failed");
    let samples_per_cycle = (sample_rate as f64 / 0.5) as usize; // tempo = 0.5 cps
    let total_samples = samples_per_cycle * cycles;
    graph.render(total_samples)
}

fn has_nan(audio: &[f32]) -> bool {
    audio.iter().any(|x| x.is_nan() || x.is_infinite())
}

#[test]
fn test_splice_level2_renders_breakbeat() {
    let code = r#"
tempo: 0.5
out $ s "breaks125" $ splice 8 "0 1 2 3 4 5 6 7"
"#;
    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);
    println!("splice breakbeat: rms={:.4}, peak={:.4}, nan={}", rms, peak, has_nan(&audio));

    assert!(!has_nan(&audio), "no NaN/Inf in spliced audio");
    assert!(rms > 0.01, "spliced breakbeat should be audible (rms={})", rms);
    assert!(peak <= 1.0001, "no clipping beyond limiter (peak={})", peak);
}

#[test]
fn test_splice_level2_onsets_match_pattern() {
    // 4 index events per cycle -> expect ~4 onsets/cycle
    let code = r#"
tempo: 0.5
out $ s "breaks125" $ splice 8 "0 1 2 3"
"#;
    let cycles = 4;
    let audio = render_dsl(code, cycles);
    let onsets = detect_audio_events(&audio, 44100.0, 0.02);
    println!("splice onsets over {} cycles: {}", cycles, onsets.len());

    // At least one onset per cycle; should be in the neighbourhood of 4/cycle.
    assert!(onsets.len() >= cycles, "expected >= {} onsets, got {}", cycles, onsets.len());
    assert!(!has_nan(&audio));
}

#[test]
fn test_splice_vs_slice_time_stretch_contrast() {
    // splice stretches 1/8 slices (speed 0.5) to fill 1/4 slots; slice leaves gaps.
    // => spliced audio sustains longer, so overall RMS should be >= slice RMS.
    let slice_code = r#"
tempo: 0.5
out $ s "breaks125" $ slice 8 "0 1 2 3"
"#;
    let splice_code = r#"
tempo: 0.5
out $ s "breaks125" $ splice 8 "0 1 2 3"
"#;
    let cycles = 4;
    let slice_audio = render_dsl(slice_code, cycles);
    let splice_audio = render_dsl(splice_code, cycles);

    let slice_rms = calculate_rms(&slice_audio);
    let splice_rms = calculate_rms(&splice_audio);
    println!("slice_rms={:.4}, splice_rms={:.4}, ratio={:.2}",
        slice_rms, splice_rms, splice_rms / slice_rms.max(1e-9));

    assert!(!has_nan(&slice_audio) && !has_nan(&splice_audio));
    assert!(slice_rms > 0.001 && splice_rms > 0.001, "both should be audible");
    // Stretched slices fill gaps -> at least as much energy as plain slice.
    assert!(splice_rms >= slice_rms * 0.9,
        "spliced (stretched) should sustain >= slice: slice={:.4}, splice={:.4}",
        slice_rms, splice_rms);
}

#[test]
fn test_stitch_level2_renders() {
    // stitch two drum patterns via a boolean
    let code = r#"
tempo: 0.5
out $ stitch "t f t f" "bd sn" "hh cp"
"#;
    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);
    println!("stitch: rms={:.4}, peak={:.4}, nan={}", rms, peak, has_nan(&audio));
    assert!(!has_nan(&audio), "no NaN/Inf in stitch audio");
    assert!(rms > 0.01, "stitch should be audible (rms={})", rms);
    assert!(peak <= 1.0001, "no clipping (peak={})", peak);
}
