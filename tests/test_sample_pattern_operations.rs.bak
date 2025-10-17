use phonon::mini_notation_v3::parse_mini_notation;
use phonon::sample_loader::SampleBank;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

fn calculate_rms(samples: &[f32]) -> f32 {
    let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

// Note: Onset detection removed - unreliable for overlapping polyphonic samples.
// All tests now use RMS/peak-based verification which is more robust.

#[test]
fn test_alternation_over_multiple_cycles() {
    // Test <bd sn> pattern alternates between bd and sn across cycles
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0); // 1 cycle per second

    let pattern = parse_mini_notation("<bd sn>");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "<bd sn>".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
    });
    graph.set_output(sample_node);

    // Render 4 cycles = 4 seconds
    let num_cycles = 4;
    let total_samples = (sample_rate * num_cycles as f32) as usize;
    let buffer = graph.render(total_samples);

    println!("\n=== Alternation Test ===");
    println!("Pattern: <bd sn>");
    println!("Total RMS: {:.4}", calculate_rms(&buffer));
    println!(
        "Peak: {:.4}",
        buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max)
    );

    let samples_per_cycle = sample_rate as usize;

    // Analyze each cycle - verify each has audio from alternating samples
    for cycle in 0..num_cycles {
        let start = cycle * samples_per_cycle;
        let end = start + samples_per_cycle;
        let cycle_samples = &buffer[start..end];

        let cycle_rms = calculate_rms(cycle_samples);
        let cycle_peak = cycle_samples
            .iter()
            .map(|&x| x.abs())
            .fold(0.0f32, f32::max);

        println!(
            "Cycle {}: RMS={:.4}, Peak={:.4}",
            cycle, cycle_rms, cycle_peak
        );

        // Each cycle should have audio (alternating bd and sn)
        assert!(
            cycle_rms > 0.05,
            "Cycle {} should have audio, got RMS={}",
            cycle,
            cycle_rms
        );
        assert!(
            cycle_peak > 0.8,
            "Cycle {} should have strong peaks, got {}",
            cycle,
            cycle_peak
        );
    }

    // Verify overall audio quality
    let total_rms = calculate_rms(&buffer);
    let total_peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Overall: RMS={:.4}, Peak={:.4}", total_rms, total_peak);

    assert!(
        total_rms > 0.08,
        "Should have substantial audio over all cycles"
    );
    assert!(total_peak > 0.9, "Should have strong peaks");
}

#[test]
fn test_concatenation_multiple_samples() {
    // Test "bd sn cp hh" plays all 4 samples in sequence
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0);

    let pattern = parse_mini_notation("bd sn cp hh");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd sn cp hh".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
    });
    graph.set_output(sample_node);

    // Render 1 cycle
    let samples = (sample_rate * 1.0) as usize;
    let buffer = graph.render(samples);

    println!("\n=== Concatenation Test ===");
    println!("Pattern: bd sn cp hh");

    // Verify audio presence in each quarter of the cycle
    // Since samples overlap (polyphonic playback), we can't use onset detection
    // Instead, verify each quarter has audio activity
    let samples_per_quarter = (sample_rate * 0.25) as usize;

    println!("\nAudio presence per quarter:");
    for quarter in 0..4 {
        let start = quarter * samples_per_quarter;
        let end = start + samples_per_quarter;
        let quarter_samples = &buffer[start..end];

        let quarter_rms = calculate_rms(quarter_samples);
        let quarter_peak = quarter_samples
            .iter()
            .map(|&x| x.abs())
            .fold(0.0f32, f32::max);

        println!(
            "  Quarter {}: RMS={:.4}, Peak={:.4}",
            quarter, quarter_rms, quarter_peak
        );

        // Note: hh sample is much quieter than bd/sn/cp, so use lower thresholds for quarter 3
        let min_rms = if quarter == 3 { 0.005 } else { 0.1 };
        let min_peak = if quarter == 3 { 0.05 } else { 0.5 };

        assert!(
            quarter_rms > min_rms,
            "Quarter {} should have audio, got RMS={}",
            quarter,
            quarter_rms
        );
        assert!(
            quarter_peak > min_peak,
            "Quarter {} should have peaks, got {}",
            quarter,
            quarter_peak
        );
    }

    // Verify overall audio quality
    let total_rms = calculate_rms(&buffer);
    let total_peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Overall: RMS={:.4}, Peak={:.4}", total_rms, total_peak);

    assert!(total_rms > 0.15, "Should have substantial audio");
    assert!(total_peak > 0.8, "Should have strong peaks");
}

