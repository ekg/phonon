use phonon::simple_dsp_executor::SimpleDspExecutor;
use phonon::glicol_parser::parse_glicol;

fn main() {
    println!("=== CPS and Pattern Modifier Timing Test ===\n");
    
    let sample_rate = 44100.0;
    
    // Simple pattern for testing
    let code = r#"
        ~click: sin 1000 >> mul 0.5
        o: s "~click ~click ~click ~click"
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    
    // Test different CPS values
    for &cps in &[0.5, 1.0, 2.0] {
        println!("Testing CPS = {}", cps);
        println!("{}", "-".repeat(40));
        
        let mut executor = SimpleDspExecutor::new(sample_rate);
        executor.set_cps(cps);
        
        let duration = 1.0; // 1 second
        let audio = executor.render(&env, duration).expect("Failed to render");
        
        // Find beat onsets (where audio starts after silence)
        let mut beat_positions = Vec::new();
        for i in 1..audio.data.len() {
            if audio.data[i-1].abs() < 0.001 && audio.data[i].abs() > 0.001 && audio.data[i+1].abs() > 0.001 {
                beat_positions.push(i);
            }
        }
        
        // Also check position 0
        if audio.data[0].abs() < 0.001 && audio.data[1].abs() > 0.001 {
            beat_positions.insert(0, 0);
        }
        
        let cycle_duration = 1.0 / cps;
        let expected_beats_per_second = 4.0 * cps; // 4 beats per cycle
        let expected_beats = (duration * expected_beats_per_second) as usize;
        
        println!("  Cycle duration: {:.3}s", cycle_duration);
        println!("  Expected beats in {}s: {}", duration, expected_beats);
        println!("  Found {} beat onsets", beat_positions.len());
        
        if beat_positions.len() >= expected_beats {
            println!("  First {} beat positions:", expected_beats.min(8));
            for i in 0..expected_beats.min(8) {
                let sample = beat_positions[i];
                let time = sample as f32 / sample_rate;
                let expected_time = i as f32 / expected_beats_per_second;
                let error = (time - expected_time) * 1000.0;
                println!("    Beat {}: sample {} ({:.3}s), expected {:.3}s, error: {:.1}ms",
                         i, sample, time, expected_time, error);
            }
        }
        println!();
    }
    
    // Test pattern modifiers
    println!("Testing Pattern Modifiers");
    println!("{}", "=".repeat(40));
    
    let test_patterns = vec![
        ("bd bd bd bd", "Normal pattern", 4),
        ("bd*4", "Single element repeated 4 times", 4),
        ("[bd bd]*2", "Fast sequence repeated", 4),
        ("bd bd*2", "Second element doubled", 3),
        ("bd/2 bd", "First element slowed", 2), // bd/2 spans 2 cycles
    ];
    
    let mut executor = SimpleDspExecutor::new(sample_rate);
    executor.set_cps(1.0);
    
    for (pattern, description, expected_events_per_cycle) in test_patterns {
        println!("\nPattern: \"{}\"", pattern);
        println!("  Description: {}", description);
        
        // Create test code with the pattern
        let test_code = format!(r#"
            ~bd: sin 200 >> mul 0.5
            o: s "{}"
        "#, pattern);
        
        let env = parse_glicol(&test_code).expect("Failed to parse");
        let audio = executor.render(&env, 1.0).expect("Failed to render");
        
        // Count distinct sound regions
        let mut sound_regions = 0;
        let mut in_sound = false;
        
        for sample in audio.data.iter() {
            if !in_sound && sample.abs() > 0.01 {
                sound_regions += 1;
                in_sound = true;
            } else if in_sound && sample.abs() < 0.001 {
                in_sound = false;
            }
        }
        
        println!("  Expected events in 1 cycle: {}", expected_events_per_cycle);
        println!("  Sound regions detected: {}", sound_regions);
        
        // For timing analysis, find the first few onsets
        let mut onsets = Vec::new();
        for i in 1..audio.data.len().min(22050) { // Check first 0.5 seconds
            if audio.data[i-1].abs() < 0.001 && audio.data[i].abs() > 0.001 {
                onsets.push(i);
                if onsets.len() >= 4 {
                    break;
                }
            }
        }
        
        if !onsets.is_empty() {
            print!("  First {} onset times: ", onsets.len());
            for onset in &onsets {
                print!("{:.3}s ", *onset as f32 / sample_rate);
            }
            println!();
        }
    }
    
    println!("\n=== Summary ===");
    println!("CPS (Cycles Per Second) controls the tempo:");
    println!("  - CPS = 0.5: Half speed (120 BPM if 4/4)");
    println!("  - CPS = 1.0: Normal speed (240 BPM if 4/4)");
    println!("  - CPS = 2.0: Double speed (480 BPM if 4/4)");
    println!("\nPattern modifiers transform patterns:");
    println!("  - Can be chained: \"bd*2/3\" means repeat twice then slow by 3");
    println!("  - Work with groups: \"[bd sn]*2\" repeats the whole group");
}