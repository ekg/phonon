use phonon::simple_dsp_executor::SimpleDspExecutor;
use phonon::glicol_parser::parse_glicol;
use std::fs::File;
use std::io::Write;

fn main() {
    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);
    
    // Create a simple 4-beat pattern
    let code = r#"
        ~beat: sin 440 >> mul 0.5
        o: s "~beat ~beat ~beat ~beat"
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");
    
    println!("Generated {} samples", audio.data.len());
    println!("Peak: {:.3}", audio.peak());
    println!("RMS: {:.3}", audio.rms());
    
    // Save raw audio for inspection
    let mut file = File::create("/tmp/timing_test.raw").unwrap();
    for sample in &audio.data {
        file.write_all(&sample.to_le_bytes()).unwrap();
    }
    println!("Saved to /tmp/timing_test.raw");
    
    // Analyze quarters
    let quarter_size = audio.data.len() / 4;
    for q in 0..4 {
        let start = q * quarter_size;
        let end = (q + 1) * quarter_size;
        let quarter = &audio.data[start..end];
        
        let max = quarter.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
        let rms: f32 = (quarter.iter().map(|x| x * x).sum::<f32>() / quarter.len() as f32).sqrt();
        
        println!("Quarter {} ({:.3}s-{:.3}s): max={:.3}, rms={:.3}",
                 q,
                 start as f32 / sample_rate,
                 end as f32 / sample_rate,
                 max, rms);
        
        // Sample first 100 samples of each quarter
        if quarter.len() > 100 {
            let first_100_max = quarter[..100].iter().map(|x| x.abs()).fold(0.0f32, f32::max);
            println!("  First 100 samples max: {:.3}", first_100_max);
        }
    }
    
    // Look for actual audio onsets with better detection
    println!("\nLooking for onsets with smoothing:");
    let mut smoothed = vec![0.0f32; audio.data.len()];
    
    // Apply simple moving average
    let window = 100;
    for i in 0..audio.data.len() {
        let start = i.saturating_sub(window / 2);
        let end = (i + window / 2).min(audio.data.len());
        let sum: f32 = audio.data[start..end].iter().map(|x| x.abs()).sum();
        smoothed[i] = sum / (end - start) as f32;
    }
    
    // Find peaks in smoothed signal
    let mut peaks = Vec::new();
    let threshold = 0.01;
    let mut was_below = true;
    
    for (i, &val) in smoothed.iter().enumerate() {
        if val > threshold && was_below {
            peaks.push(i);
            was_below = false;
        } else if val < threshold * 0.5 {
            was_below = true;
        }
    }
    
    println!("Found {} peaks", peaks.len());
    for (i, &peak) in peaks.iter().enumerate() {
        println!("  Peak {}: sample {} ({:.3}s)", i, peak, peak as f32 / sample_rate);
    }
}