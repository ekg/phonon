use phonon::simple_dsp_executor::SimpleDspExecutor;
use phonon::glicol_parser::parse_glicol;

fn main() {
    println!("=== Debugging Sample Positions ===\n");
    
    let sample_rate = 44100.0;
    
    // Calculate where each event SHOULD be
    println!("Expected sample positions for 4 beats in 1 second:");
    for i in 0..4 {
        let start_time = i as f64 * 0.25;
        let end_time = (i + 1) as f64 * 0.25;
        let start_sample = (start_time * sample_rate as f64) as usize;
        let end_sample = (end_time * sample_rate as f64) as usize;
        
        println!("  Beat {}: {:.3}s-{:.3}s = samples {}-{}", 
                 i, start_time, end_time, start_sample, end_sample);
    }
    
    println!("\nNow let's render and see what we get:");
    
    let mut executor = SimpleDspExecutor::new(sample_rate);
    
    let code = r#"
        ~click: sin 1000 >> mul 0.5
        o: s "~click ~click ~click ~click"
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");
    
    // Find where we have non-zero audio
    let mut regions = Vec::new();
    let mut in_region = false;
    let mut region_start = 0;
    
    for (i, &sample) in audio.data.iter().enumerate() {
        if sample.abs() > 0.001 && !in_region {
            in_region = true;
            region_start = i;
        } else if sample.abs() < 0.001 && in_region {
            in_region = false;
            regions.push((region_start, i));
        }
    }
    if in_region {
        regions.push((region_start, audio.data.len()));
    }
    
    println!("Found {} regions with audio:", regions.len());
    for (i, (start, end)) in regions.iter().enumerate() {
        let start_time = *start as f32 / sample_rate;
        let end_time = *end as f32 / sample_rate;
        println!("  Region {}: samples {}-{} ({:.3}s-{:.3}s)", 
                 i, start, end, start_time, end_time);
    }
    
    // Check if the issue is in the envelope duration
    println!("\nChecking envelope timing:");
    let attack_samples = (0.001 * sample_rate) as usize;
    let decay_samples = (0.1 * sample_rate) as usize;
    println!("  Attack: {} samples ({:.3}s)", attack_samples, attack_samples as f32 / sample_rate);
    println!("  Decay: {} samples ({:.3}s)", decay_samples, decay_samples as f32 / sample_rate);
    println!("  Total envelope: {} samples ({:.3}s)", 
             attack_samples + decay_samples,
             (attack_samples + decay_samples) as f32 / sample_rate);
    
    // The issue might be that we're only rendering the envelope duration,
    // not the full event duration!
}