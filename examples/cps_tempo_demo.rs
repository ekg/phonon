use phonon::simple_dsp_executor::SimpleDspExecutor;
use phonon::glicol_parser::parse_glicol;

fn main() {
    println!("=== CPS (Cycles Per Second) and BPM Demo ===\n");
    
    let sample_rate = 44100.0;
    
    // Simple 4-beat pattern
    let code = r#"
        ~kick: sin 60 >> mul 0.5
        o: s "~kick ~kick ~kick ~kick"
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    
    println!("UNDERSTANDING CPS vs BPM:");
    println!("{}", "-".repeat(40));
    println!("CPS = Cycles Per Second");
    println!("BPM = Beats Per Minute\n");
    
    println!("If your pattern has 4 beats per cycle:");
    println!("  BPM = CPS * 4 * 60");
    println!("  CPS = BPM / (4 * 60) = BPM / 240\n");
    
    println!("Common tempos:");
    println!("{}", "-".repeat(40));
    
    // Test different BPMs
    let bpms = vec![
        (60.0, "Very slow"),
        (90.0, "Slow ballad"),
        (120.0, "Standard/Moderate"),
        (128.0, "House/Techno"),
        (140.0, "Dubstep/Trap"),
        (160.0, "Juke/Footwork"),
        (174.0, "Drum & Bass"),
    ];
    
    for (bpm, description) in bpms {
        // Calculate CPS for 4/4 time (4 beats per cycle)
        let cps = bpm / 240.0;
        
        println!("\n{} BPM ({}):", bpm, description);
        println!("  CPS = {} / 240 = {:.3}", bpm, cps);
        println!("  One cycle duration: {:.3} seconds", 1.0 / cps);
        println!("  Beat duration: {:.3} seconds", 1.0 / (cps * 4.0));
        
        // Create executor with this CPS
        let mut executor = SimpleDspExecutor::new(sample_rate);
        executor.set_cps(cps);
        
        // Render 2 seconds to see how many cycles we get
        let duration = 2.0;
        let cycles = duration * cps;
        let beats = cycles * 4.0;
        
        println!("  In {} seconds: {:.1} cycles, {:.0} beats", duration, cycles, beats);
    }
    
    println!();
    println!("CURRENT API (Programmatic):");
    println!("{}", "-".repeat(40));
    println!("```rust");
    println!("let mut executor = SimpleDspExecutor::new(sample_rate);");
    println!("executor.set_cps(0.5);  // 120 BPM for 4/4");
    println!("```\n");
    
    println!("PROPOSED DSL SYNTAX OPTIONS:");
    println!("{}", "-".repeat(40));
    
    println!("Option 1: Global directive");
    println!("```");
    println!("cps: 0.5");
    println!("~kick: sin 60 >> mul 0.5");
    println!("o: s \"~kick*4\"");
    println!("```\n");
    
    println!("Option 2: BPM directive (more intuitive)");
    println!("```");
    println!("bpm: 120");
    println!("~kick: sin 60 >> mul 0.5");
    println!("o: s \"~kick*4\"");
    println!("```\n");
    
    println!("Option 3: Inline with pattern");
    println!("```");
    println!("o: s \"~kick*4\" >> cps 0.5");
    println!("```\n");
    
    println!("Option 4: As a pattern transformation");
    println!("```");
    println!("o: s \"~kick*4\" >> tempo 120");
    println!("```\n");
    
    println!("CALCULATING CPS FOR YOUR TEMPO:");
    println!("{}", "-".repeat(40));
    
    // Helper calculation
    let target_bpm = 120.0;
    let beats_per_cycle = 4.0; // assuming 4/4
    let target_cps = target_bpm / (beats_per_cycle * 60.0);
    
    println!("For {} BPM with {} beats per cycle:", target_bpm, beats_per_cycle);
    println!("  CPS = {} / ({} * 60) = {:.3}", target_bpm, beats_per_cycle, target_cps);
    println!("\nSo for 120 BPM in 4/4 time:");
    println!("  executor.set_cps({:.1});", target_cps);
    
    // Alternative calculation
    println!("\nAlternatively, if you think in cycles per minute:");
    let cpm = 30.0; // 30 cycles per minute
    let cps_from_cpm = cpm / 60.0;
    println!("  {} cycles/minute = {:.3} cycles/second", cpm, cps_from_cpm);
    
    println!("\n=== Quick Reference ===\n");
    println!("120 BPM = 0.5 CPS (for 4/4 time)");
    println!("Formula: CPS = BPM / 240 (for 4 beats/cycle)");
    println!("         CPS = BPM / (beats_per_cycle * 60)");
}