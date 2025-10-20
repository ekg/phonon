mod pattern_verification_utils;

use pattern_verification_utils::detect_audio_events;
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn investigate_degrade_behavior() {
    // Compare raw audio characteristics
    let input_normal = r#"
        cps: 2.0
        out: s "bd bd bd bd" * 0.5
    "#;

    let input_degraded = r#"
        cps: 2.0
        out: s("bd bd bd bd" $ degrade) * 0.5
    "#;

    // Render both
    let (_, statements) = parse_dsl(input_normal).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio_normal = graph.render(88200);

    let (_, statements) = parse_dsl(input_degraded).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio_degraded = graph.render(88200);

    // Calculate RMS for each
    let rms_normal: f32 =
        (audio_normal.iter().map(|x| x * x).sum::<f32>() / audio_normal.len() as f32).sqrt();
    let rms_degraded: f32 =
        (audio_degraded.iter().map(|x| x * x).sum::<f32>() / audio_degraded.len() as f32).sqrt();

    println!("RMS Normal: {:.6}", rms_normal);
    println!("RMS Degraded: {:.6}", rms_degraded);
    println!(
        "RMS Ratio (degraded/normal): {:.2}",
        rms_degraded / rms_normal
    );

    // Count non-zero samples
    let non_zero_normal = audio_normal.iter().filter(|&&x| x.abs() > 0.0001).count();
    let non_zero_degraded = audio_degraded.iter().filter(|&&x| x.abs() > 0.0001).count();

    println!("Non-zero samples Normal: {}", non_zero_normal);
    println!("Non-zero samples Degraded: {}", non_zero_degraded);

    // Detect events with different thresholds
    for threshold in [0.0001, 0.0005, 0.001, 0.005, 0.01] {
        let events_normal = detect_audio_events(&audio_normal, 44100.0, threshold);
        let events_degraded = detect_audio_events(&audio_degraded, 44100.0, threshold);

        println!("\nThreshold {:.4}:", threshold);
        println!("  Normal: {} events", events_normal.len());
        println!("  Degraded: {} events", events_degraded.len());
        if events_normal.len() > 0 {
            println!(
                "  Ratio: {:.2}",
                events_degraded.len() as f32 / events_normal.len() as f32
            );
        }
    }

    // The degraded version should have lower RMS
    assert!(
        rms_degraded < rms_normal,
        "Degraded RMS should be less than normal, got {:.6} vs {:.6}",
        rms_degraded,
        rms_normal
    );
}

#[test]
fn investigate_stutter_behavior() {
    let input_normal = r#"
        cps: 1.0
        out: s "bd ~ sn ~" * 0.5
    "#;

    let input_stutter = r#"
        cps: 1.0
        out: s("bd ~ sn ~" $ stutter 3) * 0.5
    "#;

    let (_, statements) = parse_dsl(input_normal).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio_normal = graph.render(88200);

    let (_, statements) = parse_dsl(input_stutter).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio_stutter = graph.render(88200);

    // Print audio characteristics around expected event times
    println!("Normal pattern - first 0.3 seconds:");
    print_audio_envelope(&audio_normal[..13230], 44100.0);

    println!("\nStutter pattern - first 0.3 seconds:");
    print_audio_envelope(&audio_stutter[..13230], 44100.0);

    // Try very sensitive onset detection
    let events_normal = detect_audio_events(&audio_normal, 44100.0, 0.0001);
    let events_stutter = detect_audio_events(&audio_stutter, 44100.0, 0.0001);

    println!("\nEvent detection (threshold 0.0001):");
    println!("  Normal: {} events", events_normal.len());
    for (i, e) in events_normal.iter().enumerate() {
        println!("    Event {}: t={:.3}s, amp={:.6}", i, e.time, e.amplitude);
    }

    println!("  Stutter: {} events", events_stutter.len());
    for (i, e) in events_stutter.iter().enumerate() {
        println!("    Event {}: t={:.3}s, amp={:.6}", i, e.time, e.amplitude);
    }
}

fn print_audio_envelope(audio: &[f32], sample_rate: f32) {
    let window_ms = 10.0;
    let window_size = (sample_rate * window_ms / 1000.0) as usize;

    for (i, window) in audio.chunks(window_size).enumerate() {
        let rms: f32 = (window.iter().map(|x| x * x).sum::<f32>() / window.len() as f32).sqrt();
        let time_ms = i as f32 * window_ms;

        if rms > 0.0001 {
            println!("  {:.1}ms: RMS={:.6}", time_ms, rms);
        }
    }
}
