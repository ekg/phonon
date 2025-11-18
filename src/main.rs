#![allow(unused_assignments, unused_mut)]
//! Phonon CLI - Command-line interface for the Phonon synthesis system

use clap::{Parser, Subcommand};
use phonon::pattern::Pattern;
use phonon::simple_dsp_executor;
use std::cell::RefCell;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "phonon")]
#[command(about = "Phonon modular synthesis system", long_about = None)]
struct Cli {
    /// Number of threads for parallel processing (default: 4)
    #[arg(short = 't', long, default_value = "4", global = true)]
    threads: usize,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a DSL file to WAV
    Render {
        /// Input file (.phonon or .dsl) or inline DSL code
        input: String,

        /// Output WAV file path
        output: String,

        /// Duration in seconds (default: 10.0)
        #[arg(short, long, default_value = "10.0")]
        duration: f32,

        /// Number of cycles (overrides duration if specified)
        #[arg(short, long)]
        cycles: Option<u32>,

        /// Sample rate in Hz (default: 44100)
        #[arg(short, long, default_value = "44100")]
        sample_rate: u32,

        /// Master gain 0.0-1.0 (default: 0.8)
        #[arg(short, long, default_value = "0.8")]
        gain: f32,

        /// Fade in time in seconds (default: 0.01)
        #[arg(long, default_value = "0.01")]
        fade_in: f32,

        /// Fade out time in seconds (default: 0.01)
        #[arg(long, default_value = "0.01")]
        fade_out: f32,

        /// Block size for processing (default: 512)
        #[arg(short, long, default_value = "512")]
        block_size: usize,

        /// Use realtime rendering path (process_buffer) for profiling (default: true)
        #[arg(long, default_value = "true")]
        realtime: bool,

        /// Enable parallel processing (uses all CPU cores, default: true)
        #[arg(long, default_value = "true")]
        parallel: bool,
    },

    /// Play DSL file or code (render and auto-play)
    Play {
        /// Input file (.phonon) or inline DSL code
        input: String,

        /// Duration in seconds (default: 4.0)
        #[arg(short, long, default_value = "4.0")]
        duration: f32,

        /// Sample rate in Hz (default: 44100)
        #[arg(short, long, default_value = "44100")]
        sample_rate: u32,

        /// Master gain 0.0-1.0 (default: 0.8)
        #[arg(short, long, default_value = "0.8")]
        gain: f32,
    },

    /// Start live coding session with file watching
    Live {
        /// DSL file to watch and auto-reload
        #[arg(default_value = "live.ph")]
        file: PathBuf,

        /// Duration for each render (default: 4.0)
        #[arg(short, long, default_value = "4.0")]
        duration: f32,

        /// Enable pattern mode for Strudel-style patterns
        #[arg(short = 'P', long)]
        pattern: bool,

        /// OSC port to listen on (optional)
        #[arg(short, long, default_value = "9000")]
        port: u16,
    },

    /// Start interactive REPL
    Repl {},

    /// Open modal live coding editor
    Edit {
        /// Optional file to load
        file: Option<PathBuf>,

        /// Duration for each render (default: 4.0)
        #[arg(short, long, default_value = "4.0")]
        duration: f32,
    },

    /// Run tests on DSL files
    Test {
        /// Input file or directory
        input: PathBuf,
    },

