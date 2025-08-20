//! Debug audio generation issue
//! Run with: cargo run --example test_audio_debug

use fermion::enhanced_parser::EnhancedParser;
use fermion::signal_executor::SignalExecutor;

fn main() {
    println!("Debugging audio generation...\n");
    
    let dsl = r#"
~osc: sine(440)
out: ~osc * 0.3
"#;
    
    println!("Parsing DSL:\n{}", dsl);
    
    let mut parser = EnhancedParser::new(44100.0);
    let graph = parser.parse(dsl).expect("Failed to parse");
    
    println!("\nGraph created:");
    println!("  Nodes: {}", graph.nodes.len());
    println!("  Buses: {}", graph.buses.len());
    println!("  Connections: {}", graph.connections.len());
    
    // Debug: Print nodes
    for (id, node) in &graph.nodes {
        println!("  Node {}: {:?}", id.0, node);
    }
    
    // Debug: Print connections
    for conn in &graph.connections {
        println!("  Connection: {} -> {} (amount: {})", 
                 conn.from.0, conn.to.0, conn.amount);
    }
    
    println!("\nInitializing executor...");
    let mut executor = SignalExecutor::new(graph, 44100.0, 512);
    executor.initialize().expect("Failed to initialize");
    
    println!("Processing block...");
    let output = executor.process_block().expect("Failed to process");
    
    println!("\nOutput:");
    println!("  Buffer size: {}", output.data.len());
    println!("  RMS: {:.6}", output.rms());
    println!("  Peak: {:.6}", output.peak());
    
    // Print first few samples
    print!("  First 10 samples: ");
    for i in 0..10.min(output.data.len()) {
        print!("{:.4} ", output.data[i]);
    }
    println!();
    
    if output.peak() == 0.0 {
        println!("\n❌ NO AUDIO GENERATED!");
        
        // Check if processors were created
        println!("\nDEBUG: Something went wrong in the signal chain!");
    } else {
        println!("\n✅ Audio generated successfully!");
    }
}