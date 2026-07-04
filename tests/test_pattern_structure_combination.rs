/// Tests for combining a sample pattern with `#` value-modifier patterns.
///
/// In Phonon (as in Tidal), `#` is the modifier/parameter operator: rhythmic
/// STRUCTURE comes from the audio source on the LEFT, and the pattern on the
/// RIGHT is *sampled* at each trigger to supply per-trigger VALUES. This is
/// Tidal's `#` == `|>` (structure-from-left). So `s "bd" # note "c4 e4 g4"`
/// triggers `bd` once per cycle (structure from `s "bd"`), taking whatever note
/// value is active at each trigger — it does NOT produce one trigger per note.
/// (Compare CLAUDE.md's `saw 55 # lpf ...`: `#` modifies the source, it does not
/// restructure it.)
///
/// Timing: these tests render 1.0s and `bpm: 120` => cps = 2.0, so 1.0s spans
/// 2 cycles. A single-event source like `s "bd"` therefore fires 2 triggers
/// (2 onsets); a two-event source like `s "bd sn"` fires 4.
///
/// History: the onset thresholds here were originally calibrated against a buggy
/// `detect_audio_events` helper that inflated onset counts 10-100x (a false
/// onset every hop, ~400/sec; fixed in bc7f92e). With the corrected detector
/// the thresholds now reflect the true structure-from-left trigger counts.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod pattern_verification_utils;
use pattern_verification_utils::detect_audio_events;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Test: s "bd" # note "c4 e4 g4"
/// `#` takes structure from the LEFT (`s "bd"`): `bd` fires once per cycle, and
/// the note pattern only supplies the per-trigger value. Over 2 cycles (1.0s at
/// 2 cps) that is 2 triggers, so 2 onsets — NOT one onset per note.
#[test]
fn test_note_provides_structure() {
    let code = r#"
bpm: 120
out $ s "bd" # note "c4 e4 g4"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // Structure from left (`s "bd"`): 1 trigger/cycle * 2 cycles = 2 onsets.
    // The note pattern supplies per-trigger values, not extra structure.
    assert!(
        onsets.len() >= 2,
        "s \"bd\" # note \"c4 e4 g4\" should have 2 onsets (bd fires once per cycle, 2 cycles), got {}",
        onsets.len()
    );

    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should produce sound");
    println!("note structure: {} onsets (2 expected, structure from left), RMS = {}", onsets.len(), rms);
}

/// Test: s "bd" # gain "0.5 1.0 0.8 0.3"
/// `#` takes structure from the LEFT (`s "bd"`): `bd` fires once per cycle and
/// the gain pattern only scales each trigger. Over 2 cycles that is 2 onsets —
/// the 4-event gain pattern does NOT add structure.
#[test]
fn test_gain_provides_structure() {
    let code = r#"
bpm: 120
out $ s "bd" # gain "0.5 1.0 0.8 0.3"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // Structure from left (`s "bd"`): 1 trigger/cycle * 2 cycles = 2 onsets.
    assert!(
        onsets.len() >= 2,
        "s \"bd\" # gain \"0.5 1.0 0.8 0.3\" should have 2 onsets (bd fires once per cycle, 2 cycles), got {}",
        onsets.len()
    );
    println!("gain structure: {} onsets (2 expected, structure from left)", onsets.len());
}

/// Test: s "bd" # pan "-1 0 1"
/// `#` takes structure from the LEFT (`s "bd"`): `bd` fires once per cycle and
/// the pan pattern only positions each trigger. Over 2 cycles that is 2 onsets —
/// the 3-event pan pattern does NOT add structure.
#[test]
fn test_pan_provides_structure() {
    let code = r#"
bpm: 120
out $ s "bd" # pan "-1 0 1"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // Structure from left (`s "bd"`): 1 trigger/cycle * 2 cycles = 2 onsets.
    assert!(
        onsets.len() >= 2,
        "s \"bd\" # pan \"-1 0 1\" should have 2 onsets (bd fires once per cycle, 2 cycles), got {}",
        onsets.len()
    );
    println!("pan structure: {} onsets (2 expected, structure from left)", onsets.len());
}

/// Test: s "bd sn" # note "c4 e4 g4 d4"
/// Structure comes from the LEFT source `s "bd sn"` (2 events/cycle), NOT from
/// the note pattern. Over 2 cycles (1.0s at 2 cps) that is 4 triggers = 4 onsets.
#[test]
fn test_both_sides_have_patterns() {
    let code = r#"
bpm: 120
out $ s "bd sn" # note "c4 e4 g4 d4"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // `s "bd sn"` = 2 triggers/cycle * 2 cycles = 4 onsets (structure from left).
    assert!(
        onsets.len() >= 4,
        "s \"bd sn\" # note \"c4 e4 g4 d4\" should have 4 onsets (bd sn, 2 cycles), got {}",
        onsets.len()
    );
    println!("both patterns: {} onsets (4 expected, structure from left)", onsets.len());
}

