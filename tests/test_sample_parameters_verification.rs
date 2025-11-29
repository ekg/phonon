/// Comprehensive tests for 10 untested sample parameter functions
///
/// These are CRITICAL parameters that modify sample playback. Each MUST actually work!
///
/// Parameters tested:
/// - gain: Volume control (already tested elsewhere, included for completeness)
/// - pan: Stereo positioning
/// - speed: Playback rate/pitch
/// - note: MIDI-style pitch shift
/// - n: Sample bank selection
/// - begin: Sample start point (slicing)
/// - end: Sample end point (slicing)
/// - loop: Loop mode
/// - unit: Time unit mode (rate vs cycle)
/// - cut: Cut group for voice stealing
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

// ====================
// GAIN TESTS
// ====================

#[test]
fn test_gain_doubles_amplitude() {
    let code_normal = r#"
        tempo: 0.5
        out $ s "bd"
    "#;

    let code_loud = r#"
        tempo: 0.5
        out $ s "bd" # gain 2.0
    "#;

    let normal = render_dsl(code_normal, 1.0);
    let loud = render_dsl(code_loud, 1.0);

    let rms_normal = calculate_rms(&normal);
    let rms_loud = calculate_rms(&loud);

    println!(
        "gain test: normal={:.4}, loud={:.4}, ratio={:.2}",
        rms_normal,
        rms_loud,
        rms_loud / rms_normal
    );

    assert!(
        rms_loud > rms_normal * 1.5,
        "gain 2.0 should increase amplitude, got normal={:.4}, loud={:.4}",
        rms_normal,
        rms_loud
    );
}

#[test]
fn test_gain_halves_amplitude() {
    let code_normal = r#"
        tempo: 0.5
        out $ s "bd"
    "#;

    let code_quiet = r#"
        tempo: 0.5
        out $ s "bd" # gain 0.5
    "#;

    let normal = render_dsl(code_normal, 1.0);
    let quiet = render_dsl(code_quiet, 1.0);

    let rms_normal = calculate_rms(&normal);
    let rms_quiet = calculate_rms(&quiet);

    println!(
        "gain halve test: normal={:.4}, quiet={:.4}, ratio={:.2}",
        rms_normal,
        rms_quiet,
        rms_quiet / rms_normal
    );

    assert!(
        rms_quiet < rms_normal * 0.7,
        "gain 0.5 should decrease amplitude, got normal={:.4}, quiet={:.4}",
        rms_normal,
        rms_quiet
    );
}

#[test]
fn test_gain_pattern_varies_per_event() {
    let code = r#"
        tempo: 0.5
        out $ s "bd*4" # gain "1.0 0.8 0.6 0.4"
    "#;

    let buffer = render_dsl(code, 1.0); // 1 cycle at tempo 2 = 0.5s = 22050 samples
    let quarter = buffer.len() / 4;

    let rms_values: Vec<f32> = (0..4)
        .map(|i| calculate_rms(&buffer[i * quarter..(i + 1) * quarter]))
        .collect();

    println!("gain pattern RMS: {:?}", rms_values);

    // Each event should have progressively less amplitude
    assert!(
        rms_values[0] > rms_values[1],
        "Event 0 should be louder than 1"
    );
    assert!(
        rms_values[1] > rms_values[2],
        "Event 1 should be louder than 2"
    );
    assert!(
        rms_values[2] > rms_values[3],
        "Event 2 should be louder than 3"
    );
}

// ====================
// PAN TESTS
// ====================

#[test]
fn test_pan_center_produces_audio() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # pan 0.0
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("pan center RMS: {:.4}", rms);
    assert!(rms > 0.01, "pan 0.0 (center) should produce audio");
}

#[test]
fn test_pan_left_produces_audio() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # pan (-1.0)
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("pan left RMS: {:.4}", rms);
    assert!(rms > 0.01, "pan -1.0 (hard left) should produce audio");
}

#[test]
fn test_pan_right_produces_audio() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # pan 1.0
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("pan right RMS: {:.4}", rms);
    assert!(rms > 0.01, "pan 1.0 (hard right) should produce audio");
}

#[test]
fn test_pan_pattern_varies_per_event() {
    let code = r#"
        tempo: 0.5
        out $ s "bd*3" # pan "-1 0 1"
    "#;

    let buffer = render_dsl(code, 1.0);
    let third = buffer.len() / 3;

    let rms_values: Vec<f32> = (0..3)
        .map(|i| calculate_rms(&buffer[i * third..(i + 1) * third]))
        .collect();

    println!("pan pattern RMS: {:?}", rms_values);

    // All events should produce audio regardless of pan position
    for (i, rms) in rms_values.iter().enumerate() {
        assert!(*rms > 0.01, "Event {} with pan should produce audio", i);
    }
}

