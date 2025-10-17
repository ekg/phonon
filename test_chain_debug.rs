use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

fn main() {
    let input = "out: saw(110) >> lpf(1000, 0.8)";
    println!("Parsing: {}", input);
    
    let result = parse_dsl(input);
    println!("Parse result: {:#?}", result);
    
    if let Ok((_, statements)) = result {
        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);
        
        // Render a bit
        let samples: Vec<f32> = (0..100).map(|_| graph.process_sample()).collect();
        let rms: f32 = (samples.iter().map(|x| x*x).sum::<f32>() / samples.len() as f32).sqrt();
        let dc: f32 = samples.iter().sum::<f32>() / samples.len() as f32;
        
        println!("RMS: {:.6}, DC offset: {:.6}", rms, dc);
    }
}
