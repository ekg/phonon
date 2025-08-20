//! Debug parser issue
//! Run with: cargo run --example test_parser_debug

use fermion::enhanced_parser::EnhancedParser;

fn main() {
    println!("Parser Debug\n");
    
    let dsl = r#"
~osc: sine(440)
out: ~osc * 0.3
"#;
    
    println!("Input DSL:");
    println!("{}", dsl);
    
    let mut parser = EnhancedParser::new(44100.0);
    
    // Parse and check what was created
    match parser.parse(dsl) {
        Ok(graph) => {
            println!("\n✅ Parse successful!");
            println!("\nGraph contents:");
            println!("  Nodes: {}", graph.nodes.len());
            println!("  Buses: {}", graph.buses.len()); 
            println!("  Connections: {}", graph.connections.len());
            
            println!("\nNodes:");
            for (id, node) in &graph.nodes {
                println!("  {}: {:?}", id.0, node);
            }
            
            println!("\nBuses:");
            for (id, value) in &graph.buses {
                println!("  {}: {}", id.0, value);
            }
            
            println!("\nConnections:");
            for conn in &graph.connections {
                println!("  {} -> {} (amount: {})", 
                         conn.from.0, conn.to.0, conn.amount);
            }
            
            // Check execution order
            let mut graph_mut = graph;
            match graph_mut.compute_execution_order() {
                Ok(_) => {
                    if let Ok(order) = graph_mut.get_execution_order() {
                        println!("\nExecution order:");
                        for (i, node_id) in order.iter().enumerate() {
                            println!("  {}: {}", i, node_id.0);
                        }
                    }
                }
                Err(e) => println!("\n❌ Failed to compute execution order: {}", e),
            }
        }
        Err(e) => println!("❌ Parse failed: {}", e),
    }
}