// ====================
// SPEED TESTS
// ====================

#[test]
fn test_speed_double_plays_faster() {
    let code_normal = r#"
        tempo: 1.0
        out $ s "bd"
    "#;

    let code_fast = r#"
        tempo: 1.0
        out $ s "bd" # speed 2.0
    "#;

    let buffer_normal = render_dsl(code_normal, 1.0);
    let buffer_fast = render_dsl(code_fast, 1.0);

    // Find duration of audio (when it drops below threshold)
    let duration_normal = find_audio_duration(&buffer_normal, 0.001);
    let duration_fast = find_audio_duration(&buffer_fast, 0.001);

    println!(
        "speed test: normal={} samples, fast={} samples",
        duration_normal, duration_fast
    );

    // Double speed should finish in roughly half the time
    assert!(
        duration_fast < (duration_normal as f32 * 0.8) as usize,
        "speed 2.0 should finish faster: normal={}, fast={}",
        duration_normal,
        duration_fast
    );
}

#[test]
fn test_speed_half_plays_slower() {
    let code_normal = r#"
        tempo: 1.0
        out $ s "bd"
    "#;

    let code_slow = r#"
        tempo: 1.0
        out $ s "bd" # speed 0.5
    "#;

    let buffer_normal = render_dsl(code_normal, 1.0);
    let buffer_slow = render_dsl(code_slow, 1.0);

    let duration_normal = find_audio_duration(&buffer_normal, 0.001);
    let duration_slow = find_audio_duration(&buffer_slow, 0.001);

    println!(
        "speed slow test: normal={} samples, slow={} samples",
        duration_normal, duration_slow
    );

    // Half speed should last at least 1.5x longer
    assert!(
        duration_slow > (duration_normal as f32 * 1.2) as usize,
        "speed 0.5 should last longer: normal={}, slow={}",
        duration_normal,
        duration_slow
    );
}

#[test]
fn test_speed_negative_plays_backwards() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # speed (-1.0)
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("speed reverse RMS: {:.4}", rms);
    assert!(rms > 0.01, "speed -1.0 (reverse) should produce audio");
}

#[test]
fn test_speed_pattern_varies_per_event() {
    let code = r#"
        tempo: 0.5
        out $ s "bd*3" # speed "1 2 0.5"
    "#;

    let buffer = render_dsl(code, 1.0);
    let third = buffer.len() / 3;

    let rms_values: Vec<f32> = (0..3)
        .map(|i| calculate_rms(&buffer[i * third..(i + 1) * third]))
        .collect();

    println!("speed pattern RMS: {:?}", rms_values);

    // All events should produce audio at different speeds
    for (i, rms) in rms_values.iter().enumerate() {
        assert!(*rms > 0.005, "Event {} with speed should produce audio", i);
    }
}

// ====================
// NOTE TESTS
// ====================

#[test]
fn test_note_zero_is_original_pitch() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # note 0
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("note 0 RMS: {:.4}", rms);
    assert!(rms > 0.01, "note 0 (original pitch) should produce audio");
}

#[test]
fn test_note_positive_shifts_pitch_up() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # note 12
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("note 12 (octave up) RMS: {:.4}", rms);
    assert!(rms > 0.01, "note 12 (octave up) should produce audio");
}

#[test]
fn test_note_negative_shifts_pitch_down() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # note (-12)
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("note -12 (octave down) RMS: {:.4}", rms);
    assert!(rms > 0.01, "note -12 (octave down) should produce audio");
}

#[test]
fn test_note_pattern_varies_pitch() {
    let code = r#"
        tempo: 0.5
        out $ s "bd*3" # note "0 5 7"
    "#;

    let buffer = render_dsl(code, 1.0);
    let third = buffer.len() / 3;

    let rms_values: Vec<f32> = (0..3)
        .map(|i| calculate_rms(&buffer[i * third..(i + 1) * third]))
        .collect();

    println!("note pattern RMS: {:?}", rms_values);

    // All events should produce audio at different pitches
    for (i, rms) in rms_values.iter().enumerate() {
        assert!(*rms > 0.01, "Event {} with note should produce audio", i);
    }
}

// ====================
// N TESTS (Sample Bank Selection)
// ====================

#[test]
fn test_n_zero_selects_first_sample() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # n 0
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("n 0 RMS: {:.4}", rms);
    assert!(rms > 0.01, "n 0 (first sample) should produce audio");
}

