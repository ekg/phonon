use phonon::simple_dsp_executor::SimpleDspExecutor;
use phonon::glicol_parser::parse_glicol;

fn main() {
    println!("=== Testing Last Beat Duration ===\n");
    
    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);
    
    // Simple 4-beat pattern
    let code = r#"
        ~click: sin 1000 >> mul 0.5
        o: s "~click ~click ~click ~click"
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");
    
    println!("Total samples generated: {}", audio.data.len());
    println!("Expected: {} (1 second at {}Hz)", sample_rate as usize, sample_rate);
    
    // Check the last 25% of the buffer
    let last_quarter_start = (audio.data.len() * 3) / 4;
    let last_quarter = &audio.data[last_quarter_start..];
    
    // Find where audio stops in the last quarter
    let mut last_nonzero = 0;
    for (i, &sample) in last_quarter.iter().enumerate() {
        if sample.abs() > 0.001 {
            last_nonzero = i;
        }
    }
    
    let last_audio_sample = last_quarter_start + last_nonzero;
    let last_audio_time = last_audio_sample as f32 / sample_rate;
    
    println!("\nLast quarter analysis:");
    println!("  Starts at sample {} ({:.3}s)", last_quarter_start, last_quarter_start as f32 / sample_rate);
    println!("  Last non-zero audio at sample {} ({:.3}s)", last_audio_sample, last_audio_time);
    println!("  Expected to continue until sample {} (1.000s)", audio.data.len() - 1);
    
    if last_audio_time < 0.95 {
        println!("\n❌ PROBLEM: Audio stops at {:.3}s, should continue to 1.0s!", last_audio_time);
    } else {
        println!("\n✓ Audio continues through the full cycle");
    }
    
    // Check each beat's actual duration
    println!("\nExpected beat durations:");
    for i in 0..4 {
        let start = (i as f32 * 0.25 * sample_rate) as usize;
        let end = ((i + 1) as f32 * 0.25 * sample_rate) as usize;
        println!("  Beat {}: samples {}-{} (duration: {} samples)", i, start, end, end - start);
    }
}