use phonon::unified_graph_parser::parse_dsl;

fn main() {
    let input = r#"tempo: 1.0
~drums: s("bd sn")
~fast_drums: ~drums $ fast 2
out: ~fast_drums
"#;

    match parse_dsl(input) {
        Ok((remaining, statements)) => {
            println!("Parsed {} statements", statements.len());
            for (i, stmt) in statements.iter().enumerate() {
                println!("Statement {}: {:?}", i, stmt);
            }
            println!("\nRemaining input: {:?}", remaining);
            println!("Remaining length: {}", remaining.len());
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}