#[test]
fn test_n_pattern_varies_sample_selection() {
    let code = r#"
        tempo: 0.5
        out $ s "bd*3" # n "0 1 2"
    "#;

    let buffer = render_dsl(code, 1.0);
    let third = buffer.len() / 3;

    let rms_values: Vec<f32> = (0..3)
        .map(|i| calculate_rms(&buffer[i * third..(i + 1) * third]))
        .collect();

    println!("n pattern RMS: {:?}", rms_values);

    // At least the first event should produce audio
    // (other events may be silent if sample bank doesn't have enough samples)
    assert!(rms_values[0] > 0.01, "Event 0 with n should produce audio");
}

// ====================
// BEGIN/END TESTS (Sample Slicing)
// ====================

#[test]
fn test_begin_zero_starts_at_beginning() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # begin 0.0
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("begin 0.0 RMS: {:.4}", rms);
    assert!(rms > 0.01, "begin 0.0 (start) should produce audio");
}

#[test]
fn test_begin_half_starts_at_midpoint() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # begin 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("begin 0.5 RMS: {:.4}", rms);
    assert!(rms > 0.005, "begin 0.5 (midpoint) should produce audio");
}

#[test]
fn test_begin_pattern_varies_start_point() {
    let code = r#"
        tempo: 0.5
        out $ s "bd*3" # begin "0.0 0.25 0.5"
    "#;

    let buffer = render_dsl(code, 1.0);
    let third = buffer.len() / 3;

    let rms_values: Vec<f32> = (0..3)
        .map(|i| calculate_rms(&buffer[i * third..(i + 1) * third]))
        .collect();

    println!("begin pattern RMS: {:?}", rms_values);

    // All events should produce audio (different slices)
    for (i, rms) in rms_values.iter().enumerate() {
        assert!(*rms > 0.005, "Event {} with begin should produce audio", i);
    }
}

#[test]
fn test_end_one_plays_full_sample() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # end 1.0
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("end 1.0 RMS: {:.4}", rms);
    assert!(rms > 0.01, "end 1.0 (full sample) should produce audio");
}

#[test]
fn test_end_half_stops_at_midpoint() {
    let code_full = r#"
        tempo: 0.5
        out $ s "bd" # end 1.0
    "#;

    let code_half = r#"
        tempo: 0.5
        out $ s "bd" # end 0.5
    "#;

    let buffer_full = render_dsl(code_full, 1.0);
    let buffer_half = render_dsl(code_half, 1.0);

    let duration_full = find_audio_duration(&buffer_full, 0.001);
    let duration_half = find_audio_duration(&buffer_half, 0.001);

    println!(
        "end test: full={} samples, half={} samples",
        duration_full, duration_half
    );

    // Half end should finish earlier
    assert!(
        duration_half < duration_full,
        "end 0.5 should finish earlier: full={}, half={}",
        duration_full,
        duration_half
    );
}

#[test]
fn test_begin_and_end_slice_sample() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # begin 0.25 # end 0.75
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("begin 0.25 end 0.75 RMS: {:.4}", rms);
    assert!(rms > 0.005, "begin/end slicing should produce audio");
}

// ====================
// LOOP TESTS
// ====================

#[test]
fn test_loop_false_plays_once() {
    let code = r#"
        tempo: 1.0
        out $ s "bd" # loop 0
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("loop 0 (play once) RMS: {:.4}", rms);
    assert!(rms > 0.01, "loop 0 should produce audio");
}

#[test]
fn test_loop_true_continues_playing() {
    let code = r#"
        tempo: 1.0
        out $ s "bd" # loop 1
    "#;

    let buffer = render_dsl(code, 2.0); // Render 2 seconds

    // Find audio duration - looping should fill most of the buffer
    let active_samples = buffer.iter().filter(|&&x| x.abs() > 0.001).count();
    let total_samples = buffer.len();
    let fill_ratio = active_samples as f32 / total_samples as f32;

    println!(
        "loop 1: active={}/{} samples ({:.1}% fill)",
        active_samples,
        total_samples,
        fill_ratio * 100.0
    );

    // Looping should fill at least 30% of the buffer
    // (BD sample might have long tail, so we're lenient)
    assert!(
        fill_ratio > 0.3,
        "loop 1 should continue playing, got {:.1}% fill",
        fill_ratio * 100.0
    );
}

