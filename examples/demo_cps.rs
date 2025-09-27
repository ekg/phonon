use phonon::glicol_parser::parse_glicol;
use phonon::simple_dsp_executor::SimpleDspExecutor;

fn main() {
    println!("=== CPS (Cycles Per Second) Demo ===\n");

    let sample_rate = 44100.0;

    // Simple beat pattern
    let code = r#"
        ~kick: sin 60 >> mul 0.5
        o: s "~kick ~kick ~kick ~kick"
    "#;

    let env = parse_glicol(code).expect("Failed to parse");

    println!("Pattern: \"~kick ~kick ~kick ~kick\" (4 beats per cycle)\n");

    // Test different CPS values
    for &cps in &[0.5, 1.0, 2.0] {
        let mut executor = SimpleDspExecutor::new(sample_rate);
        executor.set_cps(cps);

        let duration = 2.0; // 2 seconds
        let cycles = duration * cps;
        let beats = cycles * 4.0; // 4 beats per cycle

        println!("CPS = {} ({} BPM if 4/4)", cps, (cps * 240.0) as u32);
        println!("  In {} seconds:", duration);
        println!("  - {} cycles", cycles);
        println!("  - {} beats total", beats as u32);
        println!("  - Beat interval: {:.3}s", 1.0 / (cps * 4.0));

        // Actually render to verify
        let audio = executor.render(&env, duration).expect("Failed to render");
        println!("  - Generated {} samples", audio.data.len());
        println!();
    }

    println!("=== Pattern Modifiers Demo ===\n");

    // Show how modifiers work
    let patterns = vec![
        ("bd bd bd bd", "Standard 4/4 beat"),
        ("bd*4", "Same as above using repeat"),
        ("bd bd*2", "Second beat doubled (3 total)"),
        ("[bd hh]*2", "Pattern repeated twice fast"),
        ("bd/2", "Beat stretched over 2 cycles"),
        ("<bd sn>", "Alternates between bd and sn each cycle"),
    ];

    println!("Common pattern modifiers:\n");
    for (pattern, description) in patterns {
        println!("  \"{}\"", pattern);
        println!("    → {}\n", description);
    }

    println!("=== How to Use ===\n");
    println!("1. Set CPS (tempo):");
    println!("   executor.set_cps(0.5);  // 120 BPM");
    println!("   executor.set_cps(1.0);  // 240 BPM (default)");
    println!("   executor.set_cps(2.0);  // 480 BPM");
    println!();
    println!("2. Chain modifiers:");
    println!("   \"bd*2/3\"     → repeat 2x, then slow by 3");
    println!("   \"[bd sn]*4\"  → fast sequence repeated 4x");
    println!("   \"bd sn*2 hh\" → only middle element doubled");
}
