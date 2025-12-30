use phonon::compositional_parser::parse_program;
use phonon::compositional_compiler::compile_program;

fn main() {
    let code = r#"
o2 $ s "birds(3,8)" # n "3 7 2 1" # note "<b3 a2 c3>" # speed 0.25 # delay 0.8 0.6
o3 $ saw # note "c2'maj" # ar 2 0.2 # lpf 300
"#;

    println!("Parsing code:");
    println!("{}", code);
    println!("---");

    match parse_program(code) {
        Ok((rest, stmts)) => {
            println!("Parsed {} statements", stmts.len());
            for (i, stmt) in stmts.iter().enumerate() {
                println!("  {}: {:?}", i, stmt);
            }
            println!("Remaining: {:?}", rest);

            // Try to compile
            match compile_program(stmts, 44100.0, None) {
                Ok(mut graph) => {
                    println!("Compiled successfully!");
                    println!("Has output: {}", graph.has_output());
                    println!("Bus names: {:?}", graph.get_all_bus_names());
                    println!("Output channels: {:?}", graph.get_output_channels());

                    // Render audio
                    let buffer = graph.render(44100);
                    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
                    let max = buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
                    println!("RMS: {}, Max: {}", rms, max);
                }
                Err(e) => println!("Compile error: {}", e),
            }
        }
        Err(e) => println!("Parse error: {:?}", e),
    }
}