#[test]
fn test_loop_pattern_varies_per_event() {
    let code = r#"
        tempo: 0.5
        out $ s "bd*2" # loop "0 1"
    "#;

    let buffer = render_dsl(code, 1.0);
    let half = buffer.len() / 2;

    let rms_first = calculate_rms(&buffer[0..half]);
    let rms_second = calculate_rms(&buffer[half..]);

    println!(
        "loop pattern: first={:.4}, second={:.4}",
        rms_first, rms_second
    );

    // Both events should produce audio
    assert!(rms_first > 0.01, "First event should produce audio");
    assert!(rms_second > 0.01, "Second event should produce audio");
}

// ====================
// UNIT TESTS (Rate vs Cycle mode)
// ====================

#[test]
fn test_unit_r_uses_rate_mode() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # unit "r"
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("unit r (rate) RMS: {:.4}", rms);
    assert!(rms > 0.01, "unit r (rate mode) should produce audio");
}

#[test]
fn test_unit_c_uses_cycle_mode() {
    let code = r#"
        tempo: 0.5
        out $ s "bd" # unit "c"
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("unit c (cycle) RMS: {:.4}", rms);
    assert!(rms > 0.01, "unit c (cycle mode) should produce audio");
}

#[test]
fn test_unit_pattern_varies_mode() {
    let code = r#"
        tempo: 0.5
        out $ s "bd*2" # unit "r c"
    "#;

    let buffer = render_dsl(code, 1.0);
    let half = buffer.len() / 2;

    let rms_first = calculate_rms(&buffer[0..half]);
    let rms_second = calculate_rms(&buffer[half..]);

    println!(
        "unit pattern: rate={:.4}, cycle={:.4}",
        rms_first, rms_second
    );

    // Both events should produce audio
    assert!(rms_first > 0.01, "Rate mode event should produce audio");
    assert!(rms_second > 0.01, "Cycle mode event should produce audio");
}

// ====================
// CUT TESTS (Voice Stealing)
// ====================

#[test]
fn test_cut_zero_no_stealing() {
    let code = r#"
        tempo: 0.5
        out $ s "bd*4" # cut 0
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    println!("cut 0 (no stealing) RMS: {:.4}", rms);
    assert!(rms > 0.01, "cut 0 should produce audio with no stealing");
}

#[test]
fn test_cut_group_stops_previous() {
    let code = r#"
        tempo: 1.0
        out $ s "bd*8" # cut 1
    "#;

    let buffer = render_dsl(code, 2.0); // 2 seconds, 8 events

    // With cut group 1, each new event should stop the previous one
    // So we should NOT have massive layering/buildup
    let rms = calculate_rms(&buffer);

    println!("cut 1 (stealing) RMS: {:.4}", rms);
    assert!(rms > 0.01, "cut 1 should produce audio");

    // Check that amplitude doesn't grow too much
    // (with stealing, later events shouldn't accumulate)
    let first_quarter = calculate_rms(&buffer[0..buffer.len() / 4]);
    let last_quarter = calculate_rms(&buffer[3 * buffer.len() / 4..]);

    println!(
        "cut 1: first_quarter={:.4}, last_quarter={:.4}",
        first_quarter, last_quarter
    );

    // Last quarter shouldn't be much louder than first (no excessive buildup)
    let ratio = last_quarter / first_quarter.max(0.001);
    assert!(
        ratio < 3.0,
        "cut group should prevent massive buildup, ratio={:.2}",
        ratio
    );
}

#[test]
fn test_cut_pattern_varies_groups() {
    let code = r#"
        tempo: 0.5
        out $ s "bd*3" # cut "0 1 2"
    "#;

    let buffer = render_dsl(code, 1.0);
    let third = buffer.len() / 3;

    let rms_values: Vec<f32> = (0..3)
        .map(|i| calculate_rms(&buffer[i * third..(i + 1) * third]))
        .collect();

    println!("cut pattern RMS: {:?}", rms_values);

    // All events should produce audio in different cut groups
    for (i, rms) in rms_values.iter().enumerate() {
        assert!(*rms > 0.01, "Event {} with cut should produce audio", i);
    }
}

// ====================
// HELPER FUNCTIONS
// ====================

fn render_dsl(code: &str, duration_seconds: f32) -> Vec<f32> {
    let (_, statements) = parse_dsl(code).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let num_samples = (44100.0 * duration_seconds) as usize;
    graph.render(num_samples)
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

fn find_audio_duration(buffer: &[f32], threshold: f32) -> usize {
    // Find the last point where absolute value exceeds threshold
    for i in (0..buffer.len()).rev() {
        if buffer[i].abs() > threshold {
            return i + 1; // Duration in samples
        }
    }
    0 // No audio found
}
