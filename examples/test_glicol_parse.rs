use phonon::glicol_parser::parse_glicol;

fn main() {
    let input = "~lfo: sin 0.5 >> mul 0.5 >> add 0.5\n~bass: saw 55 >> lpf 2000 0.8\no: ~bass >> reverb 0.8 0.5 >> mul 0.4";

    println!("Parsing: {}", input);
    match parse_glicol(input) {
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