    /// Send pattern to MIDI device
    Midi {
        /// Pattern to play (mini-notation)
        #[arg(short, long)]
        pattern: Option<String>,

        /// MIDI device name (partial match)
        #[arg(short, long)]
        device: Option<String>,

        /// Tempo in BPM (default: 120)
        #[arg(short, long, default_value = "120")]
        tempo: f32,

        /// Duration in beats (default: 16)
        #[arg(short = 'D', long, default_value = "16")]
        duration: f32,

        /// MIDI channel (0-15, default: 0)
        #[arg(short, long, default_value = "0")]
        channel: u8,

        /// Note velocity (0-127, default: 64)
        #[arg(short = 'v', long, default_value = "64")]
        velocity: u8,

        /// List MIDI devices and exit
        #[arg(short, long)]
        list: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Configure rayon thread pool with user-specified thread count
    // Default is 4 threads to prevent excessive CPU usage during rendering
    rayon::ThreadPoolBuilder::new()
        .num_threads(cli.threads)
        .build_global()
        .expect("Failed to initialize thread pool");

    match cli.command {
        Commands::Render {
            input,
            output,
            duration,
            cycles,
            sample_rate,
            gain,
            fade_in,
            fade_out,
            block_size,
            realtime,
            parallel,
        } => {
            use hound::{SampleFormat, WavSpec, WavWriter};
            use phonon::mini_notation_v3::parse_mini_notation;
            use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
            use std::collections::HashMap;

            // Read phonon file
            let dsl_code = if input == "-" {
                // Read from stdin
                use std::io::Read;
                let mut buffer = String::new();
                std::io::stdin().read_to_string(&mut buffer)?;
                buffer
            } else if input.ends_with(".ph")
                || input.ends_with(".phonon")
                || input.ends_with(".pho")
                || input.ends_with(".dsl")
            {
                std::fs::read_to_string(&input)?
            } else if std::path::Path::new(&input).exists() {
                // If it's a file path without extension, read it
                std::fs::read_to_string(&input)?
            } else {
                // Treat as inline DSL code
                input.clone()
            };

            // Calculate duration from cycles if specified
            let final_duration = if let Some(cycle_count) = cycles {
                cycle_count as f32
            } else {
                duration
            };

            // Print info
            println!("🎵 Phonon Renderer");
            println!("==================");
            println!(
                "Input:       {}",
                if input.ends_with(".ph")
                    || input.ends_with(".phonon")
                    || input.ends_with(".pho")
                    || input.ends_with(".dsl")
                {
                    &input
                } else {
                    "<inline>"
                }
            );
            println!("Output:      {output}");
            println!("Duration:    {final_duration} seconds");
            println!("Sample rate: {sample_rate} Hz");
            println!("Master gain: {gain:.1}");
            println!();

            // Parse and render using the compositional parser
            use phonon::compositional_compiler::compile_program;
            use phonon::compositional_parser::parse_program;

            // Parse the DSL
            let (remaining, statements) =
                parse_program(&dsl_code).map_err(|e| format!("Failed to parse DSL: {:?}", e))?;

            // Check for parse errors (unparsed input remaining)
            if !remaining.trim().is_empty() {
                use phonon::error_diagnostics::{
                    check_for_common_mistakes, diagnose_parse_failure,
                };

                // Provide detailed diagnostic
                let diagnostic = diagnose_parse_failure(&dsl_code, remaining);
                eprintln!("{}", diagnostic);

                // Check for common mistakes in the entire file
                let warnings = check_for_common_mistakes(&dsl_code);
                if !warnings.is_empty() {
                    eprintln!("⚠️  Additional warnings:");
                    for warning in warnings {
                        eprintln!("  • {}", warning);
                    }
                }

                eprintln!();
                eprintln!("The renderer will continue with the successfully parsed portion.");
                eprintln!();
            }

            // Compile to graph (with auto-routing)
            let mut graph = compile_program(statements, sample_rate as f32)
                .map_err(|e| format!("Failed to compile: {}", e))?;

            // Print auto-routing info if it happened
            if graph.has_output() && !graph.get_all_bus_names().is_empty() {
                let bus_count = graph.get_all_bus_names().len();
                println!("🔀 Auto-routing: Mixing {} buses to output", bus_count);
            }

            let mut buses: HashMap<String, phonon::unified_graph::NodeId> = HashMap::new();
            let mut out_signal = None;

            // Parse DSL helper function
            fn parse_file_to_graph(
                content: &str,
                graph: &mut UnifiedSignalGraph,
                buses: &mut HashMap<String, phonon::unified_graph::NodeId>,
            ) -> Option<phonon::unified_graph::NodeId> {
                let mut output_node = None;

                // Helper function to parse sub-expressions in additions
                fn parse_sub_expression(
                    graph: &mut UnifiedSignalGraph,
                    expr_str: &str,
                    buses: &HashMap<String, phonon::unified_graph::NodeId>,
                ) -> phonon::unified_graph::NodeId {
                    let expr_str = expr_str.trim();

                    // Check for multiplication within this sub-expression
                    if expr_str.contains('*') {
                        let parts: Vec<&str> = expr_str.split('*').map(|s| s.trim()).collect();
                        if parts.len() == 2 {
                            // Parse left side recursively
                            let left = if parts[0].starts_with('~') {
                                if let Some(&bus_id) = buses.get(parts[0]) {
                                    bus_id
                                } else {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                }
                            } else {
                                let bus_key = format!("~{}", parts[0]);
                                if let Some(&bus_id) = buses.get(&bus_key) {
                                    bus_id
                                } else if let Ok(val) = parts[0].parse::<f32>() {
                                    graph.add_node(SignalNode::Constant { value: val })
                                } else {
                                    parse_expression_to_node(graph, parts[0], buses).unwrap_or_else(
                                        || graph.add_node(SignalNode::Constant { value: 0.0 }),
                                    )
                                }
                            };

                            // Right side is usually a gain value
                            if let Ok(gain) = parts[1].parse::<f32>() {
                                return graph.add_node(SignalNode::Multiply {
                                    a: Signal::Node(left),
                                    b: Signal::Value(gain),
                                });
                            } else {
                                let right = parse_sub_expression(graph, parts[1], buses);
                                return graph.add_node(SignalNode::Multiply {
                                    a: Signal::Node(left),
                                    b: Signal::Node(right),
                                });
                            }
                        }
                    }

                    // Check if it's a bus reference
                    let bus_key = if expr_str.starts_with('~') {
                        expr_str.to_string()
                    } else {
                        format!("~{expr_str}")
                    };

                    if let Some(&bus_id) = buses.get(&bus_key) {
                        return bus_id;
                    }

                    // Check if it's a number
                    if let Ok(val) = expr_str.parse::<f32>() {
                        return graph.add_node(SignalNode::Constant { value: val });
                    }

                    // Try parsing as oscillator or other expression
                    parse_expression_to_node(graph, expr_str, buses)
                        .unwrap_or_else(|| graph.add_node(SignalNode::Constant { value: 0.0 }))
                }

                // Helper to parse parameters (patterns, buses, numbers)
                let parse_parameter = |graph: &mut UnifiedSignalGraph,
                                       param_str: &str,
                                       buses: &HashMap<String, phonon::unified_graph::NodeId>,
                                       default_value: f32|
                 -> Signal {
                    let param_str = param_str.trim();
                    // Pattern: "100 200 300"
                    if param_str.starts_with('"') && param_str.ends_with('"') {
                        let pattern_str = &param_str[1..param_str.len() - 1];
                        let pattern = parse_mini_notation(pattern_str);
                        let pattern_node = graph.add_node(SignalNode::Pattern {
                            pattern_str: pattern_str.to_string(),
                            pattern,
                            last_value: default_value,
                            last_trigger_time: -1.0,
                        });
                        Signal::Node(pattern_node)
                    }
                    // Bus reference: ~lfo
                    else if param_str.starts_with('~') {
                        if let Some(&bus_id) = buses.get(param_str) {
                            Signal::Node(bus_id)
                        } else {
                            Signal::Value(default_value)
                        }
                    }
                    // Try parsing as number
                    else if let Ok(val) = param_str.parse::<f32>() {
                        Signal::Value(val)
                    } else {
                        Signal::Value(default_value)
                    }
                };

                // Parse DSL line by line
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }

                    // Handle tempo/cps
                    if trimmed.starts_with("tempo ") || trimmed.starts_with("cps ") {
                        let parts: Vec<&str> = trimmed.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(cps_value) = parts[1].parse::<f32>() {
                                graph.set_cps(cps_value);
                            }
                        }
                        continue;
                    }

                    // Parse assignment or output
                    if trimmed.starts_with("out ")
                        || trimmed.starts_with("out=")
                        || trimmed.contains('=')
                    {
                        let (target, expr) = if trimmed.contains('=') {
                            // Handle assignment: out = ... or ~bus = ...
                            let pos = trimmed.find('=').unwrap();
                            let target = trimmed[..pos].trim();
                            let expr = trimmed[pos + 1..].trim();
                            (target, expr)
                        } else if trimmed.starts_with("out ") {
                            // Handle: out <expr> (no equals)
                            ("out", trimmed[4..].trim())
                        } else {
                            continue;
                        };

                        // Parse expression into node
                        let node_id = if let Some(chain_pos) =
                            expr.find(">>").or_else(|| expr.find("<<"))
                        {
                            // Signal chain: source >> effect OR effect << source
                            let is_reversed = expr.contains("<<");
                            let (source_str, effect_str) = if is_reversed {
                                // << is reversed: effect << source
                                let parts: Vec<&str> = expr.splitn(2, "<<").collect();
                                (parts[1].trim(), parts[0].trim())
                            } else {
                                // >> is normal: source >> effect
                                (expr[..chain_pos].trim(), expr[chain_pos + 2..].trim())
                            };

                            // Parse source
                            let source_node = if source_str.starts_with("s(") {
                                // Sample player
                                if let Some(start) = source_str.find('(') {
                                    if let Some(end) = source_str.find(')') {
                                        let pattern_str = source_str[start + 1..end].trim();
                                        // Remove quotes if present
                                        let pattern_str = if pattern_str.starts_with('"')
                                            && pattern_str.ends_with('"')
                                        {
                                            &pattern_str[1..pattern_str.len() - 1]
                                        } else {
                                            pattern_str
                                        };
                                        let pattern = parse_mini_notation(pattern_str);
                                        graph.add_node(SignalNode::Sample {
                                            pattern_str: pattern_str.to_string(),
                                            pattern,
                                            last_trigger_time: -1.0,
                                            last_cycle: -1,
                                            playback_positions: HashMap::new(),
                                            gain: Signal::Value(1.0),
                                            pan: Signal::Value(0.0),
                                            speed: Signal::Value(1.0),
                                            cut_group: Signal::Value(0.0),
                                            n: Signal::Value(0.0),
                                            note: Signal::Value(0.0),
                                            attack: Signal::Value(0.0),
                                            release: Signal::Value(0.0),
                                            envelope_type: None,
                                            unit_mode: Signal::Value(0.0), // 0 = rate mode (default)
                                            loop_enabled: Signal::Value(0.0), // 0 = no loop (default)
                                            begin: Signal::Value(0.0),
                                            end: Signal::Value(1.0),
                                        })
                                    } else {
                                        graph.add_node(SignalNode::Constant { value: 0.0 })
                                    }
                                } else {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                }
                            } else if source_str.starts_with("sine(")
                                || source_str.starts_with("saw(")
                                || source_str.starts_with("square(")
                                || source_str.starts_with("noise")
                            {
                                // Oscillator
                                if source_str.starts_with("noise") {
                                    graph.add_node(SignalNode::Noise { seed: 12345 })
                                } else {
                                    let osc_type = if source_str.starts_with("sine") {
                                        Waveform::Sine
                                    } else if source_str.starts_with("saw") {
                                        Waveform::Saw
                                    } else if source_str.starts_with("square") {
                                        Waveform::Square
                                    } else {
                                        Waveform::Sine // Default
                                    };

                                    // Extract frequency parameter
                                    if let Some(start) = source_str.find('(') {
                                        if let Some(end) = source_str.find(')') {
                                            let param = source_str[start + 1..end].trim();
                                            let freq_signal =
                                                parse_parameter(graph, param, buses, 440.0);
                                            graph.add_node(SignalNode::Oscillator {
                                                freq: freq_signal,
                                                waveform: osc_type,
                                                phase: RefCell::new(0.0),
                                                pending_freq: RefCell::new(None),
                                                last_sample: RefCell::new(0.0),
                                            })
                                        } else {
                                            graph.add_node(SignalNode::Constant { value: 0.0 })
                                        }
                                    } else {
                                        graph.add_node(SignalNode::Constant { value: 0.0 })
                                    }
                                }
                            } else if source_str.starts_with('~') {
                                // Bus reference
                                if let Some(&bus_id) = buses.get(source_str) {
                                    bus_id
                                } else {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                }
                            } else {
                                graph.add_node(SignalNode::Constant { value: 0.0 })
                            };

                            // Parse effect (filter)
                            if effect_str.starts_with("lpf(") || effect_str.starts_with("hpf(") {
                                let is_lpf = effect_str.starts_with("lpf");

                                if let Some(start) = effect_str.find('(') {
                                    if let Some(end) = effect_str.rfind(')') {
                                        let params = effect_str[start + 1..end].trim();
                                        let parts: Vec<&str> =
                                            params.split(',').map(|s| s.trim()).collect();

                                        let cutoff_signal = if !parts.is_empty() {
                                            parse_parameter(graph, parts[0], buses, 1000.0)
                                        } else {
                                            Signal::Value(1000.0)
                                        };

                                        let q_signal = if parts.len() > 1 {
                                            parse_parameter(graph, parts[1], buses, 1.0)
                                        } else {
                                            Signal::Value(1.0)
                                        };

                                        if is_lpf {
                                            graph.add_node(SignalNode::LowPass {
                                                input: Signal::Node(source_node),
                                                cutoff: cutoff_signal,
                                                q: q_signal,
                                                state: Default::default(),
                                            })
                                        } else {
                                            graph.add_node(SignalNode::HighPass {
                                                input: Signal::Node(source_node),
                                                cutoff: cutoff_signal,
                                                q: q_signal,
                                                state: Default::default(),
                                            })
                                        }
                                    } else {
                                        source_node
                                    }
                                } else {
                                    source_node
                                }
                            } else {
                                source_node
                            }
                        } else if expr.contains('+') {
                            // Handle addition (for mixing signals)
                            let parts: Vec<&str> = expr.split('+').map(|s| s.trim()).collect();
                            if parts.len() == 2 {
                                // Parse left side
                                let left_node = parse_sub_expression(graph, parts[0], buses);
                                // Parse right side
                                let right_node = parse_sub_expression(graph, parts[1], buses);

                                graph.add_node(SignalNode::Add {
                                    a: Signal::Node(left_node),
                                    b: Signal::Node(right_node),
                                })
                            } else if parts.len() > 2 {
                                // Chain multiple additions
                                let mut result = parse_sub_expression(graph, parts[0], buses);
                                for part in &parts[1..] {
                                    let next_node = parse_sub_expression(graph, part, buses);
                                    result = graph.add_node(SignalNode::Add {
                                        a: Signal::Node(result),
                                        b: Signal::Node(next_node),
                                    });
                                }
                                result
                            } else {
                                graph.add_node(SignalNode::Constant { value: 0.0 })
                            }
                        } else if expr.contains('*') && !expr.contains('"') {
                            // Handle multiplication (but not if there are quotes - could be pattern syntax)
                            let parts: Vec<&str> = expr.split('*').map(|s| s.trim()).collect();
                            if parts.len() == 2 {
                                let left_node = if parts[0].starts_with('~') {
                                    if let Some(&bus_id) = buses.get(parts[0]) {
                                        bus_id
                                    } else {
                                        graph.add_node(SignalNode::Constant { value: 0.0 })
                                    }
                                } else if let Ok(val) = parts[0].parse::<f32>() {
                                    graph.add_node(SignalNode::Constant { value: val })
                                } else {
                                    // Check if it's a bus without ~ prefix
                                    let bus_key = format!("~{}", parts[0]);
                                    if let Some(&bus_id) = buses.get(&bus_key) {
                                        bus_id
                                    } else {
                                        // Try parsing as oscillator
                                        parse_expression_to_node(graph, parts[0], buses)
                                            .unwrap_or_else(|| {
                                                graph.add_node(SignalNode::Constant { value: 0.0 })
                                            })
                                    }
                                };

                                // Parse right side - could be number, pattern, or bus reference
                                let right_signal = if let Ok(val) = parts[1].parse::<f32>() {
                                    Signal::Value(val)
                                } else if parts[1].starts_with('~') {
                                    if let Some(&bus_id) = buses.get(parts[1]) {
                                        Signal::Node(bus_id)
                                    } else {
                                        Signal::Value(1.0)
                                    }
                                } else if let Some(node_id) =
                                    parse_expression_to_node(graph, parts[1], buses)
                                {
                                    Signal::Node(node_id)
                                } else {
                                    Signal::Value(1.0)
                                };

                                graph.add_node(SignalNode::Multiply {
                                    a: Signal::Node(left_node),
                                    b: right_signal,
                                })
                            } else {
                                graph.add_node(SignalNode::Constant { value: 0.0 })
                            }
                        } else {
                            // Check if it's a plain bus reference
                            let bus_key = if expr.starts_with('~') {
                                expr.to_string()
                            } else {
                                format!("~{expr}")
                            };

                            if let Some(&bus_id) = buses.get(&bus_key) {
                                bus_id
                            } else {
                                // Simple expression
                                parse_expression_to_node(graph, expr, buses).unwrap_or_else(|| {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                })
                            }
                        };

                        // Store signal
                        if target == "out" {
                            output_node = Some(node_id);
                        } else if target.starts_with("out") && target.len() > 3 {
                            // Check for numbered outputs: out1, out2, etc.
                            let channel_str = &target[3..];
                            if let Ok(channel) = channel_str.parse::<usize>() {
                                graph.set_output_channel(channel, node_id);
                            } else {
                                // Not a valid number, treat as bus
                                let bus_name = if target.starts_with('~') {
                                    target.to_string()
                                } else {
                                    format!("~{target}")
                                };
                                buses.insert(bus_name, node_id);
                            }
                        } else {
                            let bus_name = if target.starts_with('~') {
                                target.to_string()
                            } else {
                                format!("~{target}")
                            };
                            buses.insert(bus_name, node_id);
                        }
                    }
                }

                // Helper to apply pattern transformations
                fn apply_transform(
                    pattern: phonon::pattern::Pattern<String>,
                    transform: &str,
                ) -> phonon::pattern::Pattern<String> {
                    let transform = transform.trim();

                    // Parse transformation: fast 2, slow 2, rev, every 4 fast 2
                    if transform.starts_with("fast ") {
                        if let Ok(factor) = transform[5..].trim().parse::<f64>() {
                            return pattern.fast(Pattern::pure(factor));
                        }
                    } else if transform.starts_with("slow ") {
                        if let Ok(factor) = transform[5..].trim().parse::<f64>() {
                            return pattern.slow(Pattern::pure(factor));
                        }
                    } else if transform == "rev" {
                        return pattern.rev();
                    } else if transform.starts_with("every ") {
                        // Parse: every 4 fast 2, every 4 (fast 2), every 4 rev
                        let rest = transform[6..].trim();
                        if let Some(space_pos) = rest.find(' ') {
                            if let Ok(n) = rest[..space_pos].parse::<i32>() {
                                let inner_transform = rest[space_pos + 1..].trim();
                                // Remove parentheses if present
                                let inner_transform = if inner_transform.starts_with('(')
                                    && inner_transform.ends_with(')')
                                {
                                    &inner_transform[1..inner_transform.len() - 1]
                                } else {
                                    inner_transform
                                };
                                let inner_transform = inner_transform.to_string();
                                return pattern
                                    .every(n, move |p| apply_transform(p, &inner_transform));
                            }
                        }
                    }

                    pattern
                }

                // Helper function for parsing expressions to nodes
                fn parse_expression_to_node(
                    graph: &mut UnifiedSignalGraph,
                    expr: &str,
                    buses: &HashMap<String, phonon::unified_graph::NodeId>,
                ) -> Option<phonon::unified_graph::NodeId> {
                    let expr = expr.trim();

                    // Check for |> or <| pattern transformations
                    if expr.contains(" |> ") || expr.contains(" <| ") {
                        let (base_expr, transform_expr, reversed) = if expr.contains(" |> ") {
                            let parts: Vec<&str> = expr.splitn(2, " |> ").collect();
                            if parts.len() == 2 {
                                (parts[0].trim(), parts[1].trim(), false)
                            } else {
                                (expr, "", false)
                            }
                        } else {
                            // <| is reversed: transform <| base
                            let parts: Vec<&str> = expr.splitn(2, " <| ").collect();
                            if parts.len() == 2 {
                                (parts[1].trim(), parts[0].trim(), true)
                            } else {
                                (expr, "", false)
                            }
                        };

                        if !transform_expr.is_empty() {
                            // Parse base expression to get pattern
                            if base_expr.starts_with("s(") {
                                if let Some(start) = base_expr.find('(') {
                                    if let Some(end) = base_expr.find(')') {
                                        let pattern_str = base_expr[start + 1..end].trim();
                                        let pattern_str = if pattern_str.starts_with('"')
                                            && pattern_str.ends_with('"')
                                        {
                                            &pattern_str[1..pattern_str.len() - 1]
                                        } else {
                                            pattern_str
                                        };
                                        let mut pattern = parse_mini_notation(pattern_str);

                                        // Apply transformations (may be chained with more |>)
                                        for transform in transform_expr.split(" |> ") {
                                            pattern = apply_transform(pattern, transform);
                                        }

                                        return Some(graph.add_node(SignalNode::Sample {
                                            pattern_str: pattern_str.to_string(),
                                            pattern,
                                            last_trigger_time: -1.0,
                                            last_cycle: -1,
                                            playback_positions: HashMap::new(),
                                            gain: Signal::Value(1.0),
                                            pan: Signal::Value(0.0),
                                            speed: Signal::Value(1.0),
                                            cut_group: Signal::Value(0.0),
                                            n: Signal::Value(0.0),
                                            note: Signal::Value(0.0),
                                            attack: Signal::Value(0.0),
                                            release: Signal::Value(0.0),
                                            envelope_type: None,
                                            unit_mode: Signal::Value(0.0), // 0 = rate mode (default)
                                            loop_enabled: Signal::Value(0.0), // 0 = no loop (default)
                                            begin: Signal::Value(0.0),
                                            end: Signal::Value(1.0),
                                        }));
                                    }
                                }
                            } else if base_expr.starts_with('"') && base_expr.ends_with('"') {
                                // Plain pattern string with transformation
                                let pattern_str = &base_expr[1..base_expr.len() - 1];
                                let mut pattern = parse_mini_notation(pattern_str);

                                for transform in transform_expr.split(" |> ") {
                                    pattern = apply_transform(pattern, transform);
                                }

                                return Some(graph.add_node(SignalNode::Pattern {
                                    pattern_str: pattern_str.to_string(),
                                    pattern,
                                    last_value: 0.0,
                                    last_trigger_time: -1.0,
                                }));
                            }
                        }
                    }

                    // Plain pattern string: "110 220"
                    if expr.starts_with('"') && expr.ends_with('"') {
                        let pattern_str = &expr[1..expr.len() - 1];
                        let pattern = parse_mini_notation(pattern_str);
                        return Some(graph.add_node(SignalNode::Pattern {
                            pattern_str: pattern_str.to_string(),
                            pattern,
                            last_value: 0.0,
                            last_trigger_time: -1.0,
                        }));
                    }

                    // Sample player
                    if expr.starts_with("s(") {
                        if let Some(start) = expr.find('(') {
                            if let Some(end) = expr.find(')') {
                                let pattern_str = expr[start + 1..end].trim();
                                // Remove quotes if present
                                let pattern_str =
                                    if pattern_str.starts_with('"') && pattern_str.ends_with('"') {
                                        &pattern_str[1..pattern_str.len() - 1]
                                    } else {
                                        pattern_str
                                    };
                                let pattern = parse_mini_notation(pattern_str);
                                return Some(graph.add_node(SignalNode::Sample {
                                    pattern_str: pattern_str.to_string(),
                                    pattern,
                                    last_trigger_time: -1.0,
                                    last_cycle: -1,
                                    playback_positions: HashMap::new(),
                                    gain: Signal::Value(1.0),
                                    pan: Signal::Value(0.0),
                                    speed: Signal::Value(1.0),
                                    cut_group: Signal::Value(0.0),
                                    n: Signal::Value(0.0),
                                    note: Signal::Value(0.0),
                                    attack: Signal::Value(0.0),
                                    release: Signal::Value(0.0),
                                    envelope_type: None,
                                    unit_mode: Signal::Value(0.0), // 0 = rate mode (default)
                                    loop_enabled: Signal::Value(0.0), // 0 = no loop (default)
                                    begin: Signal::Value(0.0),
                                    end: Signal::Value(1.0),
                                }));
                            }
                        }
                        return None;
                    }

                    if expr.starts_with("sine(")
                        || expr.starts_with("saw(")
                        || expr.starts_with("square(")
                        || expr.starts_with("noise")
                    {
                        // Oscillator parsing
                        if expr.starts_with("noise") {
                            Some(graph.add_node(SignalNode::Noise { seed: 12345 }))
                        } else {
                            let osc_type = if expr.starts_with("sine") {
                                Waveform::Sine
                            } else if expr.starts_with("saw") {
                                Waveform::Saw
                            } else if expr.starts_with("square") {
                                Waveform::Square
                            } else {
                                Waveform::Sine // Default
                            };

                            if let Some(start) = expr.find('(') {
                                if let Some(end) = expr.find(')') {
                                    let param = expr[start + 1..end].trim();
                                    let freq_signal = if param.starts_with('"')
                                        && param.ends_with('"')
                                    {
                                        let pattern_str = &param[1..param.len() - 1];
                                        let pattern = parse_mini_notation(pattern_str);
                                        let pattern_node = graph.add_node(SignalNode::Pattern {
                                            pattern_str: pattern_str.to_string(),
                                            pattern,
                                            last_value: 440.0,
                                            last_trigger_time: -1.0,
                                        });
                                        Signal::Node(pattern_node)
                                    } else if let Ok(val) = param.parse::<f32>() {
                                        Signal::Value(val)
                                    } else {
                                        Signal::Value(440.0)
                                    };

                                    Some(graph.add_node(SignalNode::Oscillator {
                                        freq: freq_signal,
                                        waveform: osc_type,
                                        phase: RefCell::new(0.0),
                                        pending_freq: RefCell::new(None),
                                        last_sample: RefCell::new(0.0),
                                    }))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                    } else if expr.starts_with("supersaw(")
                        || expr.starts_with("superkick(")
                        || expr.starts_with("superpwm(")
                        || expr.starts_with("superchip(")
                        || expr.starts_with("superfm(")
                        || expr.starts_with("supersnare(")
                        || expr.starts_with("superhat(")
                    {
                        // SuperDirt synth parsing
                        use phonon::superdirt_synths::SynthLibrary;
                        let library = SynthLibrary::with_sample_rate(44100.0);

                        // Helper to parse a synth parameter
                        let mut parse_synth_param = |param_str: &str| -> Signal {
                            let param_str = param_str.trim();
                            // Pattern: "100 200 300"
                            if param_str.starts_with('"') && param_str.ends_with('"') {
                                let pattern_str = &param_str[1..param_str.len() - 1];
                                let pattern = parse_mini_notation(pattern_str);
                                let pattern_node = graph.add_node(SignalNode::Pattern {
                                    pattern_str: pattern_str.to_string(),
                                    pattern,
                                    last_value: 440.0,
                                    last_trigger_time: -1.0,
                                });
                                Signal::Node(pattern_node)
                            }
                            // Bus reference: ~lfo
                            else if param_str.starts_with('~') {
                                if let Some(&bus_id) = buses.get(param_str) {
                                    Signal::Node(bus_id)
                                } else {
                                    Signal::Value(440.0)
                                }
                            }
                            // Number
                            else if let Ok(val) = param_str.parse::<f32>() {
                                Signal::Value(val)
                            } else {
                                Signal::Value(440.0)
                            }
                        };

                        if expr.starts_with("supersaw(") {
                            if let Some(params_str) = expr
                                .strip_prefix("supersaw(")
                                .and_then(|s| s.strip_suffix(")"))
                            {
                                let params: Vec<&str> =
                                    params_str.split(',').map(|s| s.trim()).collect();

                                // Freq can be pattern, detune and voices must be constant
                                let freq_signal = params
                                    .first()
                                    .map(|p| parse_synth_param(p))
                                    .unwrap_or(Signal::Value(110.0));
                                let detune = params.get(1).and_then(|s| s.parse::<f32>().ok());
                                let voices = params.get(2).and_then(|s| s.parse::<usize>().ok());

                                return Some(library.build_supersaw(
                                    graph,
                                    freq_signal,
                                    detune,
                                    voices,
                                ));
                            }
                        } else if expr.starts_with("superkick(") {
                            if let Some(params_str) = expr
                                .strip_prefix("superkick(")
                                .and_then(|s| s.strip_suffix(")"))
                            {
                                let params: Vec<&str> =
                                    params_str.split(',').map(|s| s.trim()).collect();

                                let freq_signal = params
                                    .first()
                                    .map(|p| parse_synth_param(p))
                                    .unwrap_or(Signal::Value(60.0));
                                let pitch_env = params.get(1).map(|p| parse_synth_param(p));
                                let sustain = params.get(2).and_then(|s| s.parse::<f32>().ok());
                                let noise = params.get(3).map(|p| parse_synth_param(p));

                                return Some(library.build_kick(
                                    graph,
                                    freq_signal,
                                    pitch_env,
                                    sustain,
                                    noise,
                                ));
                            }
                        } else if expr.starts_with("superpwm(") {
                            if let Some(params_str) = expr
                                .strip_prefix("superpwm(")
                                .and_then(|s| s.strip_suffix(")"))
                            {
                                let params: Vec<&str> =
                                    params_str.split(',').map(|s| s.trim()).collect();

                                let freq_signal = params
                                    .first()
                                    .map(|p| parse_synth_param(p))
                                    .unwrap_or(Signal::Value(110.0));
                                let pwm_rate = params.get(1).and_then(|s| s.parse::<f32>().ok());
                                let pwm_depth = params.get(2).and_then(|s| s.parse::<f32>().ok());

                                return Some(library.build_superpwm(
                                    graph,
                                    freq_signal,
                                    pwm_rate,
                                    pwm_depth,
                                ));
                            }
                        } else if expr.starts_with("superchip(") {
                            if let Some(params_str) = expr
                                .strip_prefix("superchip(")
                                .and_then(|s| s.strip_suffix(")"))
                            {
                                let params: Vec<&str> =
                                    params_str.split(',').map(|s| s.trim()).collect();

                                let freq_signal = params
                                    .first()
                                    .map(|p| parse_synth_param(p))
                                    .unwrap_or(Signal::Value(440.0));
                                let vibrato_rate =
                                    params.get(1).and_then(|s| s.parse::<f32>().ok());
                                let vibrato_depth =
                                    params.get(2).and_then(|s| s.parse::<f32>().ok());

                                return Some(library.build_superchip(
                                    graph,
                                    freq_signal,
                                    vibrato_rate,
                                    vibrato_depth,
                                ));
                            }
                        } else if expr.starts_with("superfm(") {
                            if let Some(params_str) = expr
                                .strip_prefix("superfm(")
                                .and_then(|s| s.strip_suffix(")"))
                            {
                                let params: Vec<&str> =
                                    params_str.split(',').map(|s| s.trim()).collect();

                                let freq_signal = params
                                    .first()
                                    .map(|p| parse_synth_param(p))
                                    .unwrap_or(Signal::Value(110.0));
                                let mod_ratio = params.get(1).and_then(|s| s.parse::<f32>().ok());
                                let mod_index = params.get(2).and_then(|s| s.parse::<f32>().ok());

                                return Some(library.build_superfm(
                                    graph,
                                    freq_signal,
                                    mod_ratio,
                                    mod_index,
                                ));
                            }
                        } else if expr.starts_with("supersnare(") {
                            if let Some(params_str) = expr
                                .strip_prefix("supersnare(")
                                .and_then(|s| s.strip_suffix(")"))
                            {
                                let params: Vec<&str> =
                                    params_str.split(',').map(|s| s.trim()).collect();

                                let freq_signal = params
                                    .first()
                                    .map(|p| parse_synth_param(p))
                                    .unwrap_or(Signal::Value(180.0));
                                let snappy = params.get(1).and_then(|s| s.parse::<f32>().ok());
                                let sustain = params.get(2).and_then(|s| s.parse::<f32>().ok());

                                return Some(library.build_snare(
                                    graph,
                                    freq_signal,
                                    snappy,
                                    sustain,
                                ));
                            }
                        } else if expr.starts_with("superhat(") {
                            if let Some(params_str) = expr
                                .strip_prefix("superhat(")
                                .and_then(|s| s.strip_suffix(")"))
                            {
                                let params: Vec<&str> =
                                    params_str.split(',').map(|s| s.trim()).collect();

                                let bright = params.first().and_then(|s| s.parse::<f32>().ok());
                                let sustain = params.get(1).and_then(|s| s.parse::<f32>().ok());

                                return Some(library.build_hat(graph, bright, sustain));
                            }
                        }
                        None
                    } else if expr.starts_with("reverb(")
                        || expr.starts_with("dist(")
                        || expr.starts_with("distortion(")
                        || expr.starts_with("bitcrush(")
                        || expr.starts_with("chorus(")
                    {
                        // Effects parsing
                        use phonon::superdirt_synths::SynthLibrary;
                        let library = SynthLibrary::with_sample_rate(44100.0);

                        if expr.starts_with("reverb(") {
                            if let Some(params_str) = expr
                                .strip_prefix("reverb(")
                                .and_then(|s| s.strip_suffix(")"))
                            {
                                let params: Vec<&str> =
                                    params_str.split(',').map(|s| s.trim()).collect();

                                // First param is input (bus reference or expression)
                                let input_node = if let Some(input_expr) = params.first() {
                                    parse_expression_to_node(graph, input_expr, buses)
                                        .unwrap_or_else(|| {
                                            graph.add_node(SignalNode::Constant { value: 0.0 })
                                        })
                                } else {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                };

                                let room_size = params
                                    .get(1)
                                    .and_then(|s| s.parse::<f32>().ok())
                                    .unwrap_or(0.7);
                                let damping = params
                                    .get(2)
                                    .and_then(|s| s.parse::<f32>().ok())
                                    .unwrap_or(0.5);
                                let mix = params
                                    .get(3)
                                    .and_then(|s| s.parse::<f32>().ok())
                                    .unwrap_or(0.3);

                                return Some(
                                    library.add_reverb(graph, input_node, room_size, damping, mix),
                                );
                            }
                        } else if expr.starts_with("dist(") || expr.starts_with("distortion(") {
                            let (prefix, default_drive, default_mix) = if expr.starts_with("dist(")
                            {
                                ("dist(", 3.0, 0.5)
                            } else {
                                ("distortion(", 3.0, 0.5)
                            };

                            if let Some(params_str) =
                                expr.strip_prefix(prefix).and_then(|s| s.strip_suffix(")"))
                            {
                                let params: Vec<&str> =
                                    params_str.split(',').map(|s| s.trim()).collect();

                                let input_node = if let Some(input_expr) = params.first() {
                                    parse_expression_to_node(graph, input_expr, buses)
                                        .unwrap_or_else(|| {
                                            graph.add_node(SignalNode::Constant { value: 0.0 })
                                        })
                                } else {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                };

                                let drive = params
                                    .get(1)
                                    .and_then(|s| s.parse::<f32>().ok())
                                    .unwrap_or(default_drive);
                                let mix = params
                                    .get(2)
                                    .and_then(|s| s.parse::<f32>().ok())
                                    .unwrap_or(default_mix);

                                return Some(library.add_distortion(graph, input_node, drive, mix));
                            }
                        } else if expr.starts_with("bitcrush(") {
                            if let Some(params_str) = expr
                                .strip_prefix("bitcrush(")
                                .and_then(|s| s.strip_suffix(")"))
                            {
                                let params: Vec<&str> =
                                    params_str.split(',').map(|s| s.trim()).collect();

                                let input_node = if let Some(input_expr) = params.first() {
                                    parse_expression_to_node(graph, input_expr, buses)
                                        .unwrap_or_else(|| {
                                            graph.add_node(SignalNode::Constant { value: 0.0 })
                                        })
                                } else {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                };

                                let bits = params
                                    .get(1)
                                    .and_then(|s| s.parse::<f32>().ok())
                                    .unwrap_or(4.0);
                                let rate = params
                                    .get(2)
                                    .and_then(|s| s.parse::<f32>().ok())
                                    .unwrap_or(4.0);

                                return Some(library.add_bitcrush(graph, input_node, bits, rate));
                            }
                        } else if expr.starts_with("chorus(") {
                            if let Some(params_str) = expr
                                .strip_prefix("chorus(")
                                .and_then(|s| s.strip_suffix(")"))
                            {
                                let params: Vec<&str> =
                                    params_str.split(',').map(|s| s.trim()).collect();

                                let input_node = if let Some(input_expr) = params.first() {
                                    parse_expression_to_node(graph, input_expr, buses)
                                        .unwrap_or_else(|| {
                                            graph.add_node(SignalNode::Constant { value: 0.0 })
                                        })
                                } else {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                };

                                let rate = params
                                    .get(1)
                                    .and_then(|s| s.parse::<f32>().ok())
                                    .unwrap_or(1.0);
                                let depth = params
                                    .get(2)
                                    .and_then(|s| s.parse::<f32>().ok())
                                    .unwrap_or(0.5);
                                let mix = params
                                    .get(3)
                                    .and_then(|s| s.parse::<f32>().ok())
                                    .unwrap_or(0.3);

                                return Some(
                                    library.add_chorus(graph, input_node, rate, depth, mix),
                                );
                            }
                        }
                        None
                    } else if expr.starts_with('~') {
                        buses.get(expr).copied()
                    } else {
                        None
                    }
                }

                output_node
            }

            // Note: Graph is already compiled by DslCompiler above
            // out_signal is handled by the graph's output system

            // Recalculate duration based on actual tempo from DSL file
            let final_duration = if let Some(cycle_count) = cycles {
                // Convert cycles to seconds using the tempo from the DSL
                // 1 cycle = 1/cps seconds
                cycle_count as f32 / graph.get_cps()
            } else {
                final_duration
            };

            // Generate audio
            let total_samples = (final_duration * sample_rate as f32) as usize;
            let mut output_buffer = Vec::with_capacity(total_samples);

            if realtime {
                // REALTIME MODE: Use process_buffer() like live mode for profiling
                if parallel {
                    println!("🔬 Profiling mode: Using realtime process_buffer() path WITH PARALLEL PROCESSING");
                    println!("   Cores available: {}", rayon::current_num_threads());
                } else {
                    println!("🔬 Profiling mode: Using realtime process_buffer() path (single-threaded)");
                }

                const BLOCK_SIZE: usize = 512;
                let num_blocks = (total_samples + BLOCK_SIZE - 1) / BLOCK_SIZE;

                use std::time::Instant;
                let mut total_process_time = std::time::Duration::ZERO;
                let mut min_block_time = std::time::Duration::MAX;
                let mut max_block_time = std::time::Duration::ZERO;

                if parallel {
                    // PARALLEL MODE: Process multiple blocks concurrently
                    use rayon::prelude::*;
                    use std::sync::Mutex;

                    let start = Instant::now();

                    // Create graph instances (one per thread) - no mutex needed with chunks
                    let num_threads = rayon::current_num_threads();

                    println!("   Parallel threads: {} (processing ~{} blocks each)",
                        num_threads, (num_blocks + num_threads - 1) / num_threads);

                    // Split blocks into chunks, one chunk per thread
                    let blocks_per_thread = (num_blocks + num_threads - 1) / num_threads;
                    let chunks: Vec<std::ops::Range<usize>> = (0..num_threads)
                        .map(|thread_idx| {
                            let start_block = thread_idx * blocks_per_thread;
                            let end_block = ((thread_idx + 1) * blocks_per_thread).min(num_blocks);
                            start_block..end_block
                        })
                        .filter(|chunk| !chunk.is_empty())
                        .collect();

                    // Process chunks in parallel - each thread gets its own graph and processes multiple blocks
                    let mut all_blocks: Vec<(usize, Vec<f32>, std::time::Duration)> = chunks
                        .into_par_iter()
                        .flat_map(|block_range| {
                            // Each thread gets ONE graph clone and processes ALL its blocks
                            let mut my_graph = graph.clone();
                            let mut thread_blocks = Vec::new();

                            for block_idx in block_range {
                                // Calculate block size (last block might be smaller)
                                let block_start = block_idx * BLOCK_SIZE;
                                let block_samples = (total_samples - block_start).min(BLOCK_SIZE);

                                // Seek graph to correct time position for this block
                                let block_start_sample = block_idx * BLOCK_SIZE;
                                my_graph.seek_to_sample(block_start_sample);

                                // Process this block
                                let mut block_buffer = vec![0.0f32; block_samples];
                                let block_start_time = Instant::now();
                                my_graph.process_buffer(&mut block_buffer);
                                let block_time = block_start_time.elapsed();

                                // Apply gain and clamp
                                for sample in &mut block_buffer {
                                    *sample = (*sample * gain).clamp(-1.0, 1.0);
                                }

                                thread_blocks.push((block_idx, block_buffer, block_time));
                            }

                            thread_blocks
                        })
                        .collect();

                    total_process_time = start.elapsed();

                    // Sort blocks by index to maintain correct order
                    all_blocks.sort_by_key(|(idx, _, _)| *idx);

                    // Find min/max block times and concatenate buffers
                    for (_, block_buffer, block_time) in all_blocks {
                        min_block_time = min_block_time.min(block_time);
                        max_block_time = max_block_time.max(block_time);
                        output_buffer.extend_from_slice(&block_buffer);
                    }

                } else {
                    // SEQUENTIAL MODE: Process blocks one at a time
                    for block_idx in 0..num_blocks {
                        let remaining = total_samples - output_buffer.len();
                        let block_samples = remaining.min(BLOCK_SIZE);
                        let mut block_buffer = vec![0.0f32; block_samples];

                        let start = Instant::now();
                        graph.process_buffer(&mut block_buffer);
                        let elapsed = start.elapsed();

                        total_process_time += elapsed;
                        min_block_time = min_block_time.min(elapsed);
                        max_block_time = max_block_time.max(elapsed);

                        // Apply gain and clamp
                        for sample in &mut block_buffer {
                            *sample = (*sample * gain).clamp(-1.0, 1.0);
                        }

                        output_buffer.extend_from_slice(&block_buffer);

                        // Progress reporting
                        if block_idx % 100 == 0 {
                            let progress = (output_buffer.len() as f32 / total_samples as f32) * 100.0;
                            print!("\r🔄 Rendering: {:.1}% (block {}/{}, avg: {:?}, min: {:?}, max: {:?})",
                                progress, block_idx + 1, num_blocks,
                                total_process_time / (block_idx as u32 + 1),
                                min_block_time,
                                max_block_time);
                            use std::io::Write;
                            std::io::stdout().flush().ok();
                        }
                    }
                }

                println!(); // New line after progress
                println!("⏱️  PROFILING RESULTS:");
                println!("   Total blocks:     {}", num_blocks);
                println!("   Total time:       {:?}", total_process_time);
                println!("   Avg per block:    {:?}", total_process_time / num_blocks as u32);
                println!("   Min block time:   {:?}", min_block_time);
                println!("   Max block time:   {:?}", max_block_time);
                println!("   Blocks/second:    {:.1}", num_blocks as f64 / total_process_time.as_secs_f64());

                // Calculate if realtime is achievable
                let block_duration_ms = (BLOCK_SIZE as f64 / sample_rate as f64) * 1000.0;
                let avg_block_time_ms = total_process_time.as_secs_f64() * 1000.0 / num_blocks as f64;
                let cpu_usage_percent = (avg_block_time_ms / block_duration_ms) * 100.0;

                println!("   Block duration:   {:.2} ms", block_duration_ms);
                println!("   Avg process time: {:.2} ms", avg_block_time_ms);
                println!("   CPU usage:        {:.1}%", cpu_usage_percent);
                if cpu_usage_percent > 100.0 {
                    println!("   ⚠️  CANNOT RUN IN REALTIME ({}% CPU)", cpu_usage_percent as i32);
                } else {
                    println!("   ✅ Can run in realtime with {:.1}% headroom", 100.0 - cpu_usage_percent);
                }
                println!();

            } else {
                // OFFLINE MODE: Sample-by-sample using process_sample()
                if let Some(out_node) = out_signal {
                    // Single output mode (backwards compatible with old parser)
                    graph.set_output(out_node);
                    for _ in 0..total_samples {
                        let sample = graph.process_sample();
                        output_buffer.push((sample * gain).clamp(-1.0, 1.0));
                    }
                } else {
                    // DSL Compiler mode: output is already set in the graph
                    // Try single output first (DslCompiler sets this)
                    for _ in 0..total_samples {
                        let sample = graph.process_sample();
                        output_buffer.push((sample * gain).clamp(-1.0, 1.0));
                    }

                    // Warn if no audio was produced
                    if output_buffer.iter().all(|&s| s == 0.0) {
                        println!("⚠️  No 'out' signal found or audio produced, check your DSL file");
                    }
                }
            }

            // Apply fades
            let fade_in_samples = (fade_in * sample_rate as f32) as usize;
            let fade_out_samples = (fade_out * sample_rate as f32) as usize;

            for i in 0..fade_in_samples.min(output_buffer.len()) {
                let fade = i as f32 / fade_in_samples as f32;
                output_buffer[i] *= fade;
            }

            let start = output_buffer.len().saturating_sub(fade_out_samples);
            for i in start..output_buffer.len() {
                let fade = (output_buffer.len() - i) as f32 / fade_out_samples as f32;
                output_buffer[i] *= fade;
            }

            // Calculate statistics
            let rms = (output_buffer.iter().map(|&x| x * x).sum::<f32>()
                / output_buffer.len() as f32)
                .sqrt();
            let peak = output_buffer.iter().map(|x| x.abs()).fold(0.0, f32::max);
            let dc_offset = output_buffer.iter().sum::<f32>() / output_buffer.len() as f32;

            // Write WAV file
            let spec = WavSpec {
                channels: 1,
                sample_rate,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            };

            let mut writer = WavWriter::create(&output, spec)
                .map_err(|e| format!("Failed to create WAV file: {e}"))?;

            for &sample in &output_buffer {
                let sample_i16 = (sample * 32767.0) as i16;
                writer
                    .write_sample(sample_i16)
                    .map_err(|e| format!("Failed to write sample: {e}"))?;
            }

            writer
                .finalize()
                .map_err(|e| format!("Failed to finalize WAV: {e}"))?;

            // Print statistics
            println!("Render Statistics:");
            println!("------------------");
            println!("Duration:       {final_duration:.3} seconds");
            println!("Samples:        {total_samples}");
            println!("RMS level:      {:.3} ({:.1} dB)", rms, 20.0 * rms.log10());
            println!(
                "Peak level:     {:.3} ({:.1} dB)",
                peak,
                20.0 * peak.log10()
            );
            println!("DC offset:      {dc_offset:.6}");

            println!();
            println!("✅ Successfully rendered to: {output}");

            // Show file size
            let metadata = std::fs::metadata(&output)?;
            let size_kb = metadata.len() as f32 / 1024.0;
            println!("   File size: {size_kb:.1} KB");

            // Write tap debug files if any
            let tap_files = graph.write_tap_files();
            if !tap_files.is_empty() {
                println!();
                println!("🔍 Tap recordings:");
                for file in tap_files {
                    println!("   {}", file);
                }
            }
        }

        Commands::Play {
            input,
            duration,
            sample_rate,
            gain,
        } => {
            use crate::simple_dsp_executor::render_dsp_to_audio_simple;
            use std::process::Command;

            // Read DSL code
            let dsl_code = if input.ends_with(".ph")
                || input.ends_with(".phonon")
                || input.ends_with(".dsl")
            {
                std::fs::read_to_string(&input)?
            } else if std::path::Path::new(&input).exists() {
                std::fs::read_to_string(&input)?
            } else {
                // Treat as inline DSL code
                input.clone()
            };

            // Strip comments and empty lines
            let clean_code = dsl_code
                .lines()
                .filter(|line| !line.trim().starts_with('#') && !line.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n");

            if clean_code.trim().is_empty() {
                println!("❌ No DSL code to process");
                return Ok(());
            }

            println!("🎵 Phonon Player");
            println!("================");
            println!(
                "Input:      {}",
                if input.ends_with(".ph") || input.ends_with(".phonon") || input.ends_with(".dsl") {
                    &input
                } else {
                    "<inline DSL>"
                }
            );
            println!("Duration:   {duration} seconds");
            println!("Sample rate: {sample_rate} Hz");
            println!("Gain:       {gain:.1}");
            println!();

            println!("DSL Code:");
            for line in clean_code.lines() {
                println!("  {line}");
            }
            println!();

            // Render audio
            match render_dsp_to_audio_simple(&clean_code, sample_rate as f32, duration) {
                Ok(mut buffer) => {
                    // Apply gain
                    for sample in buffer.data.iter_mut() {
                        *sample *= gain;
                    }

                    let output_path = "/tmp/phonon_play.wav";

                    match buffer.write_wav(output_path) {
                        Ok(_) => {
                            println!("✅ Audio generated!");
                            println!("   Peak: {:.3}", buffer.peak());
                            println!("   RMS: {:.3}", buffer.rms());
                            println!("   Saved to: {output_path}");

                            println!("\n🔊 Playing...");

                            // Try different players
                            let players = ["play", "aplay", "pw-play", "paplay"];
                            let mut played = false;

                            for player in &players {
                                let result = if *player == "play" {
                                    Command::new(player).arg(output_path).arg("-q").status()
                                } else {
                                    Command::new(player).arg(output_path).status()
                                };

                                if let Ok(status) = result {
                                    if status.success() {
                                        played = true;
                                        break;
                                    }
                                }
                            }

                            if !played {
                                println!("⚠️  Could not auto-play. Try:");
                                for player in &players {
                                    if *player == "play" {
                                        println!("   {player} -q {output_path}");
                                    } else {
                                        println!("   {player} {output_path}");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to save WAV: {e}");
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to generate audio: {e}");
                }
            }
        }

        Commands::Live {
            file,
            duration: _,
            pattern: _,
            port: _,
        } => {
            // Import the phonon_poll implementation
            use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

            use phonon::unified_graph::UnifiedSignalGraph;

            use std::sync::{Arc, Mutex};
            use std::time::{Duration as StdDuration, SystemTime};

            // Create file if it doesn't exist
            if !file.exists() {
                println!("Creating {}", file.display());
                let default_content = r#"# Phonon Live
# Edit and save to hear changes!

tempo 1.0
out sine(440) * 0.2
"#;
                std::fs::write(&file, default_content)?;
            }

            // Setup audio
            let host = cpal::default_host();
            let device = host
                .default_output_device()
                .ok_or("No audio output device found")?;

            let config = device.default_output_config()?;
            let sample_rate = config.sample_rate().0 as f32;

            println!("🎵 Phonon Live");
            println!("==============");
            println!("📂 Watching: {}", file.display());
            println!("🎧 Audio: {} @ {} Hz", device.name()?, sample_rate);
            println!();

            // Shared state for live reloading with ring-buffered synthesis
            //
            // Architecture:
            // 1. File watcher thread: Detects changes, swaps graph
            // 2. Background synth thread: Continuously renders samples → ring buffer
            // 3. Audio callback: Just reads from ring buffer (FAST!)
            //
            // Key insight: Audio callback doesn't synthesize, just copies pre-rendered samples
            use arc_swap::ArcSwap;
            use std::cell::RefCell;
            use ringbuf::traits::{Consumer, Observer, Producer, Split};
            use ringbuf::HeapRb;

            // Newtype wrapper to impl Send+Sync for RefCell<UnifiedSignalGraph>
            // SAFETY: Each GraphCell instance is only accessed by one thread at a time.
            struct GraphCell(RefCell<UnifiedSignalGraph>);
            unsafe impl Send for GraphCell {}
            unsafe impl Sync for GraphCell {}

            // Graph for background synthesis thread (lock-free swap)
            let graph = Arc::new(ArcSwap::from_pointee(None::<GraphCell>));

            // Ring buffer: background synth writes, audio callback reads
            // Size: 1 second of audio @ 48kHz = 48000 samples
            // Provides smooth playback even if synth thread lags briefly
            let ring_buffer_size = (sample_rate * 1.0) as usize;  // 1 second buffer
            let ring = HeapRb::<f32>::new(ring_buffer_size);
            let (mut ring_producer, mut ring_consumer) = ring.split();

            // File watching metadata (only accessed by file watcher thread, can use Mutex)
            struct FileWatchState {
                current_file: std::path::PathBuf,
                last_modified: Option<SystemTime>,
                last_content: String,
            }

            let file_state = Arc::new(Mutex::new(FileWatchState {
                current_file: file.clone(),
                last_modified: None,
                last_content: String::new(),
            }));

            // Function to parse phonon file using compositional parser
            let parse_phonon =
                |content: &str, sample_rate: f32| -> Result<UnifiedSignalGraph, String> {
                    use phonon::compositional_compiler::compile_program;
                    use phonon::compositional_parser::parse_program;

                    // Parse using compositional parser
                    match parse_program(content) {
                        Ok((_, statements)) => compile_program(statements, sample_rate),
                        Err(e) => Err(format!("Parse error: {:?}", e)),
                    }
                };

            // Initial load
            {
                if let Ok(content) = std::fs::read_to_string(&file) {
                    match parse_phonon(&content, sample_rate) {
                        Ok(new_graph) => {
                            graph.store(Arc::new(Some(GraphCell(RefCell::new(new_graph)))));
                            let mut state_lock = file_state.lock().unwrap();
                            state_lock.last_content = content;
                            println!("✅ Loaded successfully");
                        }
                        Err(e) => {
                            println!("❌ Parse error: {e}");
                        }
                    }
                }
            }

            // Background synthesis thread: continuously renders samples into ring buffer
            // This is the KEY FIX for P1.3 - synthesis happens in background, not in audio callback!
            let graph_clone_synth = Arc::clone(&graph);
            std::thread::spawn(move || {
                let mut buffer = [0.0f32; 512];  // Render in chunks of 512 samples

                loop {
                    // Check if we have space in ring buffer
                    let space = ring_producer.vacant_len();

                    if space >= buffer.len() {
                        // Render a chunk of audio
                        let graph_snapshot = graph_clone_synth.load();

                        if let Some(ref graph_cell) = **graph_snapshot {
                            // Synthesize samples using optimized buffer processing
                            graph_cell.0.borrow_mut().process_buffer(&mut buffer);

                            // Write to ring buffer
                            let written = ring_producer.push_slice(&buffer);
                            if written < buffer.len() {
                                eprintln!("⚠️  Ring buffer full, dropped {} samples", buffer.len() - written);
                            }
                        } else {
                            // No graph yet, write silence
                            ring_producer.push_slice(&buffer);
                        }
                    } else {
                        // Ring buffer is full, sleep briefly
                        std::thread::sleep(StdDuration::from_micros(100));
                    }
                }
            });

            // Audio callback: just reads from ring buffer (FAST!)
            // No synthesis, no processing, just copy pre-rendered samples
            let err_fn = |err| eprintln!("Audio stream error: {err}");

            let stream = device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Read from ring buffer - this is MUCH faster than synthesis!
                    let available = ring_consumer.occupied_len();

                    if available >= data.len() {
                        // Ring buffer has enough samples, read them
                        ring_consumer.pop_slice(data);
                    } else {
                        // Underrun: not enough samples in buffer
                        // Read what we have, fill rest with silence
                        let read = ring_consumer.pop_slice(data);
                        for sample in data[read..].iter_mut() {
                            *sample = 0.0;
                        }

                        // Only warn occasionally to avoid spam
                        static mut UNDERRUN_COUNT: usize = 0;
                        unsafe {
                            UNDERRUN_COUNT += 1;
                            if UNDERRUN_COUNT % 100 == 0 {
                                eprintln!("⚠️  Audio underrun (synth can't keep up)");
                            }
                        }
                    }
                },
                err_fn,
                None,
            )?;

            stream.play()?;

            println!("✏️  Edit {} and save to hear changes", file.display());
            println!("🎹 Press Ctrl+C to stop");
            println!();

            // Poll for changes
            loop {
                std::thread::sleep(StdDuration::from_millis(100));

                // Check for file changes
                if let Ok(metadata) = std::fs::metadata(&file) {
                    if let Ok(modified) = metadata.modified() {
                        let mut state_lock = file_state.lock().unwrap();

                        let should_reload = match state_lock.last_modified {
                            None => true,
                            Some(last) => modified > last,
                        };

                        if should_reload {
                            state_lock.last_modified = Some(modified);
                            let file_path = state_lock.current_file.clone();
                            let last_content = state_lock.last_content.clone();
                            drop(state_lock);

                            if let Ok(content) = std::fs::read_to_string(&file_path) {
                                if content != last_content {
                                    println!("🔄 Reloading...");

                                    match parse_phonon(&content, sample_rate) {
                                        Ok(new_graph) => {
                                            // Lock-free atomic swap: no audio callback blocking!
                                            graph.store(Arc::new(Some(GraphCell(RefCell::new(new_graph)))));

                                            // Update file state
                                            let mut state_lock = file_state.lock().unwrap();
                                            state_lock.last_content = content;

                                            println!("✅ Loaded successfully");
                                        }
                                        Err(e) => {
                                            println!("❌ Parse error: {e}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Commands::Repl {} => {
            use phonon::live::LiveRepl;

            println!("🎵 Phonon Live REPL");
            println!("==================");
            println!();

            let mut repl = LiveRepl::new()?;
            repl.run()?;
        }

        Commands::Edit { file, duration } => {
            use phonon::modal_editor::ModalEditor;

            let mut editor = ModalEditor::new(duration, file.clone())?;
            editor.run()?;
        }

        Commands::Test { input } => {
            println!("🧪 Phonon Test Runner");
            println!("====================");
            println!("Input: {}", input.display());
            println!();
            println!("⚠️  Test mode not yet implemented");
            println!("   This will run validation tests on DSL files");
        }

        Commands::Midi {
            pattern,
            device,
            tempo,
            duration,
            channel,
            velocity,
            list,
        } => {
            use phonon::midi_output::{note_to_midi_message, MidiOutputHandler};
            use phonon::mini_notation_v3::parse_mini_notation;

            println!("🎹 Phonon MIDI Output");
            println!("====================");

            // List devices if requested
            if list {
                let devices = MidiOutputHandler::list_devices()?;
                if devices.is_empty() {
                    println!("No MIDI devices found!");
                    println!("Please connect a MIDI device or start a virtual MIDI port.");
                } else {
                    println!("Available MIDI devices:");
                    for (i, dev) in devices.iter().enumerate() {
                        println!("  [{}] {}", i, dev.name);
                    }
                }
                return Ok(());
            }

            // Check if pattern is provided
            let Some(pattern) = pattern else {
                println!("\n⚠️  Please provide a pattern with --pattern");
                println!("   Example: phonon midi --pattern \"c4 e4 g4 c5\"");
                return Ok(());
            };

            // Parse pattern
            let pat = parse_mini_notation(&pattern);
            println!("Pattern: {pattern}");
            println!("Tempo:   {tempo} BPM");
            println!("Duration: {duration} beats");

            // Connect to MIDI device
            let mut handler = MidiOutputHandler::new()?;

            if let Some(device_name) = device {
                println!("Device:  {device_name}");
                handler.connect(&device_name)?;
            } else {
                // Try to connect to first available device
                let devices = MidiOutputHandler::list_devices()?;
                if devices.is_empty() {
                    println!("\n⚠️  No MIDI devices found!");
                    println!("   Please connect a MIDI device or start a virtual MIDI port.");
                    println!("   You can list devices with: phonon midi --list");
                    return Ok(());
                }
                let device = devices.into_iter().next().unwrap();
                println!("Device:  {} (auto-selected)", device.name);
                handler.connect_to_port(device.port)?;
            }

            println!("\n▶️  Playing pattern to MIDI...");
            println!("   Press Ctrl+C to stop\n");

            // Play pattern
            handler.play_pattern(&pat, tempo, duration, |note_str| {
                note_to_midi_message(note_str, channel, velocity)
            })?;

            println!("\n✅ Playback complete!");
        }
    }

    Ok(())
}
