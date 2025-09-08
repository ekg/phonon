//! Real-time audio playback for patterns

use crate::enhanced_parser::EnhancedParser;
use crate::signal_executor::SignalExecutor;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct Player {
    executor: Arc<Mutex<SignalExecutor>>,
    sample_rate: f32,
    block_size: usize,
}

impl Player {
    pub fn new(dsl_code: &str) -> Result<Self, String> {
        let sample_rate = 44100.0;
        let block_size = 512;
        
        // Parse DSL
        let mut parser = EnhancedParser::new(sample_rate);
        let graph = parser.parse(dsl_code)?;
        
        // Create executor
        let mut executor = SignalExecutor::new(graph, sample_rate, block_size);
        executor.initialize()?;
        
        Ok(Self {
            executor: Arc::new(Mutex::new(executor)),
            sample_rate,
            block_size,
        })
    }
    
    pub fn play(self, duration: Option<f32>) -> Result<(), Box<dyn std::error::Error>> {
        // Get audio output device
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or("No output device available")?;
        let config = device.default_output_config()?;
        
        println!("🎵 Playing through: {}", device.name()?);
        println!("   Sample rate: {} Hz", config.sample_rate().0);
        println!("   Channels: {}", config.channels());
        if duration.is_none() {
            println!("\nPress Ctrl+C to stop");
        }
        
        // Create audio stream based on sample format
        match config.sample_format() {
            cpal::SampleFormat::F32 => self.run_stream::<f32>(&device, &config.into(), duration)?,
            cpal::SampleFormat::I16 => self.run_stream::<i16>(&device, &config.into(), duration)?,
            cpal::SampleFormat::U16 => self.run_stream::<u16>(&device, &config.into(), duration)?,
            _ => return Err("Unsupported sample format".into()),
        }
        
        Ok(())
    }
    
    fn run_stream<T>(&self, device: &cpal::Device, config: &cpal::StreamConfig, duration: Option<f32>) -> Result<(), Box<dyn std::error::Error>>
    where
        T: SizedSample + FromSample<f32>,
    {
        let channels = config.channels as usize;
        let executor = self.executor.clone();
        let block_size = self.block_size;
        let sample_rate = config.sample_rate.0 as f32;
        
        // Calculate total samples to play if duration is specified
        let total_samples = duration.map(|d| (d * sample_rate) as usize);
        
        // Buffer for processing
        let sample_clock = Arc::new(Mutex::new(0usize));
        let sample_clock_clone = sample_clock.clone();
        
        // Store the process buffer outside the callback
        let process_buffer = Arc::new(Mutex::new(vec![0.0f32; block_size]));
        let process_buffer_clone = process_buffer.clone();
        
        // Track if we should stop playback
        let should_stop = Arc::new(Mutex::new(false));
        let should_stop_clone = should_stop.clone();
        
        // Create the audio callback
        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let mut executor = executor.lock().unwrap();
                let mut clock = sample_clock_clone.lock().unwrap();
                let mut buffer = process_buffer_clone.lock().unwrap();
                
                // Check if we should stop
                if let Some(total) = total_samples {
                    if *clock >= total {
                        *should_stop_clone.lock().unwrap() = true;
                        // Fill with silence and return
                        for sample in data.iter_mut() {
                            *sample = T::from_sample(0.0);
                        }
                        return;
                    }
                }
                
                // Process audio in chunks
                for frame in data.chunks_mut(channels) {
                    // Check again for each frame if we've reached the limit
                    if let Some(total) = total_samples {
                        if *clock >= total {
                            for sample in frame.iter_mut() {
                                *sample = T::from_sample(0.0);
                            }
                            continue;
                        }
                    }
                    
                    // Get the next sample from executor
                    let sample_idx = *clock % block_size;
                    
                    // Process a new block if needed
                    if sample_idx == 0 {
                        match executor.process_block() {
                            Ok(audio_buffer) => {
                                // Use the output buffer data
                                if audio_buffer.data.len() >= block_size {
                                    buffer.copy_from_slice(&audio_buffer.data[..block_size]);
                                } else if !audio_buffer.data.is_empty() {
                                    // Partial buffer
                                    for (i, &sample) in audio_buffer.data.iter().enumerate() {
                                        if i < block_size {
                                            buffer[i] = sample;
                                        }
                                    }
                                }
                            }
                            Err(_) => {
                                // Silent on error
                                buffer.fill(0.0);
                            }
                        }
                    }
                    
                    // Get the sample value with bounds checking
                    let value = buffer[sample_idx];
                    
                    // Apply some limiting to prevent clipping
                    let limited_value = value.max(-1.0).min(1.0);
                    
                    // Write to all channels
                    for sample in frame.iter_mut() {
                        *sample = T::from_sample(limited_value);
                    }
                    
                    *clock += 1;
                }
            },
            |err| eprintln!("Stream error: {}", err),
            None
        )?;
        
        // Start playback
        stream.play()?;
        
        // Keep the stream alive for the specified duration or until stopped
        if let Some(d) = duration {
            std::thread::sleep(Duration::from_secs_f32(d + 0.1)); // Add small buffer
        } else {
            // Wait forever (until Ctrl+C)
            std::thread::park();
        }
        
        Ok(())
    }
}

