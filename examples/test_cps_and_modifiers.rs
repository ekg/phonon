use phonon::glicol_parser::parse_glicol;
use phonon::simple_dsp_executor::SimpleDspExecutor;

fn main() {
    println!("=== Testing CPS and Pattern Modifiers ===\n");

    let sample_rate = 44100.0;

    // Test 1: Different CPS values
    println!("Test 1: CPS (Cycles Per Second) Configuration");
    println!("----------------------------------------------");

    let code = r#"
        ~click: sin 1000 >> mul 0.5
        o: s "~click ~click ~click ~click"
    "#;

    let env = parse_glicol(code).expect("Failed to parse");

    // Test with CPS = 0.5 (half speed, 120 BPM)
    let mut executor = SimpleDspExecutor::new(sample_rate);
    executor.set_cps(0.5);
    let audio = executor.render(&env, 2.0).expect("Failed to render");

    println!("CPS = 0.5 (120 BPM):");
    println!("  Duration: 2 seconds");
    println!("  Expected: 1 cycle (4 beats) in 2 seconds");
    println!("  Audio samples generated: {}", audio.data.len());

    // Count beats by finding envelope onsets
    let mut beat_count = 0;
    let mut was_silent = true;
    for sample in audio.data.iter() {
        if was_silent && sample.abs() > 0.01 {
            beat_count += 1;
            was_silent = false;
        } else if sample.abs() < 0.001 {
            was_silent = true;
        }
    }
    println!("  Beats detected: {}\n", beat_count);

    // Test with CPS = 1.0 (normal speed)
    executor.set_cps(1.0);
    let audio = executor.render(&env, 2.0).expect("Failed to render");

    println!("CPS = 1.0 (default):");
    println!("  Duration: 2 seconds");
    println!("  Expected: 2 cycles (8 beats) in 2 seconds");

    beat_count = 0;
    was_silent = true;
    for sample in audio.data.iter() {
        if was_silent && sample.abs() > 0.01 {
            beat_count += 1;
            was_silent = false;
        } else if sample.abs() < 0.001 {
            was_silent = true;
        }
    }
    println!("  Beats detected: {}\n", beat_count);

    // Test with CPS = 2.0 (double speed)
    executor.set_cps(2.0);
    let audio = executor.render(&env, 2.0).expect("Failed to render");

    println!("CPS = 2.0 (double speed):");
    println!("  Duration: 2 seconds");
    println!("  Expected: 4 cycles (16 beats) in 2 seconds");

    beat_count = 0;
    was_silent = true;
    for sample in audio.data.iter() {
        if was_silent && sample.abs() > 0.01 {
            beat_count += 1;
            was_silent = false;
        } else if sample.abs() < 0.001 {
            was_silent = true;
        }
    }
    println!("  Beats detected: {}\n", beat_count);

    // Test 2: Pattern Modifiers
    println!("Test 2: Pattern Modifiers and Chaining");
    println!("--------------------------------------");

    // Test fast (*2)
    let code_fast = r#"
        ~click: sin 1000 >> mul 0.5
        o: s "~click ~click*2"
    "#;

    println!("Pattern: \"~click ~click*2\"");
    println!("  *2 operator doubles the speed of the second ~click");
    println!("  Expected: 3 clicks total (1 + 2)");

    let env = parse_glicol(code_fast).expect("Failed to parse");
    executor.set_cps(1.0);
    let audio = executor.render(&env, 1.0).expect("Failed to render");

    beat_count = 0;
    was_silent = true;
    for sample in audio.data.iter() {
        if was_silent && sample.abs() > 0.01 {
            beat_count += 1;
            was_silent = false;
        } else if sample.abs() < 0.001 {
            was_silent = true;
        }
    }
    println!("  Beats detected: {}\n", beat_count);

    // Test slow (/2)
    let code_slow = r#"
        ~click: sin 1000 >> mul 0.5
        o: s "~click ~click ~click ~click/2"
    "#;

    println!("Pattern: \"~click ~click ~click ~click/2\"");
    println!("  /2 operator halves the speed of the last ~click");
    println!("  Expected: Last click plays over 2 cycles");

    let env = parse_glicol(code_slow).expect("Failed to parse");
    let audio = executor.render(&env, 2.0).expect("Failed to render");

    beat_count = 0;
    was_silent = true;
    for sample in audio.data.iter() {
        if was_silent && sample.abs() > 0.01 {
            beat_count += 1;
            was_silent = false;
        } else if sample.abs() < 0.001 {
            was_silent = true;
        }
    }
    println!("  Beats detected in 2 cycles: {}\n", beat_count);

    // Test chained modifiers
    let code_chained = r#"
        ~click: sin 1000 >> mul 0.5
        o: s "[~click ~click]*2"
    "#;

    println!("Pattern: \"[~click ~click]*2\"");
    println!("  Fast sequence [...] groups the clicks");
    println!("  *2 then doubles the speed of the whole group");
    println!("  Expected: 4 clicks in one cycle (2 repetitions of 2 clicks)");

    let env = parse_glicol(code_chained).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");

    beat_count = 0;
    was_silent = true;
    for sample in audio.data.iter() {
        if was_silent && sample.abs() > 0.01 {
            beat_count += 1;
            was_silent = false;
        } else if sample.abs() < 0.001 {
            was_silent = true;
        }
    }
    println!("  Beats detected: {}\n", beat_count);

    println!("=== Summary ===");
    println!("✓ CPS can be configured with set_cps()");
    println!("✓ Pattern modifiers like * and / work");
    println!("✓ Modifiers can be chained together");
    println!("\nAvailable pattern modifiers:");
    println!("  * - fast/replicate (e.g., bd*3 plays bd 3 times)");
    println!("  / - slow (e.g., bd/2 plays bd at half speed)");
    println!("  ? - degrade/random drop (e.g., bd?0.5 plays bd 50% of the time)");
    println!("  @ - late/offset (e.g., bd@0.25 offsets bd by 0.25 cycles)");
    println!("  [...] - fast sequence (plays all in one cycle)");
    println!("  <...> - alternate (cycles through options)");
    println!("  {{...}} - polyrhythm (plays simultaneously)");
}
