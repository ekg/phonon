//! Manual test of signal graph and executor
//! Run with: cargo run --example manual_test

use fermion::signal_graph::{SignalGraph, Node, NodeId, SourceType, ProcessorType};
use fermion::signal_executor::SignalExecutor;

fn main() {
    println!("Manual Signal Graph Test\n");
    
    // Create graph manually
    let mut graph = SignalGraph::new(44100.0);
    
    // Add sine source
    let sine_node = Node::Source {
        id: NodeId("sine".to_string()),
        source_type: SourceType::Sine { freq: 440.0 },
    };
    let sine_id = graph.add_node(sine_node);
    println!("Added sine node: {:?}", sine_id);
    
    // Add gain node
    let gain_node = Node::Processor {
        id: NodeId("gain".to_string()),
        processor_type: ProcessorType::Gain { amount: 0.3 },
    };
    let gain_id = graph.add_node(gain_node);
    println!("Added gain node: {:?}", gain_id);
    
    // Add output node
    let output_node = Node::Output {
        id: NodeId("output".to_string()),
    };
    let output_id = graph.add_node(output_node);
    println!("Added output node: {:?}", output_id);
    
    // Connect sine -> gain -> output
    graph.connect(sine_id.clone(), gain_id.clone(), 1.0);
    graph.connect(gain_id.clone(), output_id.clone(), 1.0);
    
    println!("\nGraph structure:");
    println!("  Nodes: {}", graph.nodes.len());
    println!("  Connections: {}", graph.connections.len());
    
    // Create executor
    let mut executor = SignalExecutor::new(graph, 44100.0, 512);
    
    println!("\nInitializing executor...");
    executor.initialize().expect("Failed to initialize");
    
    println!("Processing audio...");
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
    
    if output.peak() > 0.0 {
        println!("\n✅ Audio generated successfully!");
        
        // Write to WAV
        let wav_path = "/tmp/manual_test.wav";
        output.write_wav(wav_path).expect("Failed to write WAV");
        println!("  Wrote WAV to: {}", wav_path);
    } else {
        println!("\n❌ NO AUDIO GENERATED!");
    }
}