/// Play a DSL pattern directly
pub fn play_dsl(dsl_code: &str, duration: Option<f32>) -> Result<(), Box<dyn std::error::Error>> {
    let player = Player::new(dsl_code)?;
    player.play(duration)
}

/// Play a pattern string directly  
pub fn play_pattern(pattern: &str, duration: Option<f32>) -> Result<(), Box<dyn std::error::Error>> {
    // Display the pattern structure
    crate::pattern_display::print_pattern(pattern);
    println!();
    
    let dsl = format!("out: pattern(\"{}\")", pattern);
    play_dsl(&dsl, duration)
}

/// Print timing information for a pattern without playing audio
pub fn print_pattern_timing(pattern_str: &str, num_cycles: u32) -> Result<(), Box<dyn std::error::Error>> {
    use crate::mini_notation_v3::parse_mini_notation;
    use crate::pattern::{State, TimeSpan, Fraction};
    use std::collections::HashMap;
    
    // First show the pattern structure
    crate::pattern_display::print_pattern(pattern_str);
    println!();
    
    // Parse the pattern
    let pattern = parse_mini_notation(pattern_str);
    
    println!("Timing log for {} cycle(s):", num_cycles);
    println!("─────────────────────────────");
    
    // Query events for each cycle
    for cycle in 0..num_cycles {
        println!("\n📍 Cycle {}:", cycle);
        
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: {
                let mut controls = HashMap::new();
                controls.insert("_global_cycle".to_string(), cycle as f64);
                controls
            },
        };
        
        let haps = pattern.query(&state);
        
        if haps.is_empty() {
            println!("  (silence)");
        } else {
            // Sort events by start time
            let mut sorted_haps = haps.clone();
            sorted_haps.sort_by(|a, b| a.part.begin.partial_cmp(&b.part.begin).unwrap());
            
            for hap in sorted_haps {
                let start_time = hap.part.begin.to_float();
                let end_time = hap.part.end.to_float();
                let duration_ms = (end_time - start_time) * 1000.0;
                
                // Format time as cycle:beat notation
                let start_beat = (start_time % 1.0) * 4.0; // Convert to 4/4 beats
                let end_beat = (end_time % 1.0) * 4.0;
                
                println!("  {:>6.3}s - {:>6.3}s │ {:>3.0}ms │ {}",
                    start_time, end_time, duration_ms, hap.value);
                
                // Also show in musical notation if within a single cycle
                if start_time.floor() == end_time.floor() {
                    println!("         (beat {:.2} to {:.2})", start_beat, end_beat);
                }
            }
        }
    }
    
    // Summary statistics
    println!("\n📊 Summary:");
    println!("───────────");
    
    let mut event_counts: HashMap<String, usize> = HashMap::new();
    let mut total_events = 0;
    
    for cycle in 0..num_cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: {
                let mut controls = HashMap::new();
                controls.insert("_global_cycle".to_string(), cycle as f64);
                controls
            },
        };
        
        let haps = pattern.query(&state);
        total_events += haps.len();
        
        for hap in haps {
            *event_counts.entry(hap.value.clone()).or_insert(0) += 1;
        }
    }
    
    println!("Total events: {}", total_events);
    println!("Events per cycle: {:.1}", total_events as f32 / num_cycles as f32);
    
    if !event_counts.is_empty() {
        println!("\nEvent frequency:");
        let mut sorted_events: Vec<_> = event_counts.iter().collect();
        sorted_events.sort_by_key(|&(name, _)| name);
        
        for (name, count) in sorted_events {
            let percentage = (*count as f32 / total_events as f32) * 100.0;
            println!("  {} : {} times ({:.1}%)", name, count, percentage);
        }
    }
    
    Ok(())
}