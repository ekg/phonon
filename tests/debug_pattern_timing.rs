use phonon::mini_notation_v3::parse_mini_notation;
/// Debug test to understand pattern timing issues
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
use std::cell::RefCell;

#[test]
fn debug_pattern_value_changes() {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles per second

    // Create pattern: 220, 330, 440, 330 Hz
    let pattern = parse_mini_notation("220 330 440 330");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "220 330 440 330".to_string(),
        pattern,
        last_value: 220.0,
        last_trigger_time: -1.0,
    });

    // Use pattern to control oscillator frequency
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(pattern_node),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    graph.set_output(osc);

    println!("\n=== Pattern Timing Debug ===");
    println!("CPS: 2.0, Cycle duration: 0.5 seconds, 22050 samples");
    println!("Pattern: 220 330 440 330");
    println!("Expected transitions:");
    println!("  Sample 0: 220 Hz");
    println!("  Sample 5512: 330 Hz");
    println!("  Sample 11025: 440 Hz");
    println!("  Sample 16537: 330 Hz");
    println!();

    let mut last_value = 0.0;
    let samples_to_check = 22050; // One full cycle

    for i in 0..samples_to_check {
        let value = graph.process_sample();

        // Sample every 100 samples to see value changes
        if i % 1000 == 0 {
            if (value - last_value).abs() > 0.01 {
                println!("Sample {}: value changed {} -> {}", i, last_value, value);
                last_value = value;
            }
        }
    }

    println!("\n=== Checking specific transition points ===");
    let mut graph2 = UnifiedSignalGraph::new(44100.0);
    graph2.set_cps(2.0);

    let pattern2 = parse_mini_notation("220 330 440 330");
    let pattern_node2 = graph2.add_node(SignalNode::Pattern {
        pattern_str: "220 330 440 330".to_string(),
        pattern: pattern2,
        last_value: 220.0,
        last_trigger_time: -1.0,
    });

    let osc2 = graph2.add_node(SignalNode::Oscillator {
        freq: Signal::Node(pattern_node2),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    graph2.set_output(osc2);

    // Check specific sample ranges
    let check_points = [0, 5512, 11025, 16537, 22000];
    for &sample_num in &check_points {
        // Process to that sample
        let mut g = UnifiedSignalGraph::new(44100.0);
        g.set_cps(2.0);
        let p = parse_mini_notation("220 330 440 330");
        let pn = g.add_node(SignalNode::Pattern {
            pattern_str: "220 330 440 330".to_string(),
            pattern: p,
            last_value: 220.0,
            last_trigger_time: -1.0,
        });
        let o = g.add_node(SignalNode::Oscillator {
            freq: Signal::Node(pn),
            waveform: Waveform::Sine,
            semitone_offset: 0.0,
            phase: RefCell::new(0.0),
            pending_freq: RefCell::new(None),
            last_sample: RefCell::new(0.0),
        });
        g.set_output(o);

        // Process to the target sample
        for _ in 0..sample_num {
            g.process_sample();
        }

        // Check a few samples around this point
        let mut samples = Vec::new();
        for _ in 0..100 {
            samples.push(g.process_sample());
        }

        // Estimate frequency using zero crossings
        let freq = estimate_frequency(&samples, 44100);
        let time_secs = sample_num as f32 / 44100.0;
        let cycle_pos = time_secs * 2.0; // 2 cps

        println!(
            "Sample {}: time={:.4}s, cycle_pos={:.3}, freq={:.1} Hz",
            sample_num, time_secs, cycle_pos, freq
        );
    }
}

fn estimate_frequency(samples: &[f32], sample_rate: u32) -> f32 {
    let mut crossings = 0;
    let mut last_sign = samples[0] >= 0.0;

    for &sample in &samples[1..] {
        let sign = sample >= 0.0;
        if sign != last_sign {
            crossings += 1;
            last_sign = sign;
        }
    }

    let duration = samples.len() as f32 / sample_rate as f32;
    (crossings as f32 / 2.0) / duration
}
