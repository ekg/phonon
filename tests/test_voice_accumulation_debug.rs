/// Debug test to trace event triggering across cycle boundaries
///
/// This test renders a pattern with alternation and logs detailed information
/// about event triggering to help diagnose if events are being re-triggered.
use phonon::unified_graph::{SignalNode, UnifiedSignalGraph};
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_debug_event_triggering_alternation() {
    // Simple alternating pattern
    let input = r#"
        tempo: 2.0
        out: s "bd(<3 5>,8)"
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Sample for 2 complete cycles, tracking voice counts at key points
    let samples_per_cycle = (44100.0 / 2.0) as usize; // 22050 samples at 2 CPS
    let num_cycles = 2;

    println!("\n=== DEBUGGING EVENT TRIGGERING ACROSS CYCLES ===");
    println!("Pattern: bd(<3 5>,8)");
    println!("Tempo: 2.0 CPS");
    println!("Samples per cycle: {}\n", samples_per_cycle);

    for cycle in 0..num_cycles {
        let cycle_start_sample = cycle * samples_per_cycle;
        let cycle_end_sample = (cycle + 1) * samples_per_cycle;

        println!(
            "--- CYCLE {} (samples {} to {}) ---",
            cycle, cycle_start_sample, cycle_end_sample
        );

        // Track voice counts throughout the cycle
        let mut voice_counts_at_samples = Vec::new();
        let checkpoint_interval = samples_per_cycle / 20; // 20 checkpoints per cycle

        for sample_idx in cycle_start_sample..cycle_end_sample {
            let _sample = graph.process_sample();
            let voice_count = graph.active_voice_count();

            // Log voice count at checkpoints
            if (sample_idx - cycle_start_sample) % checkpoint_interval == 0 {
                let relative_sample = sample_idx - cycle_start_sample;
                let cycle_position = relative_sample as f64 / samples_per_cycle as f64;
                voice_counts_at_samples.push((cycle_position, voice_count));
            }
        }

        // Print checkpoint data
        for (pos, count) in voice_counts_at_samples {
            println!("  cycle_pos={:.3} → voices={}", pos, count);
        }

        // Get stats for this cycle
        let cycle_end_voices = graph.active_voice_count();
        println!(
            "  END OF CYCLE {}: active_voices={}\n",
            cycle, cycle_end_voices
        );
    }

    // Now check one more cycle to see if pattern continues
    println!("--- CYCLE 2 (verification cycle) ---");
    let mut max_voices_cycle2 = 0;
    for _ in 0..samples_per_cycle {
        let _sample = graph.process_sample();
        let voice_count = graph.active_voice_count();
        if voice_count > max_voices_cycle2 {
            max_voices_cycle2 = voice_count;
        }
    }
    println!("  MAX VOICES in cycle 2: {}\n", max_voices_cycle2);

    // The pattern should stabilize
    // Cycle 0: 3 hits (pattern <3 5> starts with 3)
    // Cycle 1: 5 hits (alternates to 5)
    // Cycle 2: 3 hits (back to 3)

    println!("=== ANALYSIS ===");
    println!("If voices are accumulating, max_voices should keep growing.");
    println!("If voices are properly cleaned up, max_voices should oscillate but not grow.");
    println!("\nExpected behavior:");
    println!("  - Cycle 0 (3 hits): lower peak voices");
    println!("  - Cycle 1 (5 hits): higher peak voices");
    println!("  - Cycle 2 (3 hits): should match cycle 0, NOT be higher");
}

#[test]
fn test_compare_alternating_vs_constant() {
    println!("\n=== COMPARING ALTERNATING VS CONSTANT PATTERNS ===\n");

    // Test 1: Alternating pattern
    let input_alternating = r#"
        tempo: 2.0
        out: s "bd(<3 5>,8)"
    "#;

    let (_, statements) = parse_dsl(input_alternating).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_alt = compiler.compile(statements);

    let samples_per_cycle = (44100.0 / 2.0) as usize;
    let num_cycles = 4;

    println!("ALTERNATING PATTERN: bd(<3 5>,8)");
    let mut alt_peaks = Vec::new();
    for cycle in 0..num_cycles {
        let mut max_voices = 0;
        for _ in 0..samples_per_cycle {
            let _sample = graph_alt.process_sample();
            let voice_count = graph_alt.active_voice_count();
            if voice_count > max_voices {
                max_voices = voice_count;
            }
        }
        alt_peaks.push(max_voices);
        println!("  Cycle {}: peak_voices={}", cycle, max_voices);
    }

    // Test 2: Constant pattern (4 hits per cycle, no alternation)
    let input_constant = r#"
        tempo: 2.0
        out: s "bd(4,8)"
    "#;

    let (_, statements) = parse_dsl(input_constant).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_const = compiler.compile(statements);

    println!("\nCONSTANT PATTERN: bd(4,8)");
    let mut const_peaks = Vec::new();
    for cycle in 0..num_cycles {
        let mut max_voices = 0;
        for _ in 0..samples_per_cycle {
            let _sample = graph_const.process_sample();
            let voice_count = graph_const.active_voice_count();
            if voice_count > max_voices {
                max_voices = voice_count;
            }
        }
        const_peaks.push(max_voices);
        println!("  Cycle {}: peak_voices={}", cycle, max_voices);
    }

    println!("\n=== ANALYSIS ===");
    println!("Alternating pattern peaks: {:?}", alt_peaks);
    println!("Constant pattern peaks: {:?}", const_peaks);

    // Check if constant pattern has stable voice count
    let const_stable = const_peaks.iter().skip(1).all(|&p| p == const_peaks[1]);
    println!("\nConstant pattern stable after cycle 0: {}", const_stable);

    // Check if alternating pattern oscillates or grows
    let alt_growing = alt_peaks[3] as f32 > alt_peaks[1] as f32 * 1.2;
    println!("Alternating pattern growing: {}", alt_growing);
}