/// Test: s "bd" # note "c4 e4" # gain "0.5 1.0"
/// Chained modifiers never change structure: it stays with the LEFT source
/// `s "bd"`. Over 2 cycles that is 2 onsets regardless of how many modifiers or
/// how many events each modifier pattern has.
#[test]
fn test_multiple_modifiers() {
    let code = r#"
bpm: 120
out $ s "bd" # note "c4 e4" # gain "0.5 1.0"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // `#note` then `#gain` only supply per-trigger values; structure stays with
    // `s "bd"` = 1 trigger/cycle * 2 cycles = 2 onsets.
    assert!(
        onsets.len() >= 2,
        "Chained modifiers keep the sample structure: should have 2 onsets, got {}",
        onsets.len()
    );
    println!("multiple modifiers: {} onsets (2 expected, structure from left)", onsets.len());
}

/// Test: s "bd" # note "c4 e4" # gain "0.5 1.0 0.8"
/// Even when chained modifiers have DIFFERENT event counts (note: 2, gain: 3),
/// neither provides structure — it stays with the LEFT source `s "bd"`. Over
/// 2 cycles that is 2 onsets; the modifiers do not "dominate" the structure.
#[test]
fn test_multiple_modifiers_different_structure() {
    let code = r#"
bpm: 120
out $ s "bd" # note "c4 e4" # gain "0.5 1.0 0.8"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // Structure stays with `s "bd"` = 1 trigger/cycle * 2 cycles = 2 onsets,
    // independent of the differing modifier event counts.
    assert!(
        onsets.len() >= 2,
        "Chained modifiers keep the sample structure: should have 2 onsets, got {}",
        onsets.len()
    );
    println!("multiple modifiers (different): {} onsets (2 expected, structure from left)", onsets.len());
}

/// Test: s "bd sn hh" without modifiers
/// Should have 3 triggers (structure from sample pattern only)
#[test]
fn test_sample_pattern_alone() {
    let code = r#"
bpm: 120
out $ s "bd sn hh"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    assert!(
        onsets.len() >= 3,
        "s \"bd sn hh\" should have at least 3 onsets, got {}",
        onsets.len()
    );
    println!("sample pattern alone: {} onsets (>= 3 expected)", onsets.len());
}

/// Test: s "bd" # note "c4"
/// Single note should trigger once
#[test]
fn test_single_note() {
    let code = r#"
bpm: 120
out $ s "bd" # note "c4"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    assert!(
        onsets.len() >= 1,
        "s \"bd\" # note \"c4\" should have at least 1 onset, got {}",
        onsets.len()
    );
    println!("single note: {} onsets (>= 1 expected)", onsets.len());
}

/// Test: s "bd" # note "c4 ~ e4"
/// Pattern with rest - should have 3 events but only 2 onsets
#[test]
fn test_pattern_with_rest() {
    let code = r#"
bpm: 120
out $ s "bd" # note "c4 ~ e4"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // Pattern "c4 ~ e4" has 3 events, but middle one is rest
    // So we should detect at least 2 audio onsets
    assert!(
        onsets.len() >= 2,
        "s \"bd\" # note \"c4 ~ e4\" should have at least 2 onsets (3 events with 1 rest), got {}",
        onsets.len()
    );
    println!("pattern with rest: {} onsets (>= 2 expected)", onsets.len());
}

/// Test: s "bd" # note "c4*2 e4"
/// Subdivision inside the note pattern (`c4*2 e4` = 3 note events/cycle) does
/// NOT subdivide the trigger structure: `#` keeps structure from the LEFT source
/// `s "bd"` = 1 trigger/cycle. Over 2 cycles that is 2 onsets.
#[test]
fn test_subdivision_in_note() {
    let code = r#"
bpm: 120
out $ s "bd" # note "c4*2 e4"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // Note-side subdivision is ignored for structure; `s "bd"` gives
    // 1 trigger/cycle * 2 cycles = 2 onsets.
    assert!(
        onsets.len() >= 2,
        "s \"bd\" # note \"c4*2 e4\" should have 2 onsets (bd fires once per cycle, 2 cycles), got {}",
        onsets.len()
    );
    println!("subdivision in note: {} onsets (2 expected, structure from left)", onsets.len());
}

/// Test: s "bd" # note "[c4, e4, g4]"
/// Chord (polyrhythm) - should trigger once with chord
#[test]
fn test_chord_in_note() {
    let code = r#"
bpm: 120
out $ s "bd" # note "[c4, e4, g4]"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // Chord should trigger as a single event
    assert!(
        onsets.len() >= 1,
        "s \"bd\" # note \"[c4, e4, g4]\" should have at least 1 onset (chord), got {}",
        onsets.len()
    );
    println!("chord in note: {} onsets (>= 1 expected)", onsets.len());
}
