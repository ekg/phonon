use phonon::nom_parser::parse_dsl;

fn main() {
    let code = r#"
        ~lfo: sin 0.5 >> mul 0.5 >> add 0.5
        ~bass: saw 55 >> lpf ~lfo * 2000 + 500 0.8
        o: ~bass >> mul 0.4
    "#;

    println!("Parsing DSL:\n{}", code);

    match parse_dsl(code) {
        Ok(env) => {
            println!("✓ Parsed successfully");
            println!("  Buses: {:?}", env.ref_chains.keys().collect::<Vec<_>>());
            println!("  Has output: {}", env.output_chain.is_some());
        }
        Err(e) => {
            println!("✗ Parse error: {}", e);
        }
    }
}
