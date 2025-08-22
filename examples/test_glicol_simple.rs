use phonon::glicol_parser::parse_glicol;

fn main() {
    // Test each line separately
    let lines = [
        "~lfo: sin 0.5 >> mul 0.5 >> add 0.5",
        "~bass: saw 55 >> lpf 2000 0.8",
        "o: ~bass >> reverb 0.8 0.5 >> mul 0.4",
    ];
    
    for line in &lines {
        println!("\nParsing: {}", line);
        match parse_glicol(line) {
            Ok(env) => {
                println!("Success!");
                println!("Output chain: {:?}", env.output_chain.is_some());
                println!("Ref chains: {}", env.ref_chains.len());
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}