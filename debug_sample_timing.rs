fn main() {
    let sample_rate = 44100.0;
    let cps_values = vec![0.5, 2.0];

    for cps in cps_values {
        let sample_width = 1.0 / sample_rate as f64 / cps as f64;
        println!("\nCPS: {}, Sample width: {:.10} cycles", cps, sample_width);
        println!("Samples per cycle: {:.0}", 1.0 / sample_width);

        // For hh*4, events are at 0.0, 0.25, 0.5, 0.75
        let event_positions = vec![0.0, 0.25, 0.5, 0.75];
        println!("\nEvent positions for hh*4:");
        for pos in event_positions {
            let samples_to_event = pos / sample_width;
            println!("  Event at {}: sample {:.0}", pos, samples_to_event);
        }
    }
}
