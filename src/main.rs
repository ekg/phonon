#![allow(unused_assignments, unused_mut)]
//! Phonon CLI - Command-line interface for the Phonon synthesis system

#![allow(
    clippy::if_same_then_else,
    clippy::manual_strip
)]
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "phonon")]
#[command(about = "Phonon modular synthesis system", long_about = None)]
struct Cli {
    /// Number of threads for parallel processing (default: all available cores)
    #[arg(short = 't', long, default_value_t = num_cpus::get(), global = true)]
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

        /// Output stereo WAV (for pan/jux effects, default: false)
        #[arg(long, default_value = "false")]
        stereo: bool,
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

        /// Audio buffer size in samples (default: 512, range: 64-16384)
        #[arg(short, long)]
        buffer_size: Option<usize>,
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

    /// Manage VST/AU/CLAP/LV2 plugins
    Plugins {
        #[command(subcommand)]
        action: PluginAction,
    },
}

#[derive(Subcommand)]
enum PluginAction {
    /// Scan system paths for available plugins
    Scan {
        /// Force rescan (ignore cache)
        #[arg(short, long)]
        force: bool,
    },

    /// List all discovered plugins
    List {
        /// Filter by category: instrument, effect, or all
        #[arg(short, long, default_value = "all")]
        category: String,

        /// Output format: table, json, or names
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Search for plugins by name
    Search {
        /// Search query (partial match)
        query: String,
    },

    /// Show detailed info about a plugin
    Info {
        /// Plugin name (partial match)
        name: String,
    },

    /// Show plugin parameters
    Params {
        /// Plugin name (partial match)
        name: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging - redirect to file for Edit mode to prevent TUI corruption
    let is_edit_mode = matches!(cli.command, Commands::Edit { .. });
    if is_edit_mode {
        // Redirect tracing to a log file to prevent TUI corruption
        
        let log_file = std::fs::File::create("/tmp/phonon_audio_errors.log")
            .unwrap_or_else(|_| std::fs::File::create("/dev/null").unwrap());
        tracing_subscriber::fmt()
            .with_writer(std::sync::Mutex::new(log_file))
            .with_ansi(false)
            .init();
    } else {
        tracing_subscriber::fmt::init();
    }

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
            block_size: _,
            realtime,
            parallel,
            stereo,
        } => {
            use hound::{SampleFormat, WavSpec, WavWriter};
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

            // Parse and compile using compositional parser (supports $ and # and new transform bus syntax)
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

            // Compile to graph using compositional compiler
            let mut graph = compile_program(statements, sample_rate as f32, None)
                .map_err(|e| format!("Compile error: {}", e))?;

            // Print auto-routing info if it happened
            if graph.has_output() && !graph.get_all_bus_names().is_empty() {
                let bus_count = graph.get_all_bus_names().len();
                println!("🔀 Auto-routing: Mixing {} buses to output", bus_count);
            }

            let _buses: HashMap<String, phonon::unified_graph::NodeId> = HashMap::new();
            let mut out_signal = None;
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
            let mut left_buffer: Vec<f32> = Vec::new();
            let mut right_buffer: Vec<f32> = Vec::new();

            if stereo {
                // STEREO MODE: Sample-by-sample for proper pan/jux stereo output
                println!("🔊 Stereo mode: Using process_sample_stereo() for pan/jux separation");

                if let Some(out_node) = out_signal {
                    graph.set_output(out_node);
                }

                left_buffer = Vec::with_capacity(total_samples);
                right_buffer = Vec::with_capacity(total_samples);

                for i in 0..total_samples {
                    let (l, r) = graph.process_sample_stereo();
                    left_buffer.push((l * gain).clamp(-1.0, 1.0));
                    right_buffer.push((r * gain).clamp(-1.0, 1.0));

                    // Progress every ~1 second
                    if i % sample_rate as usize == 0 && i > 0 {
                        let progress = (i as f32 / total_samples as f32) * 100.0;
                        print!("\r🔄 Rendering stereo: {:.1}%", progress);
                        use std::io::Write;
                        std::io::stdout().flush().ok();
                    }
                }
                println!();
            } else if realtime {
                // REALTIME MODE: Use process_buffer() like live mode for profiling
                const BLOCK_SIZE: usize = 512;
                let num_blocks = total_samples.div_ceil(BLOCK_SIZE);

                // Check if graph contains effects that need sequential processing
                let needs_sequential = graph.has_sequential_dependencies();

                // Force sequential mode for graphs with reverb/delay
                // These effects have block-to-block state dependencies that cannot be parallelized
                let use_parallel = parallel && !needs_sequential;

                if use_parallel {
                    println!("🔬 Profiling mode: Using realtime process_buffer() path WITH PARALLEL PROCESSING");
                    println!("   Cores available: {}", rayon::current_num_threads());
                } else if parallel && needs_sequential {
                    println!("🔬 Profiling mode: Sequential (reverb/delay effects require ordered processing)");
                } else {
                    println!(
                        "🔬 Profiling mode: Using realtime process_buffer() path (single-threaded)"
                    );
                }

                use std::time::Instant;
                let mut total_process_time = std::time::Duration::ZERO;
                let mut min_block_time = std::time::Duration::MAX;
                let mut max_block_time = std::time::Duration::ZERO;

                if use_parallel {
                    // PURE PARALLEL MODE: No sequential effects
                    use rayon::prelude::*;

                    let start = Instant::now();
                    let num_threads = rayon::current_num_threads();

                    println!(
                        "   Parallel threads: {} (processing ~{} blocks each)",
                        num_threads,
                        num_blocks.div_ceil(num_threads)
                    );

                    // Split blocks into chunks, one chunk per thread
                    let blocks_per_thread = num_blocks.div_ceil(num_threads);
                    let chunks: Vec<std::ops::Range<usize>> = (0..num_threads)
                        .map(|thread_idx| {
                            let start_block = thread_idx * blocks_per_thread;
                            let end_block = ((thread_idx + 1) * blocks_per_thread).min(num_blocks);
                            start_block..end_block
                        })
                        .filter(|chunk| !chunk.is_empty())
                        .collect();

                    // Pre-clone graphs for parallel processing
                    let graph_clones: Vec<_> = chunks.iter().map(|_| graph.clone()).collect();

                    // Process chunks in parallel
                    let mut all_blocks: Vec<(usize, Vec<f32>, std::time::Duration)> = chunks
                        .into_par_iter()
                        .zip(graph_clones.into_par_iter())
                        .flat_map(|(block_range, mut my_graph)| {
                            let mut thread_blocks = Vec::new();

                            for block_idx in block_range {
                                let block_start = block_idx * BLOCK_SIZE;
                                let block_samples = (total_samples - block_start).min(BLOCK_SIZE);

                                my_graph.seek_to_sample(block_idx * BLOCK_SIZE);
                                // CRITICAL: process_buffer expects STEREO (interleaved L/R), so 2x size
                                let mut stereo_buffer = vec![0.0f32; block_samples * 2];
                                let block_start_time = Instant::now();
                                my_graph.process_buffer(&mut stereo_buffer);
                                let block_time = block_start_time.elapsed();

                                // Extract mono (left channel) and apply gain
                                let mut mono_buffer = Vec::with_capacity(block_samples);
                                for i in 0..block_samples {
                                    let mono = stereo_buffer[i * 2]; // Left channel
                                    mono_buffer.push((mono * gain).clamp(-1.0, 1.0));
                                }

                                thread_blocks.push((block_idx, mono_buffer, block_time));
                            }

                            thread_blocks
                        })
                        .collect();

                    total_process_time = start.elapsed();

                    // Sort blocks by index to maintain correct order
                    all_blocks.sort_by_key(|(idx, _, _)| *idx);

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
                        // CRITICAL: process_buffer expects STEREO (interleaved L/R), so 2x size
                        let mut stereo_buffer = vec![0.0f32; block_samples * 2];

                        let start = Instant::now();
                        graph.process_buffer(&mut stereo_buffer);
                        let elapsed = start.elapsed();

                        total_process_time += elapsed;
                        min_block_time = min_block_time.min(elapsed);
                        max_block_time = max_block_time.max(elapsed);

                        // Extract mono (left channel) and apply gain
                        for i in 0..block_samples {
                            let mono = stereo_buffer[i * 2]; // Left channel
                            output_buffer.push((mono * gain).clamp(-1.0, 1.0));
                        }

                        // Progress reporting
                        if block_idx % 100 == 0 {
                            let progress =
                                (output_buffer.len() as f32 / total_samples as f32) * 100.0;
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

                // Apply zero-crossing crossfade at block boundaries in the final output.
                // Independent graph clones in parallel mode can produce discontinuities
                // at block edges. Scan for large jumps and smooth them with a short fade.
                {
                    const BLOCK_SIZE_XFADE: usize = 512;
                    const FADE_SAMPLES: usize = 32;
                    const CLICK_THRESHOLD: f32 = 0.1;

                    let len = output_buffer.len();
                    let mut block_start = BLOCK_SIZE_XFADE;
                    while block_start < len {
                        if block_start > 0 {
                            let prev = output_buffer[block_start - 1];
                            let curr = output_buffer[block_start];
                            let delta = (curr - prev).abs();
                            if delta > CLICK_THRESHOLD {
                                // Apply short crossfade: ramp from prev value to actual value
                                let fade_len = FADE_SAMPLES.min(len - block_start);
                                for i in 0..fade_len {
                                    let t = (i + 1) as f32 / (fade_len + 1) as f32;
                                    // Blend: at i=0, mostly prev; at i=fade_len-1, mostly actual
                                    output_buffer[block_start + i] =
                                        prev * (1.0 - t) + output_buffer[block_start + i] * t;
                                }
                            }
                        }
                        block_start += BLOCK_SIZE_XFADE;
                    }
                }

                println!(); // New line after progress
                println!("⏱️  PROFILING RESULTS:");
                println!("   Total blocks:     {}", num_blocks);
                println!("   Total time:       {:?}", total_process_time);
                println!(
                    "   Avg per block:    {:?}",
                    total_process_time / num_blocks as u32
                );
                println!("   Min block time:   {:?}", min_block_time);
                println!("   Max block time:   {:?}", max_block_time);
                println!(
                    "   Blocks/second:    {:.1}",
                    num_blocks as f64 / total_process_time.as_secs_f64()
                );

                // Calculate if realtime is achievable
                let block_duration_ms = (BLOCK_SIZE as f64 / sample_rate as f64) * 1000.0;
                let avg_block_time_ms =
                    total_process_time.as_secs_f64() * 1000.0 / num_blocks as f64;
                let cpu_usage_percent = (avg_block_time_ms / block_duration_ms) * 100.0;

                println!("   Block duration:   {:.2} ms", block_duration_ms);
                println!("   Avg process time: {:.2} ms", avg_block_time_ms);
                println!("   CPU usage:        {:.1}%", cpu_usage_percent);
                if cpu_usage_percent > 100.0 {
                    println!(
                        "   ⚠️  CANNOT RUN IN REALTIME ({}% CPU)",
                        cpu_usage_percent as i32
                    );
                } else {
                    println!(
                        "   ✅ Can run in realtime with {:.1}% headroom",
                        100.0 - cpu_usage_percent
                    );
                }
                println!();
            } else {
                // OFFLINE MODE: Sample-by-sample using process_sample()
                if let Some(out_node) = out_signal {
                    // Single output mode (backwards compatible with old parser)
                    graph.set_output(out_node);
                }
                // DSL Compiler mode: output is already set in the graph
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

            if stereo {
                // Apply fades to stereo buffers
                for i in 0..fade_in_samples.min(left_buffer.len()) {
                    let fade = i as f32 / fade_in_samples as f32;
                    left_buffer[i] *= fade;
                    right_buffer[i] *= fade;
                }

                let start = left_buffer.len().saturating_sub(fade_out_samples);
                for i in start..left_buffer.len() {
                    let fade = (left_buffer.len() - i) as f32 / fade_out_samples as f32;
                    left_buffer[i] *= fade;
                    right_buffer[i] *= fade;
                }
            } else {
                // Apply fades to mono buffer
                for i in 0..fade_in_samples.min(output_buffer.len()) {
                    let fade = i as f32 / fade_in_samples as f32;
                    output_buffer[i] *= fade;
                }

                let start = output_buffer.len().saturating_sub(fade_out_samples);
                for i in start..output_buffer.len() {
                    let fade = (output_buffer.len() - i) as f32 / fade_out_samples as f32;
                    output_buffer[i] *= fade;
                }
            }

            // Calculate statistics
            let (rms, peak, dc_offset) = if stereo {
                let rms_left = (left_buffer.iter().map(|&x| x * x).sum::<f32>()
                    / left_buffer.len() as f32)
                    .sqrt();
                let rms_right = (right_buffer.iter().map(|&x| x * x).sum::<f32>()
                    / right_buffer.len() as f32)
                    .sqrt();
                let rms = (rms_left + rms_right) / 2.0;
                let peak_left = left_buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
                let peak_right = right_buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
                let peak = peak_left.max(peak_right);
                let dc_left = left_buffer.iter().sum::<f32>() / left_buffer.len() as f32;
                let dc_right = right_buffer.iter().sum::<f32>() / right_buffer.len() as f32;
                let dc_offset = (dc_left + dc_right) / 2.0;
                (rms, peak, dc_offset)
            } else {
                let rms = (output_buffer.iter().map(|&x| x * x).sum::<f32>()
                    / output_buffer.len() as f32)
                    .sqrt();
                let peak = output_buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
                let dc_offset = output_buffer.iter().sum::<f32>() / output_buffer.len() as f32;
                (rms, peak, dc_offset)
            };

            // Write WAV file
            let spec = WavSpec {
                channels: if stereo { 2 } else { 1 },
                sample_rate,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            };

            let mut writer = WavWriter::create(&output, spec)
                .map_err(|e| format!("Failed to create WAV file: {e}"))?;

            if stereo {
                // Write interleaved stereo samples
                for i in 0..left_buffer.len() {
                    let left_i16 = (left_buffer[i] * 32767.0) as i16;
                    let right_i16 = (right_buffer[i] * 32767.0) as i16;
                    writer
                        .write_sample(left_i16)
                        .map_err(|e| format!("Failed to write sample: {e}"))?;
                    writer
                        .write_sample(right_i16)
                        .map_err(|e| format!("Failed to write sample: {e}"))?;
                }
            } else {
                // Write mono samples
                for &sample in &output_buffer {
                    let sample_i16 = (sample * 32767.0) as i16;
                    writer
                        .write_sample(sample_i16)
                        .map_err(|e| format!("Failed to write sample: {e}"))?;
                }
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
            use hound::{SampleFormat, WavSpec, WavWriter};
            use phonon::compositional_compiler::compile_program;
            use phonon::compositional_parser::parse_program;
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

            // Parse using compositional_parser (supports vst, $ and # syntax)
            let (remaining, statements) =
                parse_program(&dsl_code).map_err(|e| format!("Failed to parse DSL: {:?}", e))?;

            if !remaining.trim().is_empty() {
                use phonon::error_diagnostics::{
                    check_for_common_mistakes, diagnose_parse_failure,
                };
                let diagnostic = diagnose_parse_failure(&dsl_code, remaining);
                eprintln!("{}", diagnostic);
                let warnings = check_for_common_mistakes(&dsl_code);
                if !warnings.is_empty() {
                    eprintln!("⚠️  Additional warnings:");
                    for warning in warnings {
                        eprintln!("  • {}", warning);
                    }
                }
            }

            // Compile to graph using compositional compiler
            let mut graph = compile_program(statements, sample_rate as f32, None)
                .map_err(|e| format!("Compile error: {}", e))?;

            // Calculate samples
            let num_samples = (duration * sample_rate as f32) as usize;

            // Render audio
            let buffer = graph.render(num_samples);

            // Apply gain and calculate stats
            let mut peak: f32 = 0.0;
            let mut sum_sq: f32 = 0.0;
            let samples: Vec<f32> = buffer
                .iter()
                .map(|&s: &f32| {
                    let sample: f32 = s * gain;
                    peak = peak.max(sample.abs());
                    sum_sq += sample * sample;
                    sample
                })
                .collect();
            let rms = (sum_sq / samples.len() as f32).sqrt();

            // Write WAV
            let output_path = "/tmp/phonon_play.wav";
            let spec = WavSpec {
                channels: 1,
                sample_rate,
                bits_per_sample: 32,
                sample_format: SampleFormat::Float,
            };

            let mut writer = WavWriter::create(output_path, spec)?;
            for sample in &samples {
                writer.write_sample(*sample)?;
            }
            writer.finalize()?;

            println!("✅ Audio generated!");
            println!("   Peak: {:.3}", peak);
            println!("   RMS: {:.3}", rms);
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
            use ringbuf::traits::{Consumer, Observer, Producer, Split};
            use ringbuf::HeapRb;
            use std::cell::RefCell;

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
            let ring_buffer_size = (sample_rate * 1.0) as usize; // 1 second buffer
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
                        Ok((_, statements)) => compile_program(statements, sample_rate, None),
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
                let mut buffer = [0.0f32; 512]; // Render in chunks of 512 samples

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
                                eprintln!(
                                    "⚠️  Ring buffer full, dropped {} samples",
                                    buffer.len() - written
                                );
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
                            if UNDERRUN_COUNT.is_multiple_of(100) {
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
                                            graph.store(Arc::new(Some(GraphCell(RefCell::new(
                                                new_graph,
                                            )))));

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

        Commands::Edit { file, duration, buffer_size } => {
            use phonon::modal_editor::ModalEditor;

            let mut editor = ModalEditor::new(duration, file.clone(), buffer_size)?;
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

        Commands::Plugins { action } => {
            use phonon::plugin_host::{PluginCategory, PluginRegistry};
            use std::path::PathBuf;

            // Get cache path
            let cache_path = dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("phonon")
                .join("plugin_cache.json");

            // Create cache directory if needed
            if let Some(parent) = cache_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }

            match action {
                PluginAction::Scan { force } => {
                    println!("🔍 Scanning for plugins...");

                    let mut registry = PluginRegistry::with_cache(cache_path.clone());

                    // Load existing cache unless force rescan
                    if !force {
                        if let Ok(count) = registry.load_cache(&cache_path) {
                            if count > 0 {
                                println!("   Loaded {} plugins from cache", count);
                            }
                        }
                    }

                    // Scan for new plugins
                    match registry.scan() {
                        Ok(count) => {
                            println!("✅ Found {} plugins", count);
                            if count > 0 {
                                println!("   Cache saved to: {}", cache_path.display());
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Scan failed: {}", e);
                        }
                    }
                }

                PluginAction::List { category, format } => {
                    let mut registry = PluginRegistry::with_cache(cache_path.clone());

                    // Load from cache
                    if registry.load_cache(&cache_path).is_err() {
                        println!("⚠️  No plugin cache found. Run 'phonon plugins scan' first.");
                        return Ok(());
                    }

                    // Filter by category
                    let plugins: Vec<_> = match category.to_lowercase().as_str() {
                        "instrument" | "instruments" | "synth" | "synths" => {
                            registry.list_by_category(PluginCategory::Instrument)
                        }
                        "effect" | "effects" | "fx" => {
                            registry.list_by_category(PluginCategory::Effect)
                        }
                        _ => registry.list(),
                    };

                    if plugins.is_empty() {
                        println!("No plugins found. Run 'phonon plugins scan' to discover plugins.");
                        return Ok(());
                    }

                    match format.to_lowercase().as_str() {
                        "json" => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&plugins).unwrap_or_default()
                            );
                        }
                        "names" => {
                            for plugin in &plugins {
                                println!("{}", plugin.id.name);
                            }
                        }
                        _ => {
                            // Table format
                            println!(
                                "\n{:<30} {:<12} {:<15} Vendor",
                                "Name", "Format", "Category"
                            );
                            println!("{}", "-".repeat(75));
                            for plugin in &plugins {
                                println!(
                                    "{:<30} {:<12} {:<15} {}",
                                    truncate_string(&plugin.id.name, 28),
                                    format!("{}", plugin.id.format),
                                    format!("{:?}", plugin.category),
                                    plugin.vendor
                                );
                            }
                            println!("\nTotal: {} plugins", plugins.len());
                        }
                    }
                }

                PluginAction::Search { query } => {
                    let mut registry = PluginRegistry::with_cache(cache_path.clone());

                    if registry.load_cache(&cache_path).is_err() {
                        println!("⚠️  No plugin cache found. Run 'phonon plugins scan' first.");
                        return Ok(());
                    }

                    let results = registry.search(&query);

                    if results.is_empty() {
                        println!("No plugins matching '{}'", query);
                    } else {
                        println!("Found {} plugins matching '{}':\n", results.len(), query);
                        for plugin in results {
                            println!(
                                "  {} ({:?}) - {}",
                                plugin.id.name, plugin.id.format, plugin.vendor
                            );
                        }
                    }
                }

                PluginAction::Info { name } => {
                    let mut registry = PluginRegistry::with_cache(cache_path.clone());

                    if registry.load_cache(&cache_path).is_err() {
                        println!("⚠️  No plugin cache found. Run 'phonon plugins scan' first.");
                        return Ok(());
                    }

                    match registry.find(&name) {
                        Some(plugin) => {
                            println!("\n🔌 Plugin: {}", plugin.id.name);
                            println!("{}", "=".repeat(40));
                            println!("Format:     {}", plugin.id.format);
                            println!("Vendor:     {}", plugin.vendor);
                            println!("Version:    {}", plugin.version);
                            println!("Category:   {:?}", plugin.category);
                            println!("Inputs:     {}", plugin.num_inputs);
                            println!("Outputs:    {}", plugin.num_outputs);
                            println!("Parameters: {}", plugin.parameters.len());
                            println!("Has GUI:    {}", plugin.has_gui);
                            println!("Path:       {}", plugin.path);

                            if !plugin.factory_presets.is_empty() {
                                println!("\nFactory Presets ({}):", plugin.factory_presets.len());
                                for preset in plugin.factory_presets.iter().take(10) {
                                    println!("  - {}", preset);
                                }
                                if plugin.factory_presets.len() > 10 {
                                    println!("  ... and {} more", plugin.factory_presets.len() - 10);
                                }
                            }
                        }
                        None => {
                            println!("Plugin '{}' not found", name);
                            println!("Try 'phonon plugins search {}' to find matches", name);
                        }
                    }
                }

                PluginAction::Params { name } => {
                    let mut registry = PluginRegistry::with_cache(cache_path.clone());

                    if registry.load_cache(&cache_path).is_err() {
                        println!("⚠️  No plugin cache found. Run 'phonon plugins scan' first.");
                        return Ok(());
                    }

                    match registry.find(&name) {
                        Some(plugin) => {
                            if plugin.parameters.is_empty() {
                                println!("Plugin '{}' has no parameters", plugin.id.name);
                            } else {
                                println!(
                                    "\n🎛️  Parameters for {} ({} total):",
                                    plugin.id.name,
                                    plugin.parameters.len()
                                );
                                println!("{}", "=".repeat(60));
                                println!(
                                    "{:<5} {:<25} {:<10} {:<8} Unit",
                                    "ID", "Name", "Default", "Range"
                                );
                                println!("{}", "-".repeat(60));

                                for param in &plugin.parameters {
                                    println!(
                                        "{:<5} {:<25} {:<10.3} {:.1}-{:.1}  {}",
                                        param.index,
                                        truncate_string(&param.name, 23),
                                        param.default_value,
                                        param.min_value,
                                        param.max_value,
                                        param.unit
                                    );
                                }

                                println!("\nUsage in Phonon:");
                                println!(
                                    "  ~synth $ vst \"{}\" # {} 0.5",
                                    plugin.id.name,
                                    plugin
                                        .parameters
                                        .first()
                                        .map(|p| p.name.to_lowercase().replace(" ", "_"))
                                        .unwrap_or_else(|| "param".to_string())
                                );
                            }
                        }
                        None => {
                            println!("Plugin '{}' not found", name);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Truncate string to max length with ellipsis
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
