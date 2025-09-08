use phonon::simple_dsp_executor::SimpleDspExecutor;
use phonon::glicol_parser::parse_glicol;

fn main() {
    println!("=== Debugging Sine Wave Generation ===\n");
    
    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);
    
    // Simple test: one click per beat
    let code = r#"
        ~click: sin 1000 >> mul 0.5
        o: s "~click ~click ~click ~click"
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");
    
    // Analyze the waveform
    println!("Looking for sine wave continuity...");
    
    // Expected: 4 beats at 0, 0.25, 0.5, 0.75 seconds
    // At 1000 Hz, we should see 1000 cycles per second
    // So in 0.25 seconds (one beat duration), we should see 250 cycles
    
    // Check the first beat region (0 to 0.25s)
    let beat1_start = 0;
    let beat1_end = (0.25 * sample_rate) as usize;
    
    // Count zero crossings in first beat
    let mut zero_crossings = 0;
    let mut last_sample = 0.0;
    for i in beat1_start..beat1_end.min(audio.data.len()) {
        let sample = audio.data[i];
        if last_sample <= 0.0 && sample > 0.0 {
            zero_crossings += 1;
        }
        last_sample = sample;
    }
    
    println!("Beat 1 (0-0.25s):");
    println!("  Zero crossings: {} (expected ~250 for 1000Hz)", zero_crossings);
    
    // Sample some actual values to see the waveform
    println!("\nFirst 100 samples:");
    for i in 0..100.min(audio.data.len()) {
        if i % 10 == 0 {
            println!("  Sample {}: {:.4}", i, audio.data[i]);
        }
    }
    
    // Check if we're generating continuous sine or resetting
    println!("\nChecking phase continuity at beat boundaries:");
    
    for beat in 0..4 {
        let beat_start = ((beat as f32 * 0.25) * sample_rate) as usize;
        let beat_end = beat_start + 10; // Check first 10 samples of each beat
        
        println!("\nBeat {} (starting at sample {}):", beat, beat_start);
        for i in beat_start..beat_end.min(audio.data.len()) {
            let time = i as f32 / sample_rate;
            let expected_phase = time * 1000.0 * 2.0 * std::f32::consts::PI;
            let expected_value = expected_phase.sin();
            let actual_value = audio.data[i];
            
            if (i - beat_start) < 3 {
                println!("  Sample {}: actual={:.4}, expected={:.4}, diff={:.4}", 
                         i, actual_value, expected_value, (actual_value - expected_value).abs());
            }
        }
    }
}