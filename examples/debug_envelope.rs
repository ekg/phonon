use phonon::simple_dsp_executor::SimpleDspExecutor;
use phonon::glicol_parser::parse_glicol;

fn main() {
    println!("=== Debugging Envelope Generation ===\n");
    
    let sample_rate = 44100.0;
    
    // Simulate what happens in render_sample_pattern
    let start_sample = 0;
    let end_sample = (0.25 * sample_rate) as usize; // One beat duration
    
    let event_duration_samples = end_sample - start_sample;
    let attack_samples = (0.001 * sample_rate) as usize; // 1ms attack
    let decay_samples = event_duration_samples.saturating_sub(attack_samples);
    
    println!("Event duration: {} samples ({:.3}s)", event_duration_samples, event_duration_samples as f32 / sample_rate);
    println!("Attack: {} samples ({:.3}s)", attack_samples, attack_samples as f32 / sample_rate);
    println!("Decay: {} samples ({:.3}s)", decay_samples, decay_samples as f32 / sample_rate);
    println!();
    
    // Calculate envelope at various points
    let test_points = vec![0, 10, 20, 44, 100, 1000, 5000, 10000, 11024];
    
    println!("Envelope values at different sample offsets:");
    for &sample_offset in &test_points {
        if sample_offset >= event_duration_samples {
            break;
        }
        
        let env = if sample_offset < attack_samples {
            sample_offset as f32 / attack_samples as f32
        } else if sample_offset < event_duration_samples {
            let decay_progress = (sample_offset - attack_samples) as f32 / decay_samples.max(1) as f32;
            1.0 * (-5.0 * decay_progress).exp()
        } else {
            0.0
        };
        
        let time = sample_offset as f32 / sample_rate;
        println!("  Offset {:5} ({:.4}s): envelope = {:.6}", sample_offset, time, env);
        
        // Also show what the decay progress is
        if sample_offset >= attack_samples && sample_offset < event_duration_samples {
            let decay_progress = (sample_offset - attack_samples) as f32 / decay_samples.max(1) as f32;
            println!("    (decay_progress = {:.4}, exp({:.4}) = {:.6})", 
                     decay_progress, -5.0 * decay_progress, (-5.0 * decay_progress).exp());
        }
    }
    
    println!("\nNow let's test the actual rendering:");
    
    let mut executor = SimpleDspExecutor::new(sample_rate);
    
    let code = r#"
        ~click: sin 1000 >> mul 0.5
        o: s "~click ~click ~click ~click"
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");
    
    // Find maximum amplitude in each beat
    for beat in 0..4 {
        let beat_start = ((beat as f32 * 0.25) * sample_rate) as usize;
        let beat_end = ((beat as f32 * 0.25 + 0.25) * sample_rate) as usize;
        
        let mut max_amp = 0.0f32;
        for i in beat_start..beat_end.min(audio.data.len()) {
            max_amp = max_amp.max(audio.data[i].abs());
        }
        
        println!("Beat {}: max amplitude = {:.6}", beat, max_amp);
    }
}