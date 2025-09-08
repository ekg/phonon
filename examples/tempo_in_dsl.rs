use phonon::phonon_lang::{PhononParser, PhononEnv};
use phonon::simple_dsp_executor::SimpleDspExecutor;

fn main() {
    println!("=== Setting Tempo in Phonon DSL ===\n");
    
    let sample_rate = 44100.0;
    
    // Example 1: Using BPM
    println!("Example 1: Setting BPM");
    println!("{}", "-".repeat(40));
    
    let code_bpm = r#"
        bpm: 120
        ~kick: sin 60 >> mul 0.5
        ~snare: noise >> hpf 2000 0.9 >> mul 0.3
        ~drums: s "bd ~ sn ~"
        o: ~drums
    "#;
    
    println!("Code:");
    println!("{}", code_bpm);
    
    let mut env = PhononEnv::new(sample_rate);
    match env.eval(code_bpm) {
        Ok(()) => {
            println!("\nParsed successfully!");
            println!("  CPS set to: {}", env.cps);
            println!("  Equivalent BPM: {}", env.cps * 240.0);
            
            // Use with executor
            let mut executor = SimpleDspExecutor::new(sample_rate);
            executor.set_cps(env.cps as f32);
            
            println!("  Executor CPS set to: {}", env.cps);
            println!("  This means:");
            println!("    - 1 cycle = {} seconds", 1.0 / env.cps);
            println!("    - 4 beats = {} seconds", 1.0 / env.cps);
            println!("    - 1 beat = {} seconds", 1.0 / (env.cps * 4.0));
        },
        Err(e) => println!("Parse error: {}", e),
    }
    
    // Example 2: Using CPS directly
    println!("\n\nExample 2: Setting CPS directly");
    println!("{}", "-".repeat(40));
    
    let code_cps = r#"
        cps: 2.0
        ~kick: sin 60 >> mul 0.5
        ~pattern: s "bd*4"
        o: ~pattern
    "#;
    
    println!("Code:");
    println!("{}", code_cps);
    
    let mut env = PhononEnv::new(sample_rate);
    match env.eval(code_cps) {
        Ok(()) => {
            println!("\nParsed successfully!");
            println!("  CPS: {}", env.cps);
            println!("  Equivalent BPM: {}", env.cps * 240.0);
            
            let mut executor = SimpleDspExecutor::new(sample_rate);
            executor.set_cps(env.cps as f32);
            
            println!("  This means:");
            println!("    - {} cycles per second", env.cps);
            println!("    - 1 cycle = {} seconds", 1.0 / env.cps);
            println!("    - {} beats per second", env.cps * 4.0);
        },
        Err(e) => println!("Parse error: {}", e),
    }
    
    // Example 3: Different musical styles
    println!("\n\nExample 3: Tempo for Different Styles");
    println!("{}", "-".repeat(40));
    
    let styles = vec![
        ("Hip Hop", 85.0),
        ("House", 128.0),
        ("Techno", 130.0),
        ("Drum & Bass", 174.0),
        ("Footwork", 160.0),
    ];
    
    for (style, bpm) in styles {
        let cps = bpm / 240.0;
        println!("\n{} ({}BPM):", style, bpm);
        println!("  cps: {:.3}", cps);
        println!("  Pattern cycle duration: {:.3}s", 1.0 / cps);
        
        let code = format!(r#"
            bpm: {}
            ~kick: sin 60 >> mul 0.5
            ~hat: noise >> hpf 8000 0.95 >> mul 0.2
            o: s "~kick ~ ~hat ~"
        "#, bpm);
        
        println!("  Example code:");
        for line in code.lines() {
            if !line.trim().is_empty() {
                println!("    {}", line.trim());
            }
        }
    }
    
    println!("\n\n=== Complete Example with Pattern Transformations ===");
    println!("{}", "-".repeat(40));
    
    let complete_example = r#"
        // Set tempo to 120 BPM
        bpm: 120
        
        // Pattern with transformations using >> operator
        ~drums: s "bd sn hh cp" >> fast 2 >> every 4 (rev)
        ~bass: s "c2 e2 g2 c3" >> slow 2
        ~hats: s "hh*8" >> degrade
        
        // Mix everything
        o: ~drums
    "#;
    
    println!("Full DSL code with tempo and pattern transformations:");
    println!("{}", complete_example);
    
    let mut env = PhononEnv::new(sample_rate);
    match env.eval(complete_example) {
        Ok(()) => {
            println!("\nParsed successfully!");
            println!("  Tempo: 120 BPM (CPS = {})", env.cps);
            println!("  Patterns defined with transformations");
        },
        Err(e) => println!("Parse error: {}", e),
    }
    
    println!("\n=== How to Use ===");
    println!("{}", "-".repeat(40));
    
    println!("1. Set tempo at the top of your code:");
    println!("   bpm: 120  // For BPM");
    println!("   cps: 0.5  // For cycles per second");
    println!();
    println!("2. Use pattern transformations with >>:");
    println!("   s \"bd sn\" >> fast 2");
    println!("   s \"bd sn\" >> rev >> degrade");
    println!("   s \"bd sn\" >> every 4 (slow 2)");
    println!();
    println!("3. BPM to CPS conversion:");
    println!("   CPS = BPM / 240  (for 4 beats per cycle)");
    println!("   120 BPM = 0.5 CPS");
    println!("   128 BPM = 0.533 CPS");
    println!("   140 BPM = 0.583 CPS");
    println!();
    println!("4. The tempo affects pattern playback speed:");
    println!("   Higher CPS = Faster playback");
    println!("   Lower CPS = Slower playback");
}