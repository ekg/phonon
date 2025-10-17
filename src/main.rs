//! Phonon CLI - Command-line interface for the Phonon synthesis system

use clap::{Parser, Subcommand};
use phonon::simple_dsp_executor;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "phonon")]
#[command(about = "Phonon modular synthesis system", long_about = None)]
struct Cli {
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
                if input.ends_with(".ph") || input.ends_with(".phonon") || input.ends_with(".pho") || input.ends_with(".dsl")
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

            // Parse and render using the DSL compiler
            use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

            // Parse the DSL
            let (_, statements) = parse_dsl(&dsl_code).map_err(|e| {
                format!("Failed to parse DSL: {:?}", e)
            })?;

            // Compile to graph
            let compiler = DslCompiler::new(sample_rate as f32);
            let mut graph = compiler.compile(statements);

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
                    if trimmed.starts_with("out ") || trimmed.starts_with("out=") || trimmed.contains('=') {
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
                        let node_id = if let Some(chain_pos) = expr.find(">>").or_else(|| expr.find("<<")) {
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
                                        let pattern_str = if pattern_str.starts_with('"') && pattern_str.ends_with('"') {
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
                                                phase: 0.0,
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
                                } else if let Some(node_id) = parse_expression_to_node(graph, parts[1], buses) {
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
                fn apply_transform(pattern: phonon::pattern::Pattern<String>, transform: &str) -> phonon::pattern::Pattern<String> {
                    let transform = transform.trim();

                    // Parse transformation: fast 2, slow 2, rev, every 4 fast 2
                    if transform.starts_with("fast ") {
                        if let Ok(factor) = transform[5..].trim().parse::<f64>() {
                            return pattern.fast(factor);
                        }
                    } else if transform.starts_with("slow ") {
                        if let Ok(factor) = transform[5..].trim().parse::<f64>() {
                            return pattern.slow(factor);
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
                                let inner_transform = if inner_transform.starts_with('(') && inner_transform.ends_with(')') {
                                    &inner_transform[1..inner_transform.len() - 1]
                                } else {
                                    inner_transform
                                };
                                let inner_transform = inner_transform.to_string();
                                return pattern.every(n, move |p| apply_transform(p, &inner_transform));
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
                                        let pattern_str = if pattern_str.starts_with('"') && pattern_str.ends_with('"') {
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
                                let pattern_str = if pattern_str.starts_with('"') && pattern_str.ends_with('"') {
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
                                        phase: 0.0,
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
                        let mut parse_synth_param =
                            |param_str: &str| -> Signal {
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
                            if let Some(params_str) = expr.strip_prefix("supersaw(").and_then(|s| s.strip_suffix(")")) {
                                let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();

                                // Freq can be pattern, detune and voices must be constant
                                let freq_signal = params.first().map(|p| parse_synth_param(p)).unwrap_or(Signal::Value(110.0));
                                let detune = params.get(1).and_then(|s| s.parse::<f32>().ok());
                                let voices = params.get(2).and_then(|s| s.parse::<usize>().ok());

                                return Some(library.build_supersaw(graph, freq_signal, detune, voices));
                            }
                        } else if expr.starts_with("superkick(") {
                            if let Some(params_str) = expr.strip_prefix("superkick(").and_then(|s| s.strip_suffix(")")) {
                                let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();

                                let freq_signal = params.first().map(|p| parse_synth_param(p)).unwrap_or(Signal::Value(60.0));
                                let pitch_env = params.get(1).map(|p| parse_synth_param(p));
                                let sustain = params.get(2).and_then(|s| s.parse::<f32>().ok());
                                let noise = params.get(3).map(|p| parse_synth_param(p));

                                return Some(library.build_kick(graph, freq_signal, pitch_env, sustain, noise));
                            }
                        } else if expr.starts_with("superpwm(") {
                            if let Some(params_str) = expr.strip_prefix("superpwm(").and_then(|s| s.strip_suffix(")")) {
                                let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();

                                let freq_signal = params.first().map(|p| parse_synth_param(p)).unwrap_or(Signal::Value(110.0));
                                let pwm_rate = params.get(1).and_then(|s| s.parse::<f32>().ok());
                                let pwm_depth = params.get(2).and_then(|s| s.parse::<f32>().ok());

                                return Some(library.build_superpwm(graph, freq_signal, pwm_rate, pwm_depth));
                            }
                        } else if expr.starts_with("superchip(") {
                            if let Some(params_str) = expr.strip_prefix("superchip(").and_then(|s| s.strip_suffix(")")) {
                                let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();

                                let freq_signal = params.first().map(|p| parse_synth_param(p)).unwrap_or(Signal::Value(440.0));
                                let vibrato_rate = params.get(1).and_then(|s| s.parse::<f32>().ok());
                                let vibrato_depth = params.get(2).and_then(|s| s.parse::<f32>().ok());

                                return Some(library.build_superchip(graph, freq_signal, vibrato_rate, vibrato_depth));
                            }
                        } else if expr.starts_with("superfm(") {
                            if let Some(params_str) = expr.strip_prefix("superfm(").and_then(|s| s.strip_suffix(")")) {
                                let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();

                                let freq_signal = params.first().map(|p| parse_synth_param(p)).unwrap_or(Signal::Value(110.0));
                                let mod_ratio = params.get(1).and_then(|s| s.parse::<f32>().ok());
                                let mod_index = params.get(2).and_then(|s| s.parse::<f32>().ok());

                                return Some(library.build_superfm(graph, freq_signal, mod_ratio, mod_index));
                            }
                        } else if expr.starts_with("supersnare(") {
                            if let Some(params_str) = expr.strip_prefix("supersnare(").and_then(|s| s.strip_suffix(")")) {
                                let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();

                                let freq_signal = params.first().map(|p| parse_synth_param(p)).unwrap_or(Signal::Value(180.0));
                                let snappy = params.get(1).and_then(|s| s.parse::<f32>().ok());
                                let sustain = params.get(2).and_then(|s| s.parse::<f32>().ok());

                                return Some(library.build_snare(graph, freq_signal, snappy, sustain));
                            }
                        } else if expr.starts_with("superhat(") {
                            if let Some(params_str) = expr.strip_prefix("superhat(").and_then(|s| s.strip_suffix(")")) {
                                let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();

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
                            if let Some(params_str) = expr.strip_prefix("reverb(").and_then(|s| s.strip_suffix(")")) {
                                let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();

                                // First param is input (bus reference or expression)
                                let input_node = if let Some(input_expr) = params.first() {
                                    parse_expression_to_node(graph, input_expr, buses).unwrap_or_else(|| {
                                        graph.add_node(SignalNode::Constant { value: 0.0 })
                                    })
                                } else {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                };

                                let room_size = params.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.7);
                                let damping = params.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.5);
                                let mix = params.get(3).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.3);

                                return Some(library.add_reverb(graph, input_node, room_size, damping, mix));
                            }
                        } else if expr.starts_with("dist(") || expr.starts_with("distortion(") {
                            let (prefix, default_drive, default_mix) = if expr.starts_with("dist(") {
                                ("dist(", 3.0, 0.5)
                            } else {
                                ("distortion(", 3.0, 0.5)
                            };

                            if let Some(params_str) = expr.strip_prefix(prefix).and_then(|s| s.strip_suffix(")")) {
                                let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();

                                let input_node = if let Some(input_expr) = params.first() {
                                    parse_expression_to_node(graph, input_expr, buses).unwrap_or_else(|| {
                                        graph.add_node(SignalNode::Constant { value: 0.0 })
                                    })
                                } else {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                };

                                let drive = params.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(default_drive);
                                let mix = params.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(default_mix);

                                return Some(library.add_distortion(graph, input_node, drive, mix));
                            }
                        } else if expr.starts_with("bitcrush(") {
                            if let Some(params_str) = expr.strip_prefix("bitcrush(").and_then(|s| s.strip_suffix(")")) {
                                let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();

                                let input_node = if let Some(input_expr) = params.first() {
                                    parse_expression_to_node(graph, input_expr, buses).unwrap_or_else(|| {
                                        graph.add_node(SignalNode::Constant { value: 0.0 })
                                    })
                                } else {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                };

                                let bits = params.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(4.0);
                                let rate = params.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(4.0);

                                return Some(library.add_bitcrush(graph, input_node, bits, rate));
                            }
                        } else if expr.starts_with("chorus(") {
                            if let Some(params_str) = expr.strip_prefix("chorus(").and_then(|s| s.strip_suffix(")")) {
                                let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();

                                let input_node = if let Some(input_expr) = params.first() {
                                    parse_expression_to_node(graph, input_expr, buses).unwrap_or_else(|| {
                                        graph.add_node(SignalNode::Constant { value: 0.0 })
                                    })
                                } else {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                };

                                let rate = params.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.0);
                                let depth = params.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.5);
                                let mix = params.get(3).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.3);

                                return Some(library.add_chorus(graph, input_node, rate, depth, mix));
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
            let dsl_code = if input.ends_with(".ph") || input.ends_with(".phonon") || input.ends_with(".dsl") {
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

            use phonon::mini_notation_v3::parse_mini_notation;
            use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
            use std::collections::HashMap;
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

            // Shared state for live reloading
            struct LiveState {
                graph: Option<UnifiedSignalGraph>,
                current_file: std::path::PathBuf,
                last_modified: Option<SystemTime>,
                last_content: String,
            }

            let state = Arc::new(Mutex::new(LiveState {
                graph: None,
                current_file: file.clone(),
                last_modified: None,
                last_content: String::new(),
            }));

            // Helper to apply pattern transformations
            fn apply_transform(pattern: phonon::pattern::Pattern<String>, transform: &str) -> phonon::pattern::Pattern<String> {
                let transform = transform.trim();

                if transform.starts_with("fast ") {
                    if let Ok(factor) = transform[5..].trim().parse::<f64>() {
                        return pattern.fast(factor);
                    }
                } else if transform.starts_with("slow ") {
                    if let Ok(factor) = transform[5..].trim().parse::<f64>() {
                        return pattern.slow(factor);
                    }
                } else if transform == "rev" {
                    return pattern.rev();
                } else if transform.starts_with("every ") {
                    let rest = transform[6..].trim();
                    if let Some(space_pos) = rest.find(' ') {
                        if let Ok(n) = rest[..space_pos].parse::<i32>() {
                            let inner_transform = rest[space_pos + 1..].trim();
                            let inner_transform = if inner_transform.starts_with('(') && inner_transform.ends_with(')') {
                                &inner_transform[1..inner_transform.len() - 1]
                            } else {
                                inner_transform
                            };
                            let inner_transform = inner_transform.to_string();
                            return pattern.every(n, move |p| apply_transform(p, &inner_transform));
                        }
                    }
                }

                pattern
            }

            // Full parser implementation from phonon_poll
            fn parse_expression(
                graph: &mut UnifiedSignalGraph,
                expr: &str,
                buses: &HashMap<String, phonon::unified_graph::NodeId>,
            ) -> Option<phonon::unified_graph::NodeId> {
                let expr = expr.trim();

                // Hush and panic keywords - handle via graph methods
                if expr.starts_with("hush") || expr == "panic" {
                    // These will be handled at the top-level parser, not as expressions
                    return Some(graph.add_node(SignalNode::Constant { value: 0.0 }));
                }

                // Check for |> or <| pattern transformations
                if expr.contains(" |> ") || expr.contains(" <| ") {
                    let (base_expr, transform_expr) = if expr.contains(" |> ") {
                        let parts: Vec<&str> = expr.splitn(2, " |> ").collect();
                        if parts.len() == 2 {
                            (parts[0].trim(), parts[1].trim())
                        } else {
                            (expr, "")
                        }
                    } else {
                        // <| is reversed: transform <| base
                        let parts: Vec<&str> = expr.splitn(2, " <| ").collect();
                        if parts.len() == 2 {
                            (parts[1].trim(), parts[0].trim())
                        } else {
                            (expr, "")
                        }
                    };

                    if !transform_expr.is_empty() {
                        if base_expr.starts_with("s(") {
                            if let Some(pattern_str) = base_expr.strip_prefix("s(").and_then(|s| s.strip_suffix(")")) {
                                let pattern_str = pattern_str.trim();
                                let pattern_str = if pattern_str.starts_with('"') && pattern_str.ends_with('"') {
                                    &pattern_str[1..pattern_str.len() - 1]
                                } else {
                                    pattern_str
                                };
                                let mut pattern = parse_mini_notation(pattern_str);

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
                                }));
                            }
                        } else if base_expr.starts_with('"') && base_expr.ends_with('"') {
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

                // Pattern in quotes: "bd ~ sn ~"
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

                // Number
                if let Ok(value) = expr.parse::<f32>() {
                    return Some(graph.add_node(SignalNode::Constant { value }));
                }

                // Parse parameter helper
                let parse_parameter = |graph: &mut UnifiedSignalGraph,
                                       param_str: &str,
                                       buses: &HashMap<String, phonon::unified_graph::NodeId>,
                                       default: f32|
                 -> Signal {
                    let param_str = param_str.trim();
                    if param_str.starts_with('"') && param_str.ends_with('"') {
                        let pattern_str = &param_str[1..param_str.len() - 1];
                        let pattern = parse_mini_notation(pattern_str);
                        let pattern_node = graph.add_node(SignalNode::Pattern {
                            pattern_str: pattern_str.to_string(),
                            pattern,
                            last_value: default,
                            last_trigger_time: -1.0,
                        });
                        Signal::Node(pattern_node)
                    } else if let Some(&node_id) = buses.get(param_str) {
                        Signal::Node(node_id)
                    } else if let Ok(value) = param_str.parse::<f32>() {
                        Signal::Value(value)
                    } else {
                        Signal::Value(default)
                    }
                };

                // Oscillators
                if expr.starts_with("sine(") || expr.starts_with("sin(") {
                    if let Some(freq_str) = expr
                        .strip_prefix("sine(")
                        .or(expr.strip_prefix("sin("))
                        .and_then(|s| s.strip_suffix(")"))
                    {
                        let freq_signal = parse_parameter(graph, freq_str, buses, 440.0);
                        return Some(graph.add_node(SignalNode::Oscillator {
                            freq: freq_signal,
                            waveform: Waveform::Sine,
                            phase: 0.0,
                        }));
                    }
                }

                if expr.starts_with("saw(") {
                    if let Some(freq_str) =
                        expr.strip_prefix("saw(").and_then(|s| s.strip_suffix(")"))
                    {
                        let freq_signal = parse_parameter(graph, freq_str, buses, 110.0);
                        return Some(graph.add_node(SignalNode::Oscillator {
                            freq: freq_signal,
                            waveform: Waveform::Saw,
                            phase: 0.0,
                        }));
                    }
                }

                if expr.starts_with("square(") || expr.starts_with("sq(") {
                    if let Some(freq_str) = expr
                        .strip_prefix("square(")
                        .or(expr.strip_prefix("sq("))
                        .and_then(|s| s.strip_suffix(")"))
                    {
                        let freq_signal = parse_parameter(graph, freq_str, buses, 220.0);
                        return Some(graph.add_node(SignalNode::Oscillator {
                            freq: freq_signal,
                            waveform: Waveform::Square,
                            phase: 0.0,
                        }));
                    }
                }

                if expr.starts_with("tri(") {
                    if let Some(freq_str) =
                        expr.strip_prefix("tri(").and_then(|s| s.strip_suffix(")"))
                    {
                        let freq_signal = parse_parameter(graph, freq_str, buses, 330.0);
                        return Some(graph.add_node(SignalNode::Oscillator {
                            freq: freq_signal,
                            waveform: Waveform::Triangle,
                            phase: 0.0,
                        }));
                    }
                }

                // Scale quantization: scale("0 1 2 3", "major", "60")
                if expr.starts_with("scale(") {
                    if let Some(params_str) = expr.strip_prefix("scale(").and_then(|s| s.strip_suffix(")")) {
                        // Parse the three parameters: pattern, scale_name, root_note
                        let parts: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
                        if parts.len() >= 2 {
                            // Extract pattern string
                            let pattern_str = parts[0];
                            let pattern_str = if pattern_str.starts_with('"') && pattern_str.ends_with('"') {
                                &pattern_str[1..pattern_str.len() - 1]
                            } else {
                                pattern_str
                            };

                            // Extract scale name
                            let scale_name = if parts.len() > 1 {
                                let s = parts[1];
                                if s.starts_with('"') && s.ends_with('"') {
                                    &s[1..s.len() - 1]
                                } else {
                                    s
                                }
                            } else {
                                "major"
                            };

                            // Extract root note
                            let root_note_str = if parts.len() > 2 {
                                let r = parts[2];
                                if r.starts_with('"') && r.ends_with('"') {
                                    &r[1..r.len() - 1]
                                } else {
                                    r
                                }
                            } else {
                                "60"
                            };

                            // Convert root note to MIDI number
                            use phonon::pattern_tonal::note_to_midi;
                            let root_midi = if let Ok(midi) = root_note_str.parse::<u8>() {
                                midi
                            } else if let Some(midi) = note_to_midi(root_note_str) {
                                midi
                            } else {
                                60 // Default to C4
                            };

                            // Create ScaleQuantize node
                            let pattern = parse_mini_notation(pattern_str);
                            return Some(graph.add_node(SignalNode::ScaleQuantize {
                                pattern_str: pattern_str.to_string(),
                                pattern,
                                scale_name: scale_name.to_string(),
                                root_note: root_midi,
                                last_value: 261.63, // Default to C4 frequency
                            }));
                        }
                    }
                }

                // Noise
                if expr == "noise" {
                    return Some(graph.add_node(SignalNode::Noise { seed: 12345 }));
                }

                // Sample player: s("bd cp hh")
                if expr.starts_with("s(") {
                    if let Some(pattern_str) = expr.strip_prefix("s(").and_then(|s| s.strip_suffix(")")) {
                        let pattern_str = pattern_str.trim();
                        // Remove quotes if present
                        let pattern_str = if pattern_str.starts_with('"') && pattern_str.ends_with('"') {
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
                        }));
                    }
                }

                // Filters: lpf(input, cutoff, q) OR lpf(cutoff, q) << input
                if expr.contains(" >> lpf(") || expr.contains(" << lpf(") {
                    let (input_expr, filter_params) = if expr.contains(" >> lpf(") {
                        let parts: Vec<&str> = expr.splitn(2, " >> lpf(").collect();
                        if parts.len() == 2 && parts[1].ends_with(")") {
                            (parts[0], &parts[1][..parts[1].len() - 1])
                        } else {
                            ("", "")
                        }
                    } else {
                        // << is reversed: lpf(...) << input
                        let parts: Vec<&str> = expr.splitn(2, " << lpf(").collect();
                        if parts.len() == 2 && parts[0].starts_with("lpf(") {
                            let filter_start = parts[0].find('(').unwrap();
                            let filter_params = &parts[0][filter_start + 1..];
                            if filter_params.ends_with(")") {
                                (parts[1], &filter_params[..filter_params.len() - 1])
                            } else {
                                ("", "")
                            }
                        } else {
                            ("", "")
                        }
                    };

                    if !input_expr.is_empty() && !filter_params.is_empty() {
                        let param_parts: Vec<&str> = filter_params.split(',').map(|s| s.trim()).collect();

                        if let Some(input) = parse_expression(graph, input_expr, buses) {
                            let cutoff_signal = if let Some(cutoff_str) = param_parts.first() {
                                parse_parameter(graph, cutoff_str, buses, 1000.0)
                            } else {
                                Signal::Value(1000.0)
                            };

                            let q_signal = if let Some(q_str) = param_parts.get(1) {
                                parse_parameter(graph, q_str, buses, 1.0)
                            } else {
                                Signal::Value(1.0)
                            };

                            return Some(graph.add_node(SignalNode::LowPass {
                                input: Signal::Node(input),
                                cutoff: cutoff_signal,
                                q: q_signal,
                                state: Default::default(),
                            }));
                        }
                    }
                }

                // HPF
                if expr.contains(" >> hpf(") || expr.contains(" << hpf(") {
                    let (input_expr, filter_params) = if expr.contains(" >> hpf(") {
                        let parts: Vec<&str> = expr.splitn(2, " >> hpf(").collect();
                        if parts.len() == 2 && parts[1].ends_with(")") {
                            (parts[0], &parts[1][..parts[1].len() - 1])
                        } else {
                            ("", "")
                        }
                    } else {
                        // << is reversed: hpf(...) << input
                        let parts: Vec<&str> = expr.splitn(2, " << hpf(").collect();
                        if parts.len() == 2 && parts[0].starts_with("hpf(") {
                            let filter_start = parts[0].find('(').unwrap();
                            let filter_params = &parts[0][filter_start + 1..];
                            if filter_params.ends_with(")") {
                                (parts[1], &filter_params[..filter_params.len() - 1])
                            } else {
                                ("", "")
                            }
                        } else {
                            ("", "")
                        }
                    };

                    if !input_expr.is_empty() && !filter_params.is_empty() {
                        let param_parts: Vec<&str> = filter_params.split(',').map(|s| s.trim()).collect();

                        if let Some(input) = parse_expression(graph, input_expr, buses) {
                            let cutoff_signal = if let Some(cutoff_str) = param_parts.first() {
                                parse_parameter(graph, cutoff_str, buses, 1000.0)
                            } else {
                                Signal::Value(1000.0)
                            };

                            let q_signal = if let Some(q_str) = param_parts.get(1) {
                                parse_parameter(graph, q_str, buses, 1.0)
                            } else {
                                Signal::Value(1.0)
                            };

                            return Some(graph.add_node(SignalNode::HighPass {
                                input: Signal::Node(input),
                                cutoff: cutoff_signal,
                                q: q_signal,
                                state: Default::default(),
                            }));
                        }
                    }
                }

                // Binary operations: a * b, a + b
                if expr.contains(" * ") {
                    let parts: Vec<&str> = expr.splitn(2, " * ").collect();
                    if parts.len() == 2 {
                        if let (Some(left), Some(right)) = (
                            parse_expression(graph, parts[0], buses),
                            parse_expression(graph, parts[1], buses),
                        ) {
                            return Some(graph.add_node(SignalNode::Multiply {
                                a: Signal::Node(left),
                                b: Signal::Node(right),
                            }));
                        }
                    }
                }

                if expr.contains(" + ") {
                    let parts: Vec<&str> = expr.splitn(2, " + ").collect();
                    if parts.len() == 2 {
                        if let (Some(left), Some(right)) = (
                            parse_expression(graph, parts[0], buses),
                            parse_expression(graph, parts[1], buses),
                        ) {
                            return Some(graph.add_node(SignalNode::Add {
                                a: Signal::Node(left),
                                b: Signal::Node(right),
                            }));
                        }
                    }
                }

                // SuperDirt synths
                if expr.starts_with("supersaw(")
                    || expr.starts_with("superkick(")
                    || expr.starts_with("superpwm(")
                    || expr.starts_with("superchip(")
                    || expr.starts_with("superfm(")
                    || expr.starts_with("supersnare(")
                    || expr.starts_with("superhat(")
                {
                    use phonon::superdirt_synths::SynthLibrary;
                    let library = SynthLibrary::with_sample_rate(44100.0);

                    let mut parse_synth_param = |param_str: &str| -> Signal {
                        let param_str = param_str.trim();
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
                        } else if param_str.starts_with('~') {
                            if let Some(&bus_id) = buses.get(param_str) {
                                Signal::Node(bus_id)
                            } else {
                                Signal::Value(440.0)
                            }
                        } else if let Ok(val) = param_str.parse::<f32>() {
                            Signal::Value(val)
                        } else {
                            Signal::Value(440.0)
                        }
                    };

                    if expr.starts_with("supersaw(") {
                        if let Some(params_str) = expr.strip_prefix("supersaw(").and_then(|s| s.strip_suffix(")")) {
                            let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
                            let freq_signal = params.first().map(|p| parse_synth_param(p)).unwrap_or(Signal::Value(110.0));
                            let detune = params.get(1).and_then(|s| s.parse::<f32>().ok());
                            let voices = params.get(2).and_then(|s| s.parse::<usize>().ok());
                            return Some(library.build_supersaw(graph, freq_signal, detune, voices));
                        }
                    } else if expr.starts_with("superkick(") {
                        if let Some(params_str) = expr.strip_prefix("superkick(").and_then(|s| s.strip_suffix(")")) {
                            let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
                            let freq_signal = params.first().map(|p| parse_synth_param(p)).unwrap_or(Signal::Value(60.0));
                            let pitch_env = params.get(1).map(|p| parse_synth_param(p));
                            let sustain = params.get(2).and_then(|s| s.parse::<f32>().ok());
                            let noise = params.get(3).map(|p| parse_synth_param(p));
                            return Some(library.build_kick(graph, freq_signal, pitch_env, sustain, noise));
                        }
                    } else if expr.starts_with("superpwm(") {
                        if let Some(params_str) = expr.strip_prefix("superpwm(").and_then(|s| s.strip_suffix(")")) {
                            let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
                            let freq_signal = params.first().map(|p| parse_synth_param(p)).unwrap_or(Signal::Value(110.0));
                            let pwm_rate = params.get(1).and_then(|s| s.parse::<f32>().ok());
                            let pwm_depth = params.get(2).and_then(|s| s.parse::<f32>().ok());
                            return Some(library.build_superpwm(graph, freq_signal, pwm_rate, pwm_depth));
                        }
                    } else if expr.starts_with("superchip(") {
                        if let Some(params_str) = expr.strip_prefix("superchip(").and_then(|s| s.strip_suffix(")")) {
                            let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
                            let freq_signal = params.first().map(|p| parse_synth_param(p)).unwrap_or(Signal::Value(440.0));
                            let vibrato_rate = params.get(1).and_then(|s| s.parse::<f32>().ok());
                            let vibrato_depth = params.get(2).and_then(|s| s.parse::<f32>().ok());
                            return Some(library.build_superchip(graph, freq_signal, vibrato_rate, vibrato_depth));
                        }
                    } else if expr.starts_with("superfm(") {
                        if let Some(params_str) = expr.strip_prefix("superfm(").and_then(|s| s.strip_suffix(")")) {
                            let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
                            let freq_signal = params.first().map(|p| parse_synth_param(p)).unwrap_or(Signal::Value(110.0));
                            let mod_ratio = params.get(1).and_then(|s| s.parse::<f32>().ok());
                            let mod_index = params.get(2).and_then(|s| s.parse::<f32>().ok());
                            return Some(library.build_superfm(graph, freq_signal, mod_ratio, mod_index));
                        }
                    } else if expr.starts_with("supersnare(") {
                        if let Some(params_str) = expr.strip_prefix("supersnare(").and_then(|s| s.strip_suffix(")")) {
                            let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
                            let freq_signal = params.first().map(|p| parse_synth_param(p)).unwrap_or(Signal::Value(180.0));
                            let snappy = params.get(1).and_then(|s| s.parse::<f32>().ok());
                            let sustain = params.get(2).and_then(|s| s.parse::<f32>().ok());
                            return Some(library.build_snare(graph, freq_signal, snappy, sustain));
                        }
                    } else if expr.starts_with("superhat(") {
                        if let Some(params_str) = expr.strip_prefix("superhat(").and_then(|s| s.strip_suffix(")")) {
                            let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
                            let bright = params.first().and_then(|s| s.parse::<f32>().ok());
                            let sustain = params.get(1).and_then(|s| s.parse::<f32>().ok());
                            return Some(library.build_hat(graph, bright, sustain));
                        }
                    }
                    return None;
                }

                // Effects
                if expr.starts_with("reverb(")
                    || expr.starts_with("dist(")
                    || expr.starts_with("distortion(")
                    || expr.starts_with("bitcrush(")
                    || expr.starts_with("chorus(")
                {
                    use phonon::superdirt_synths::SynthLibrary;
                    let library = SynthLibrary::with_sample_rate(44100.0);

                    if expr.starts_with("reverb(") {
                        if let Some(params_str) = expr.strip_prefix("reverb(").and_then(|s| s.strip_suffix(")")) {
                            let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
                            let input_node = if let Some(input_expr) = params.first() {
                                parse_expression(graph, input_expr, buses).unwrap_or_else(|| {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                })
                            } else {
                                graph.add_node(SignalNode::Constant { value: 0.0 })
                            };
                            let room_size = params.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.7);
                            let damping = params.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.5);
                            let mix = params.get(3).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.3);
                            return Some(library.add_reverb(graph, input_node, room_size, damping, mix));
                        }
                    } else if expr.starts_with("dist(") || expr.starts_with("distortion(") {
                        let (prefix, default_drive, default_mix) = if expr.starts_with("dist(") {
                            ("dist(", 3.0, 0.5)
                        } else {
                            ("distortion(", 3.0, 0.5)
                        };
                        if let Some(params_str) = expr.strip_prefix(prefix).and_then(|s| s.strip_suffix(")")) {
                            let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
                            let input_node = if let Some(input_expr) = params.first() {
                                parse_expression(graph, input_expr, buses).unwrap_or_else(|| {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                })
                            } else {
                                graph.add_node(SignalNode::Constant { value: 0.0 })
                            };
                            let drive = params.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(default_drive);
                            let mix = params.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(default_mix);
                            return Some(library.add_distortion(graph, input_node, drive, mix));
                        }
                    } else if expr.starts_with("bitcrush(") {
                        if let Some(params_str) = expr.strip_prefix("bitcrush(").and_then(|s| s.strip_suffix(")")) {
                            let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
                            let input_node = if let Some(input_expr) = params.first() {
                                parse_expression(graph, input_expr, buses).unwrap_or_else(|| {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                })
                            } else {
                                graph.add_node(SignalNode::Constant { value: 0.0 })
                            };
                            let bits = params.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(4.0);
                            let rate = params.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(4.0);
                            return Some(library.add_bitcrush(graph, input_node, bits, rate));
                        }
                    } else if expr.starts_with("chorus(") {
                        if let Some(params_str) = expr.strip_prefix("chorus(").and_then(|s| s.strip_suffix(")")) {
                            let params: Vec<&str> = params_str.split(',').map(|s| s.trim()).collect();
                            let input_node = if let Some(input_expr) = params.first() {
                                parse_expression(graph, input_expr, buses).unwrap_or_else(|| {
                                    graph.add_node(SignalNode::Constant { value: 0.0 })
                                })
                            } else {
                                graph.add_node(SignalNode::Constant { value: 0.0 })
                            };
                            let rate = params.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.0);
                            let depth = params.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.5);
                            let mix = params.get(3).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.3);
                            return Some(library.add_chorus(graph, input_node, rate, depth, mix));
                        }
                    }
                    return None;
                }

                // Bus reference
                if let Some(&node_id) = buses.get(expr) {
                    return Some(node_id);
                }

                None
            }

            // Function to parse phonon file with full features
            let parse_phonon =
                |content: &str, sample_rate: f32| -> Result<UnifiedSignalGraph, String> {
                    let mut graph = UnifiedSignalGraph::new(sample_rate);
                    let mut buses: HashMap<String, phonon::unified_graph::NodeId> = HashMap::new();

                    // Default tempo
                    graph.set_cps(1.0);

                    for line in content.lines() {
                        let line = line.trim();

                        // Skip comments and empty lines
                        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                            continue;
                        }

                        // Parse tempo/cps
                        if line.starts_with("cps ") || line.starts_with("tempo ") {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 2 {
                                if let Ok(cps) = parts[1].parse::<f32>() {
                                    graph.set_cps(cps);
                                }
                            }
                        }
                        // Parse hush/panic commands
                        else if line.starts_with("hush") || line == "panic" {
                            if line == "panic" {
                                // Panic: kill all voices and silence all outputs
                                graph.panic();
                            } else if line == "hush" {
                                // Hush all outputs
                                graph.hush_all();
                            } else if line.starts_with("hush ") {
                                // Hush specific channel: "hush 1", "hush 2", etc.
                                let parts: Vec<&str> = line.split_whitespace().collect();
                                if parts.len() >= 2 {
                                    if let Ok(channel) = parts[1].parse::<usize>() {
                                        graph.hush_channel(channel);
                                    }
                                }
                            }
                        }
                        // Parse output
                        else if line.starts_with("out ") {
                            let expr = line.strip_prefix("out ").unwrap_or("").trim();
                            if let Some(node_id) = parse_expression(&mut graph, expr, &buses) {
                                let output = graph.add_node(SignalNode::Output {
                                    input: Signal::Node(node_id),
                                });
                                graph.set_output(output);
                            }
                        }
                        // Parse assignment: lfo = sine(0.5) OR out = s("bd")
                        else if line.contains('=') {
                            let parts: Vec<&str> = line.splitn(2, '=').collect();
                            if parts.len() == 2 {
                                let name = parts[0].trim();
                                let expr = parts[1].trim();

                                if let Some(node_id) = parse_expression(&mut graph, expr, &buses) {
                                    // Special case: "out = ..." should set output, not create a bus
                                    if name == "out" {
                                        let output = graph.add_node(SignalNode::Output {
                                            input: Signal::Node(node_id),
                                        });
                                        graph.set_output(output);
                                    } else {
                                        buses.insert(name.to_string(), node_id);
                                        graph.add_bus(name.to_string(), node_id);
                                    }
                                }
                            }
                        }
                    }

                    // If no output defined, create silence (not default sine wave)
                    if !graph.has_output() {
                        let silence = graph.add_node(SignalNode::Constant { value: 0.0 });
                        let output = graph.add_node(SignalNode::Output {
                            input: Signal::Node(silence),
                        });
                        graph.set_output(output);
                    }

                    Ok(graph)
                };

            // Initial load
            {
                if let Ok(content) = std::fs::read_to_string(&file) {
                    match parse_phonon(&content, sample_rate) {
                        Ok(graph) => {
                            let mut state_lock = state.lock().unwrap();
                            state_lock.graph = Some(graph);
                            state_lock.last_content = content;
                            println!("✅ Loaded successfully");
                        }
                        Err(e) => {
                            println!("❌ Parse error: {e}");
                        }
                    }
                }
            }

            // Audio callback
            let state_clone = Arc::clone(&state);
            let err_fn = |err| eprintln!("Audio stream error: {err}");

            let stream = device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut state = state_clone.lock().unwrap();

                    for sample in data.iter_mut() {
                        *sample = if let Some(ref mut graph) = state.graph {
                            graph.process_sample()
                        } else {
                            0.0
                        };
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
                        let mut state_lock = state.lock().unwrap();

                        let should_reload = match state_lock.last_modified {
                            None => true,
                            Some(last) => modified > last,
                        };

                        if should_reload {
                            state_lock.last_modified = Some(modified);
                            let file_path = state_lock.current_file.clone();
                            drop(state_lock);

                            if let Ok(content) = std::fs::read_to_string(&file_path) {
                                let mut state_lock = state.lock().unwrap();
                                if content != state_lock.last_content {
                                    state_lock.last_content = content.clone();
                                    drop(state_lock);

                                    println!("🔄 Reloading...");

                                    match parse_phonon(&content, sample_rate) {
                                        Ok(graph) => {
                                            let mut state_lock = state.lock().unwrap();
                                            state_lock.graph = Some(graph);
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
