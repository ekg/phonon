//! Real-time audio output using cpal
//! Works with JACK, ALSA, OpenSL ES (Android/Termux), etc.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tracing::{error, info};

pub struct AudioEngine {
    sample_rate: u32,
    mixer: Arc<Mutex<Mixer>>,
    _stream: cpal::Stream,
}

struct Mixer {
    voices: Vec<Voice>,
    pending_samples: VecDeque<PlayCommand>,
}

struct Voice {
    samples: Vec<f32>,
    position: usize,
    speed: f32,
    active: bool,
}

pub struct PlayCommand {
    pub samples: Vec<f32>,
    pub speed: f32,
}

impl AudioEngine {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Get the default audio host (JACK/ALSA/OpenSL ES/etc)
        let host = cpal::default_host();
        info!("Audio host: {:?}", host.id());

        // Get default output device
        let device = host
            .default_output_device()
            .ok_or("No audio output device found")?;
        info!("Audio device: {}", device.name()?);

        // Get default output config
        let config = device.default_output_config()?;
        info!("Audio config: {:?}", config);

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        let mixer = Arc::new(Mutex::new(Mixer {
            voices: Vec::new(),
            pending_samples: VecDeque::new(),
        }));

        let mixer_clone = mixer.clone();

        // Build the output stream
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                Self::build_stream::<f32>(&device, &config.into(), mixer_clone, channels)
            }
            cpal::SampleFormat::I16 => {
                Self::build_stream::<i16>(&device, &config.into(), mixer_clone, channels)
            }
            cpal::SampleFormat::U16 => {
                Self::build_stream::<u16>(&device, &config.into(), mixer_clone, channels)
            }
            _ => return Err("Unsupported sample format".into()),
        }?;

        // Start audio stream
        stream.play()?;
        info!("Audio stream started at {} Hz", sample_rate);

        Ok(Self {
            sample_rate,
            mixer,
            _stream: stream,
        })
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        mixer: Arc<Mutex<Mixer>>,
        channels: usize,
    ) -> Result<cpal::Stream, Box<dyn std::error::Error>>
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let mut mixer = mixer.lock().unwrap();
                mixer.process_audio(data, channels);
            },
            |err| error!("Audio stream error: {}", err),
            None,
        )?;

        Ok(stream)
    }

    pub fn play_sample(&self, samples: Vec<f32>, speed: f32) {
        let mut mixer = self.mixer.lock().unwrap();
        mixer
            .pending_samples
            .push_back(PlayCommand { samples, speed });
    }

    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl Mixer {
    fn process_audio<T>(&mut self, output: &mut [T], channels: usize)
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        // Add any pending samples as new voices
        while let Some(cmd) = self.pending_samples.pop_front() {
            // Find an inactive voice or create a new one
            let voice_idx = self.voices.iter().position(|v| !v.active);

            let voice = if let Some(idx) = voice_idx {
                &mut self.voices[idx]
            } else {
                self.voices.push(Voice {
                    samples: Vec::new(),
                    position: 0,
                    speed: 1.0,
                    active: false,
                });
                self.voices.last_mut().unwrap()
            };

            voice.samples = cmd.samples;
            voice.position = 0;
            voice.speed = cmd.speed;
            voice.active = true;
        }

        // Clear output buffer
        for sample in output.iter_mut() {
            *sample = T::from_sample(0.0);
        }

        // Mix all active voices
        for frame in output.chunks_mut(channels) {
            let mut mixed = 0.0f32;

            for voice in &mut self.voices {
                if !voice.active {
                    continue;
                }

                // Get the sample at current position
                let pos = voice.position as f32 * voice.speed;
                let idx = pos as usize;

                if idx >= voice.samples.len() {
                    voice.active = false;
                    continue;
                }

                // Simple linear interpolation for speed changes
                let sample = if voice.speed != 1.0 && idx + 1 < voice.samples.len() {
                    let frac = pos - idx as f32;
                    voice.samples[idx] * (1.0 - frac) + voice.samples[idx + 1] * frac
                } else {
                    voice.samples[idx]
                };

                mixed += sample;
                voice.position += 1;
            }

            // Soft clipping to prevent distortion
            mixed = mixed.tanh() * 0.8;

            // Write to all channels (mono -> stereo/multi-channel)
            for channel in frame.iter_mut() {
                *channel = T::from_sample(mixed);
            }
        }
    }
}
