use phonon::simple_dsp_executor::SimpleDspExecutor;
use phonon::glicol_parser::parse_glicol;

fn main() {
    println!("=== Verifying Beat Timing (2 Cycles) ===\n");
    
    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);
    
    // Use a lower frequency click so beats are more distinct
    let code = r#"
        ~click: sin 200 >> mul 0.5
        o: s "~click ~click ~click ~click"
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 2.0).expect("Failed to render");
    
    // Find beat boundaries by looking for envelope resets
    // Each beat starts with a ramp up from 0 due to the attack envelope
    let mut beat_starts = Vec::new();
    
    for i in 1..audio.data.len() {
        // Look for transitions from very low to rising amplitude
        // This indicates the start of the attack envelope
        if i > 0 && audio.data[i-1].abs() < 0.001 && audio.data[i].abs() > 0.001 {
            beat_starts.push(i);
        }
    }
    
    // Also check sample 0 explicitly
    if audio.data[0].abs() < 0.001 && audio.data[1].abs() > 0.001 {
        beat_starts.insert(0, 0);
    }
    
    println!("Found {} beat starts in 2 seconds", beat_starts.len());
    println!("Expected: 8 (4 beats × 2 cycles)\n");
    
    // Verify beat positions
    println!("Beat | Expected Sample | Actual Sample | Time | Error");
    println!("-----|-----------------|---------------|------|-------");
    
    let mut all_correct = true;
    for beat in 0..8 {
        let expected_sample = ((beat as f32 * 0.25) * sample_rate) as usize;
        let expected_time = beat as f32 * 0.25;
        
        if beat < beat_starts.len() {
            let actual_sample = beat_starts[beat];
            let actual_time = actual_sample as f32 / sample_rate;
            let error_samples = (actual_sample as i32 - expected_sample as i32).abs();
            let error_ms = ((actual_time - expected_time) * 1000.0).abs();
            
            let status = if error_samples <= 2 { "✓" } else { "✗" };
            println!("{:4} | {:15} | {:13} | {:.3}s | {}{}",
                     beat, expected_sample, actual_sample, actual_time, 
                     if error_samples <= 2 { " " } else { "!" },
                     status);
            
            if error_samples > 2 {
                all_correct = false;
            }
        } else {
            println!("{:4} | {:15} | MISSING       | -     | !✗", beat, expected_sample);
            all_correct = false;
        }
    }
    
    // Check spacing consistency
    if beat_starts.len() >= 2 {
        println!("\nBeat Spacing Analysis:");
        let mut spacings = Vec::new();
        for i in 1..beat_starts.len() {
            spacings.push(beat_starts[i] - beat_starts[i-1]);
        }
        
        let expected_spacing = (0.25 * sample_rate) as usize;
        let min_spacing = *spacings.iter().min().unwrap();
        let max_spacing = *spacings.iter().max().unwrap();
        let avg_spacing = spacings.iter().sum::<usize>() / spacings.len();
        
        println!("Expected spacing: {} samples", expected_spacing);
        println!("Actual spacing:   min={}, max={}, avg={}", min_spacing, max_spacing, avg_spacing);
        
        let variance = max_spacing - min_spacing;
        if variance <= 2 {
            println!("✓ Spacing is consistent (variance: {} samples)", variance);
        } else {
            println!("✗ Spacing is inconsistent (variance: {} samples)", variance);
            all_correct = false;
        }
    }
    
    // Final verdict
    println!("\n=== VERDICT ===");
    if all_correct && beat_starts.len() == 8 {
        println!("✓✓✓ SUCCESS: All 8 beats are evenly spaced across 2 cycles!");
        println!("The pattern timing is working correctly.");
    } else {
        println!("✗✗✗ FAILURE: Beat timing issues detected");
        if beat_starts.len() != 8 {
            println!("- Wrong number of beats: {} instead of 8", beat_starts.len());
        }
        if !all_correct {
            println!("- Some beats are not at the expected positions");
        }
    }
}