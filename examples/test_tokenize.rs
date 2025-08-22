fn main() {
    let test = "~bass: saw 55";
    let chars: Vec<char> = test.chars().collect();
    
    println!("Chars: {:?}", chars);
    
    // Simulate tokenization
    let mut pos = 0;
    
    // Skip ~
    if chars[pos] == '~' {
        println!("Found tilde");
        pos += 1;
    }
    
    // Get identifier
    let start = pos;
    while pos < chars.len() && (chars[pos].is_ascii_alphanumeric() || chars[pos] == '_' || chars[pos] == '-') {
        pos += 1;
    }
    
    let ident: String = chars[start..pos].iter().collect();
    println!("Identifier: '{}'", ident);
    
    // Check what it would become
    let token = match ident.as_str() {
        "lfo" => "Token::Lfo",
        "bass" => "Token::Symbol(\"bass\")",
        _ => "Token::Symbol(...)",
    };
    
    println!("Would become: {}", token);
}