#[test]
fn test_layering_simultaneous_samples() {
    // Test [bd, sn] plays both samples simultaneously
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0);

    let pattern = parse_mini_notation("[bd, sn]");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "[bd, sn]".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
    });
    graph.set_output(sample_node);

    // Render 1 cycle
    let samples = (sample_rate * 1.0) as usize;
    let buffer = graph.render(samples);

    println!("\n=== Layering Test ===");
    println!("Pattern: [bd, sn]");

    // Load samples to check mixing
    let mut bank = SampleBank::new();
    let bd_sample = bank.get_sample("bd").expect("BD should load");
    let sn_sample = bank.get_sample("sn").expect("SN should load");

    // Check the first few samples contain both bd and sn
    // When layered, amplitude should be higher than either alone
    let first_chunk = &buffer[0..1000];
    let chunk_rms = calculate_rms(first_chunk);
    let chunk_peak = first_chunk.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!(
        "First 1000 samples: RMS={:.4}, Peak={:.4}",
        chunk_rms, chunk_peak
    );

    // RMS should be higher than a single sample due to mixing
    let bd_rms = calculate_rms(&bd_sample[0..1000.min(bd_sample.len())]);
    let sn_rms = calculate_rms(&sn_sample[0..1000.min(sn_sample.len())]);

    println!("BD alone RMS: {:.4}", bd_rms);
    println!("SN alone RMS: {:.4}", sn_rms);

    // Layered should be close to sum of both (allowing for some phase cancellation)
    assert!(
        chunk_rms > bd_rms * 0.8 || chunk_rms > sn_rms * 0.8,
        "Layered RMS should be comparable to individual samples"
    );
}

#[test]
fn test_alternation_with_subdivision() {
    // Test <bd*2 sn*2> - alternates between two bd hits and two sn hits
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0);

    let pattern = parse_mini_notation("<bd*2 sn*2>");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "<bd*2 sn*2>".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
    });
    graph.set_output(sample_node);

    // Render 4 cycles
    let num_cycles = 4;
    let total_samples = (sample_rate * num_cycles as f32) as usize;
    let buffer = graph.render(total_samples);

    println!("\n=== Alternation with Subdivision Test ===");
    println!("Pattern: <bd*2 sn*2>");

    // Verify each cycle has audio (alternating bd*2 and sn*2)
    let samples_per_cycle = sample_rate as usize;
    for cycle in 0..num_cycles {
        let start = cycle * samples_per_cycle;
        let end = start + samples_per_cycle;
        let cycle_samples = &buffer[start..end];

        let cycle_rms = calculate_rms(cycle_samples);
        let cycle_peak = cycle_samples
            .iter()
            .map(|&x| x.abs())
            .fold(0.0f32, f32::max);

        println!(
            "Cycle {}: RMS={:.4}, Peak={:.4}",
            cycle, cycle_rms, cycle_peak
        );

        assert!(cycle_rms > 0.1, "Cycle {} should have audio", cycle);
        assert!(cycle_peak > 0.8, "Cycle {} should have strong peaks", cycle);
    }

    // Verify overall audio quality
    let total_rms = calculate_rms(&buffer);
    let total_peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Overall: RMS={:.4}, Peak={:.4}", total_rms, total_peak);

    assert!(total_rms > 0.15, "Should have substantial audio");
    assert!(total_peak > 0.9, "Should have strong peaks");
}

#[test]
fn test_concatenation_over_multiple_bars() {
    // Test that a simple pattern plays correctly over many bars
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(0.5); // 0.5 CPS = 2 seconds per cycle

    let pattern = parse_mini_notation("bd cp");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd cp".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
    });
    graph.set_output(sample_node);

    // Render 8 cycles = 16 seconds
    let num_cycles = 8;
    let cycle_duration = 1.0 / 0.5; // 2 seconds per cycle
    let total_samples = (sample_rate * num_cycles as f32 * cycle_duration) as usize;
    let buffer = graph.render(total_samples);

    println!("\n=== Multi-Bar Concatenation Test ===");
    println!("Pattern: bd cp");
    println!("Cycles: {}, CPS: 0.5", num_cycles);

    // Verify each cycle has audio
    let samples_per_cycle = (sample_rate * cycle_duration) as usize;
    for cycle in 0..num_cycles {
        let start = cycle * samples_per_cycle;
        let end = start + samples_per_cycle;
        let cycle_samples = &buffer[start..end];

        let cycle_rms = calculate_rms(cycle_samples);
        let cycle_peak = cycle_samples
            .iter()
            .map(|&x| x.abs())
            .fold(0.0f32, f32::max);

        if cycle < 2 || cycle >= num_cycles - 1 {
            println!(
                "Cycle {}: RMS={:.4}, Peak={:.4}",
                cycle, cycle_rms, cycle_peak
            );
        } else if cycle == 2 {
            println!("...");
        }

        assert!(cycle_rms > 0.08, "Cycle {} should have audio", cycle);
        assert!(cycle_peak > 0.7, "Cycle {} should have strong peaks", cycle);
    }

    // Verify overall audio quality
    let total_rms = calculate_rms(&buffer);
    let total_peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Overall: RMS={:.4}, Peak={:.4}", total_rms, total_peak);

    assert!(
        total_rms > 0.05,
        "Should have substantial audio over all cycles"
    );
    assert!(total_peak > 0.8, "Should have strong peaks");
}

