/// Test to verify that voices don't accumulate across cycles
///
/// BUG REPORT: User observes that samples appear to accumulate across cycles,
/// with each cycle adding MORE voices on top of previous cycles, leading to
/// exponentially increasing density and volume.
///
/// EXPECTED: Voice count should stabilize after first cycle. New events in
/// cycle N should not re-trigger if they're the "same" events from the pattern.
///
/// ACTUAL: Voice count may be growing unboundedly as cycles repeat.

use phonon::unified_graph::{SignalNode, UnifiedSignalGraph};
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use phonon::mini_notation_v3::parse_mini_notation;

#[test]
fn test_voice_count_does_not_accumulate() {
    // Create a simple alternating pattern
    let input = r#"
        tempo: 1.0
        out: s("bd(<3 5>,8)")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Sample for multiple cycles and track voice counts
    let samples_per_cycle = (44100.0 / 1.0) as usize; // 44100 samples at 1 CPS
    let num_cycles = 8;

    let mut voice_counts = Vec::new();
    let mut peak_voice_counts = Vec::new();

    for cycle in 0..num_cycles {
        let mut max_voices_this_cycle = 0;
        let mut sum_voices_this_cycle = 0;

        for _ in 0..samples_per_cycle {
            let _sample = graph.process_sample();
            let voice_count = graph.active_voice_count();

            sum_voices_this_cycle += voice_count;
            if voice_count > max_voices_this_cycle {
                max_voices_this_cycle = voice_count;
            }
        }

        let avg_voices = sum_voices_this_cycle as f32 / samples_per_cycle as f32;
        voice_counts.push(avg_voices);
        peak_voice_counts.push(max_voices_this_cycle);

        println!("Cycle {}: avg_voices={:.2}, peak_voices={}",
                 cycle, avg_voices, max_voices_this_cycle);
    }

    // Check if voice count is growing unboundedly
    // After the first few cycles, voice count should stabilize
    let early_avg = voice_counts[2]; // Cycle 2
    let late_avg = voice_counts[num_cycles - 1]; // Last cycle

    let growth_ratio = late_avg / early_avg.max(0.1);

    println!("\nVoice count growth ratio (cycle 7 / cycle 2): {:.2}x", growth_ratio);
    println!("Early average (cycle 2): {:.2}", early_avg);
    println!("Late average (cycle 7): {:.2}", late_avg);

    // Voice count should NOT grow significantly
    // Allow up to 1.5x growth for natural variation, but not more
    assert!(
        growth_ratio < 1.5,
        "Voice count is accumulating! Grew from {:.2} to {:.2} ({:.2}x growth). This indicates voices are not being cleaned up properly.",
        early_avg, late_avg, growth_ratio
    );
}

#[test]
fn test_rms_does_not_grow_exponentially() {
    // Create a simple alternating pattern
    let input = r#"
        tempo: 1.0
        out: s("bd(<3 5>,8)")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let samples_per_cycle = (44100.0 / 1.0) as usize;
    let num_cycles = 8;

    let mut rms_per_cycle = Vec::new();

    for cycle in 0..num_cycles {
        let mut sum_squares = 0.0;

        for _ in 0..samples_per_cycle {
            let sample = graph.process_sample();
            sum_squares += sample * sample;
        }

        let rms = (sum_squares / samples_per_cycle as f32).sqrt();
        rms_per_cycle.push(rms);

        println!("Cycle {}: RMS={:.4} ({:.2} dB)",
                 cycle, rms, 20.0 * rms.max(0.0001).log10());
    }

    // Check if RMS is growing across cycles
    let early_rms = rms_per_cycle[2]; // Cycle 2 (after initial transient)
    let late_rms = rms_per_cycle[num_cycles - 1]; // Last cycle

    let rms_growth = late_rms / early_rms.max(0.0001);

    println!("\nRMS growth ratio (cycle 7 / cycle 2): {:.2}x", rms_growth);
    println!("Early RMS (cycle 2): {:.4}", early_rms);
    println!("Late RMS (cycle 7): {:.4}", late_rms);

    // RMS should be relatively stable
    // Allow some variation but not exponential growth
    assert!(
        rms_growth < 1.3,
        "RMS is growing exponentially! Grew from {:.4} to {:.4} ({:.2}x growth). This indicates voice accumulation.",
        early_rms, late_rms, rms_growth
    );
}

#[test]
fn test_simple_pattern_voice_stability() {
    // Even simpler: just "bd" repeated
    let input = r#"
        tempo: 2.0
        out: s("bd")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let samples_per_cycle = (44100.0 / 2.0) as usize; // 22050 samples at 2 CPS
    let num_cycles = 10;

    let mut max_voices_per_cycle = Vec::new();

    for cycle in 0..num_cycles {
        let mut max_voices = 0;

        for _ in 0..samples_per_cycle {
            let _sample = graph.process_sample();
            let voice_count = graph.active_voice_count();
            if voice_count > max_voices {
                max_voices = voice_count;
            }
        }

        max_voices_per_cycle.push(max_voices);
        println!("Cycle {}: max_voices={}", cycle, max_voices);
    }

    // For a simple repeating pattern, max voice count should stabilize at 1
    // (one voice per kick drum hit)
    let late_max = max_voices_per_cycle[num_cycles - 1];

    assert!(
        late_max <= 2,
        "Simple 'bd' pattern should use at most 1-2 voices, but cycle {} used {}",
        num_cycles - 1,
        late_max
    );
}
