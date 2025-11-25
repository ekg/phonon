
fn main() {
    // Test parsing individual lines
    let lines = vec![
        "~lfo: sin 0.5 >> mul 0.5 >> add 0.5",
        "~bass: saw 55 >> lpf ~lfo * 2000 + 500 0.8",
        "o: ~bass >> mul 0.4",
        "        o: ~bass >> mul 0.4", // with whitespace
    ];

    for line in lines {
        println!("\nTesting line: '{}'", line);

        // Try the simpler version first
        let trimmed = line.trim();
        if trimmed.starts_with("o:") {
            println!("  Line starts with 'o:'");
            let expr_part = trimmed.strip_prefix("o:").unwrap().trim();
            println!("  Expression part: '{}'", expr_part);
        }
    }
}
