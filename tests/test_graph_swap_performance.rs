/// Test for performance degradation during repeated graph swaps
/// This simulates the live coding scenario where patterns are edited repeatedly

use phonon::unified_graph::UnifiedSignalGraph;
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use std::time::Instant;

fn compile_code(code: &str, sample_rate: f32) -> UnifiedSignalGraph {
    let (_, statements) = parse_dsl(code).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(sample_rate);
    let mut graph = compiler.compile(statements);
    graph.use_wall_clock = false; // Use deterministic timing for tests
    graph
}

fn measure_render_time(graph: &mut UnifiedSignalGraph, buffer_size: usize) -> f64 {
    let start = Instant::now();
    let _ = graph.render(buffer_size);
    start.elapsed().as_secs_f64() * 1000.0 // Return milliseconds
}

#[test]
fn test_graph_swap_performance_stability() {
    let sample_rate = 44100.0;
    let buffer_size = 512;

    // Initial pattern
    let code1 = r#"
cps: 1
out $ s "bd sn hh cp"
"#;

    // More complex pattern
    let code2 = r#"
cps: 1
out $ s "bd*2 sn hh*4 cp" # gain 0.8
"#;

    // Even more complex pattern
    let code3 = r#"
cps: 1
~drums $ s "bd*2 sn hh*4 cp" # gain 0.8
~bass $ saw 55
out $ ~drums + ~bass * 0.3
"#;

    let codes = [code1, code2, code3, code1, code2, code3];

    let mut graph = compile_code(codes[0], sample_rate);

    // Warm up
    for _ in 0..10 {
        let _ = graph.render(buffer_size);
    }

    let mut render_times: Vec<f64> = Vec::new();
    let mut swap_count = 0;

    // Simulate 30 graph swaps (like editing pattern 30 times)
    for i in 0..30 {
        let code = codes[i % codes.len()];

        // Create new graph
        let mut new_graph = compile_code(code, sample_rate);

        // Transfer state (like modal_editor does)
        new_graph.transfer_fx_states(&graph);
        new_graph.transfer_voice_manager(graph.take_voice_manager());

        // Swap
        graph = new_graph;
        swap_count += 1;

        // Measure render time after swap
        let time = measure_render_time(&mut graph, buffer_size);
        render_times.push(time);

        // Render a few more buffers
        for _ in 0..10 {
            let _ = graph.render(buffer_size);
        }
    }

    // Analyze: first 10 vs last 10 render times
    let first_10_avg: f64 = render_times[0..10].iter().sum::<f64>() / 10.0;
    let last_10_avg: f64 = render_times[20..30].iter().sum::<f64>() / 10.0;

    // Print diagnostics
    println!("Final graph stats:");
    println!("  Node count: {}", graph.node_count());
    println!("  Voice pool size: {} (active: {})",
        graph.voice_pool_size(), graph.active_voice_count());
    println!("  Voice breakdown: {:?}", graph.voice_type_breakdown());

    println!("Swap count: {}", swap_count);
    println!("First 10 avg: {:.3}ms", first_10_avg);
    println!("Last 10 avg: {:.3}ms", last_10_avg);
    println!("All times: {:?}", render_times.iter().map(|t| format!("{:.3}", t)).collect::<Vec<_>>());

    // Performance should be stable - last 10 shouldn't be more than 2x first 10
    assert!(
        last_10_avg < first_10_avg * 2.0,
        "Performance degraded: first 10 avg = {:.3}ms, last 10 avg = {:.3}ms (> 2x)",
        first_10_avg, last_10_avg
    );
}

#[test]
fn test_node_count_stability() {
    let sample_rate = 44100.0;

    let code1 = "cps: 1\nout $ s \"bd sn\"";
    let code2 = "cps: 1\nout $ s \"bd sn hh cp\"";
    let code3 = "cps: 1\nout $ s \"bd*2 sn hh*4 cp\" # gain 0.8";

    let codes = [code1, code2, code3, code1, code2, code3];

    let mut graph = compile_code(codes[0], sample_rate);
    let initial_node_count = graph.node_count();

    let mut node_counts: Vec<usize> = vec![initial_node_count];

    // Do 20 swaps
    for i in 1..20 {
        let code = codes[i % codes.len()];

        let mut new_graph = compile_code(code, sample_rate);
        new_graph.transfer_fx_states(&graph);
        new_graph.transfer_voice_manager(graph.take_voice_manager());

        graph = new_graph;
        node_counts.push(graph.node_count());
    }

    println!("Node counts over swaps: {:?}", node_counts);

    // Node count should be bounded - not growing linearly with swaps
    let max_count = *node_counts.iter().max().unwrap();
    let min_count = *node_counts.iter().min().unwrap();

    // The count should oscillate based on pattern complexity, not accumulate
    assert!(
        max_count < min_count * 3,
        "Node count unbounded growth! min={}, max={} (> 3x min)",
        min_count, max_count
    );
}

#[test]
fn test_memory_stability_with_many_swaps() {
    let sample_rate = 44100.0;
    let buffer_size = 512;

    // Start with a complex pattern
    let complex_code = r#"
cps: 2
~drums $ s "bd*4 sn*2 hh*8 cp*2" # gain 0.8
~bass $ saw "55 110" # lpf 800 0.8
~lead $ sine "220 440 660"
out $ ~drums * 0.4 + ~bass * 0.3 + ~lead * 0.2
"#;

    let simple_code = r#"
cps: 2
out $ s "bd sn"
"#;

    let mut graph = compile_code(simple_code, sample_rate);

    // Render some buffers
    for _ in 0..100 {
        let _ = graph.render(buffer_size);
    }

    // Now do rapid swaps between complex and simple
    let mut times: Vec<f64> = Vec::new();

    for i in 0..50 {
        let code = if i % 2 == 0 { complex_code } else { simple_code };

        let mut new_graph = compile_code(code, sample_rate);
        new_graph.transfer_fx_states(&graph);
        new_graph.transfer_voice_manager(graph.take_voice_manager());
        graph = new_graph;

        // Measure time to render 10 buffers
        let start = Instant::now();
        for _ in 0..10 {
            let _ = graph.render(buffer_size);
        }
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    // Check for performance anomalies
    let avg_time: f64 = times.iter().sum::<f64>() / times.len() as f64;
    let max_time = times.iter().cloned().fold(0.0, f64::max);

    println!("Average 10-buffer time: {:.3}ms", avg_time);
    println!("Max 10-buffer time: {:.3}ms", max_time);
    println!("Times: {:?}", times.iter().map(|t| format!("{:.2}", t)).collect::<Vec<_>>());

    // Max shouldn't be more than 5x average (allows for some variance)
    assert!(
        max_time < avg_time * 5.0,
        "Performance spike detected! avg={:.3}ms, max={:.3}ms (> 5x avg)",
        avg_time, max_time
    );
}