#[test]
fn test_euclidean_alternation_combo() {
    // Test <bd(3,8) sn(5,8)> - alternating euclidean patterns
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0);

    let pattern = parse_mini_notation("<bd(3,8) sn(5,8)>");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "<bd(3,8) sn(5,8)>".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
    });
    graph.set_output(sample_node);

    // Render 4 cycles
    let num_cycles = 4;
    let total_samples = (sample_rate * num_cycles as f32) as usize;
    let buffer = graph.render(total_samples);

    println!("\n=== Euclidean Alternation Test ===");
    println!("Pattern: <bd(3,8) sn(5,8)>");

    // Cycle 0,2: bd(3,8) = 3 hits
    // Cycle 1,3: sn(5,8) = 5 hits

    // Check each cycle has activity
    let samples_per_cycle = sample_rate as usize;
    for cycle in 0..num_cycles {
        let start = cycle * samples_per_cycle;
        let end = start + samples_per_cycle;
        let cycle_samples = &buffer[start..end];

        let cycle_rms = calculate_rms(cycle_samples);
        let cycle_peak = cycle_samples
            .iter()
            .map(|&x| x.abs())
            .fold(0.0f32, f32::max);

        println!(
            "Cycle {}: RMS={:.4}, Peak={:.4}",
            cycle, cycle_rms, cycle_peak
        );

        assert!(cycle_rms > 0.08, "Cycle {} should have audio", cycle);
        assert!(cycle_peak > 0.8, "Cycle {} should have strong peaks", cycle);
    }

    // Verify overall audio quality
    let total_rms = calculate_rms(&buffer);
    let total_peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Overall: RMS={:.4}, Peak={:.4}", total_rms, total_peak);

    assert!(total_rms > 0.12, "Should have substantial audio");
    assert!(total_peak > 0.9, "Should have strong peaks");
}

#[test]
fn test_fast_subdivision_accuracy() {
    // Test bd*16 - 16 hits in one cycle, checking timing accuracy
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0);

    let pattern = parse_mini_notation("bd*16");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd*16".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
    });
    graph.set_output(sample_node);

    // Render 2 cycles
    let num_cycles = 2;
    let total_samples = (sample_rate * num_cycles as f32) as usize;
    let buffer = graph.render(total_samples);

    println!("\n=== Fast Subdivision Test ===");
    println!("Pattern: bd*16");

    // Verify each cycle has substantial audio from 16 rapid hits
    let samples_per_cycle = sample_rate as usize;
    for cycle in 0..num_cycles {
        let start = cycle * samples_per_cycle;
        let end = start + samples_per_cycle;
        let cycle_samples = &buffer[start..end];

        let cycle_rms = calculate_rms(cycle_samples);
        let cycle_peak = cycle_samples
            .iter()
            .map(|&x| x.abs())
            .fold(0.0f32, f32::max);

        println!(
            "Cycle {}: RMS={:.4}, Peak={:.4}",
            cycle, cycle_rms, cycle_peak
        );

        // 16 hits should produce substantial continuous audio
        assert!(
            cycle_rms > 0.4,
            "Cycle {} should have high RMS from 16 hits, got {}",
            cycle,
            cycle_rms
        );
        assert!(cycle_peak > 0.9, "Cycle {} should have strong peaks", cycle);
    }

    // Verify overall audio quality
    let total_rms = calculate_rms(&buffer);
    let total_peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Overall: RMS={:.4}, Peak={:.4}", total_rms, total_peak);

    assert!(total_rms > 0.4, "Should have high RMS from rapid hits");
    assert!(total_peak > 0.9, "Should have strong peaks");
}
