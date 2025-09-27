use phonon::glicol_parser::*;

fn main() {
    // Test tokenization of "bass"
    let input = "~bass: saw 55";

    // We can't directly access the tokenizer, so let's just try parsing
    match parse_glicol(input) {
        Ok(env) => {
            println!("Success!");
            println!(
                "Ref chains: {:?}",
                env.ref_chains.keys().collect::<Vec<_>>()
            );
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
