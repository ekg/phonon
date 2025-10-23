#![allow(unused_assignments, unused_mut)]
//! SuperDirt-inspired synthesizer definitions
//!
//! This module provides a library of predefined synthesizers inspired by
//! SuperCollider's SuperDirt, designed for live coding and pattern-based music.
//!
//! # Available Synths
//!
//! ## Drums
//! - `superkick` - Kick drum with pitch envelope and sine/noise blend
//! - `supersnare` - Snare drum with filtered noise and pitch envelope
//! - `superhat` - Hi-hat with filtered noise burst
//! - `superclap` - Hand clap with multiple noise bursts
//!
//! ## Melodic
//! - `supersaw` - Detuned saw waves for thick, rich sounds
//! - `superpwm` - Pulse width modulation synthesis
//! - `supersquare` - Square wave with envelope shaping
//! - `superchip` - Chiptune-style square wave with vibrato
//! - `superfm` - 2-operator FM synthesis
//!
//! ## Bass
//! - `superbass` - Deep bass with sub-oscillator
//! - `superreese` - Reese-style bass with detuned saws
//!
//! # Usage
//!
//! ```rust
//! use phonon::superdirt_synths::SynthLibrary;
//! use phonon::unified_graph::{UnifiedSignalGraph, Signal};
//!
//! let mut graph = UnifiedSignalGraph::new(44100.0);
//! let library = SynthLibrary::new();
//!
//! // Add a superkick at 60 Hz
//! let kick = library.build_kick(&mut graph, Signal::Value(60.0), None, None, None);
//! graph.set_output(kick);
//! ```

use crate::unified_graph::{
    EnvState, FilterState, NodeId, Signal, SignalNode, UnifiedSignalGraph, Waveform,
};

/// Library of predefined synthesizers
pub struct SynthLibrary {
    sample_rate: f32,
}

impl SynthLibrary {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
        }
    }

    pub fn with_sample_rate(sample_rate: f32) -> Self {
        Self { sample_rate }
    }

    /// Build a SuperKick synthesizer
    ///
    /// Classic kick drum with:
    /// - Pitch envelope (high to low)
    /// - Sine wave oscillator
    /// - Optional noise layer
    /// - Amplitude envelope
    ///
    /// # Parameters
    /// - `freq`: Base frequency (typically 40-80 Hz)
    /// - `pitch_env`: Pitch envelope amount (0.0-1.0, default 0.5)
    /// - `sustain`: Sustain time (default 0.3)
    /// - `noise`: Noise layer amount (0.0-1.0, default 0.1)
    pub fn build_kick(
        &self,
        graph: &mut UnifiedSignalGraph,
        freq: Signal,
        pitch_env: Option<Signal>,
        sustain: Option<f32>,
        noise_amt: Option<Signal>,
    ) -> NodeId {
        let pitch_env = pitch_env.unwrap_or(Signal::Value(0.5));
        let sustain = sustain.unwrap_or(0.3);
        let noise_amt = noise_amt.unwrap_or(Signal::Value(0.1));

        // Pitch envelope: fast decay from 3x freq to base freq
        let pitch_env_node = graph.add_node(SignalNode::Envelope {
            input: Signal::Value(1.0),
            trigger: Signal::Value(1.0),
            attack: 0.001,
            decay: 0.05,
            sustain: 0.0,
            release: 0.001,
            state: EnvState::default(),
        });

        // Modulate frequency: base_freq + (pitch_env * base_freq * 2)
        let modulated_freq = Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Add(
            freq.clone(),
            Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
                Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
                    Signal::Node(pitch_env_node),
                    freq.clone(),
                ))),
                Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
                    pitch_env,
                    Signal::Value(2.0),
                ))),
            ))),
        )));

        // Sine oscillator for kick body
        let osc = graph.add_node(SignalNode::Oscillator {
            freq: modulated_freq,
            waveform: Waveform::Sine,
            phase: 0.0,
        });

        // Amplitude envelope
        let amp_env = graph.add_node(SignalNode::Envelope {
            input: Signal::Node(osc),
            trigger: Signal::Value(1.0),
            attack: 0.001,
            decay: sustain * 0.7,
            sustain: 0.3,
            release: sustain * 0.3,
            state: EnvState::default(),
        });

        // Optional noise layer for attack click
        let noise = graph.add_node(SignalNode::Noise { seed: 12345 });

        let noise_env = graph.add_node(SignalNode::Envelope {
            input: Signal::Node(noise),
            trigger: Signal::Value(1.0),
            attack: 0.001,
            decay: 0.01,
            sustain: 0.0,
            release: 0.005,
            state: EnvState::default(),
        });

        let noise_filtered = graph.add_node(SignalNode::LowPass {
            input: Signal::Node(noise_env),
            cutoff: Signal::Value(800.0),
            q: Signal::Value(0.5),
            state: FilterState::default(),
        });

        // Mix sine and noise
        let mixed = graph.add_node(SignalNode::Add {
            a: Signal::Node(amp_env),
            b: Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
                Signal::Node(noise_filtered),
                noise_amt,
            ))),
        });

        mixed
    }

    /// Build a SuperSaw synthesizer
    ///
    /// Rich, thick sound using multiple detuned saw waves
    ///
    /// # Parameters
    /// - `freq`: Base frequency
    /// - `detune`: Detune amount (0.0-1.0, default 0.3)
    /// - `voices`: Number of voices (2-7, default 7)
    pub fn build_supersaw(
        &self,
        graph: &mut UnifiedSignalGraph,
        freq: Signal,
        detune: Option<f32>,
        voices: Option<usize>,
    ) -> NodeId {
        let detune = detune.unwrap_or(0.3);
        let voices = voices.unwrap_or(7).min(7).max(2);

        // Create multiple detuned oscillators
        let mut oscillators = Vec::new();

        for i in 0..voices {
            // Calculate detune amount for this voice
            let offset = if voices > 1 {
                ((i as f32 / (voices - 1) as f32) - 0.5) * 2.0 // -1.0 to 1.0
            } else {
                0.0
            };

            let detune_factor = 1.0 + (offset * detune * 0.1); // Up to ±10% detuning

            let detuned_freq =
                Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
                    freq.clone(),
                    Signal::Value(detune_factor),
                )));

            let osc = graph.add_node(SignalNode::Oscillator {
                freq: detuned_freq,
                waveform: Waveform::Saw,
                phase: (i as f32 * 0.13) % 1.0, // Slight phase offset
            });

            oscillators.push(Signal::Node(osc));
        }

        // Mix all oscillators with scaling to prevent clipping
        let scale = 1.0 / voices as f32 * 0.7;
        self.mix_signals(graph, oscillators, scale)
    }

    /// Build a SuperPWM synthesizer
    ///
    /// Pulse width modulation creates a hollow, nasal sound
    ///
    /// # Parameters
    /// - `freq`: Base frequency
    /// - `pwm_rate`: LFO rate for PWM (0.1-10 Hz, default 0.5)
    /// - `pwm_depth`: PWM depth (0.0-1.0, default 0.8)
    pub fn build_superpwm(
        &self,
        graph: &mut UnifiedSignalGraph,
        freq: Signal,
        pwm_rate: Option<f32>,
        pwm_depth: Option<f32>,
    ) -> NodeId {
        let pwm_rate = pwm_rate.unwrap_or(0.5);
        let pwm_depth = pwm_depth.unwrap_or(0.8);

        // LFO for pulse width modulation
        let lfo = graph.add_node(SignalNode::Oscillator {
            freq: Signal::Value(pwm_rate),
            waveform: Waveform::Triangle,
            phase: 0.0,
        });

        // Create two square waves in opposite phase
        let square1 = graph.add_node(SignalNode::Oscillator {
            freq: freq.clone(),
            waveform: Waveform::Square,
            phase: 0.0,
        });

        let square2 = graph.add_node(SignalNode::Oscillator {
            freq,
            waveform: Waveform::Square,
            phase: 0.5, // 180 degrees out of phase
        });

        // Mix with LFO to create PWM effect
        let pwm = graph.add_node(SignalNode::Add {
            a: Signal::Node(square1),
            b: Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
                Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
                    Signal::Node(square2),
                    Signal::Node(lfo),
                ))),
                Signal::Value(pwm_depth),
            ))),
        });

        pwm
    }

    /// Build a SuperChip synthesizer
    ///
    /// Chiptune-style square wave with vibrato
    ///
    /// # Parameters
    /// - `freq`: Base frequency
    /// - `vibrato_rate`: Vibrato LFO rate (default 5.0 Hz)
    /// - `vibrato_depth`: Vibrato depth (default 0.05)
    pub fn build_superchip(
        &self,
        graph: &mut UnifiedSignalGraph,
        freq: Signal,
        vibrato_rate: Option<f32>,
        vibrato_depth: Option<f32>,
    ) -> NodeId {
        let vibrato_rate = vibrato_rate.unwrap_or(5.0);
        let vibrato_depth = vibrato_depth.unwrap_or(0.05);

        // Vibrato LFO
        let lfo = graph.add_node(SignalNode::Oscillator {
            freq: Signal::Value(vibrato_rate),
            waveform: Waveform::Sine,
            phase: 0.0,
        });

        // Modulate frequency with vibrato
        let modulated_freq = Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Add(
            freq.clone(),
            Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
                Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
                    Signal::Node(lfo),
                    freq,
                ))),
                Signal::Value(vibrato_depth),
            ))),
        )));

        let osc = graph.add_node(SignalNode::Oscillator {
            freq: modulated_freq,
            waveform: Waveform::Square,
            phase: 0.0,
        });

        osc
    }

    /// Build a SuperFM synthesizer
    ///
    /// 2-operator FM synthesis for bells, mallets, and metallic sounds
    ///
    /// # Parameters
    /// - `freq`: Carrier frequency
    /// - `mod_ratio`: Modulator/carrier ratio (default 2.0)
    /// - `mod_index`: Modulation index (default 1.0)
    pub fn build_superfm(
        &self,
        graph: &mut UnifiedSignalGraph,
        freq: Signal,
        mod_ratio: Option<f32>,
        mod_index: Option<f32>,
    ) -> NodeId {
        let mod_ratio = mod_ratio.unwrap_or(2.0);
        let mod_index = mod_index.unwrap_or(1.0);

        // Modulator oscillator
        let mod_freq = Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
            freq.clone(),
            Signal::Value(mod_ratio),
        )));

        let modulator = graph.add_node(SignalNode::Oscillator {
            freq: mod_freq,
            waveform: Waveform::Sine,
            phase: 0.0,
        });

        // Modulate carrier frequency
        let carrier_freq = Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Add(
            freq.clone(),
            Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
                Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
                    Signal::Node(modulator),
                    freq,
                ))),
                Signal::Value(mod_index),
            ))),
        )));

        let carrier = graph.add_node(SignalNode::Oscillator {
            freq: carrier_freq,
            waveform: Waveform::Sine,
            phase: 0.0,
        });

        carrier
    }

    /// Build a SuperSnare synthesizer
    ///
    /// Snare drum with filtered noise and pitch envelope
    ///
    /// # Parameters
    /// - `freq`: Base frequency (typically 150-250 Hz)
    /// - `snappy`: Snappiness/noise amount (0.0-1.0, default 0.8)
    /// - `sustain`: Decay time (default 0.15)
    pub fn build_snare(
        &self,
        graph: &mut UnifiedSignalGraph,
        freq: Signal,
        snappy: Option<f32>,
        sustain: Option<f32>,
    ) -> NodeId {
        let snappy = snappy.unwrap_or(0.8);
        let sustain = sustain.unwrap_or(0.15);

        // Tonal body (two slightly detuned oscillators)
        let osc1 = graph.add_node(SignalNode::Oscillator {
            freq: freq.clone(),
            waveform: Waveform::Triangle,
            phase: 0.0,
        });

        let osc2_freq = Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
            freq,
            Signal::Value(1.05),
        )));

        let osc2 = graph.add_node(SignalNode::Oscillator {
            freq: osc2_freq,
            waveform: Waveform::Triangle,
            phase: 0.3,
        });

        let body = graph.add_node(SignalNode::Add {
            a: Signal::Node(osc1),
            b: Signal::Node(osc2),
        });

        let body_env = graph.add_node(SignalNode::Envelope {
            input: Signal::Node(body),
            trigger: Signal::Value(1.0),
            attack: 0.001,
            decay: sustain * 0.3,
            sustain: 0.0,
            release: 0.001,
            state: EnvState::default(),
        });

        // Noise layer for snappiness
        let noise = graph.add_node(SignalNode::Noise { seed: 54321 });

        let noise_filtered = graph.add_node(SignalNode::HighPass {
            input: Signal::Node(noise),
            cutoff: Signal::Value(3000.0),
            q: Signal::Value(0.5),
            state: FilterState::default(),
        });

        let noise_env = graph.add_node(SignalNode::Envelope {
            input: Signal::Node(noise_filtered),
            trigger: Signal::Value(1.0),
            attack: 0.001,
            decay: sustain,
            sustain: 0.0,
            release: 0.001,
            state: EnvState::default(),
        });

        // Mix body and noise
        let mixed = graph.add_node(SignalNode::Add {
            a: Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
                Signal::Node(body_env),
                Signal::Value(0.5),
            ))),
            b: Signal::Expression(Box::new(crate::unified_graph::SignalExpr::Multiply(
                Signal::Node(noise_env),
                Signal::Value(snappy),
            ))),
        });

        mixed
    }

    /// Build a SuperHat synthesizer
    ///
    /// Hi-hat with filtered noise burst
    ///
    /// # Parameters
    /// - `bright`: Brightness/filter cutoff (0.0-1.0, default 0.7)
    /// - `sustain`: Decay time (default 0.05 for closed, 0.3 for open)
    pub fn build_hat(
        &self,
        graph: &mut UnifiedSignalGraph,
        bright: Option<f32>,
        sustain: Option<f32>,
    ) -> NodeId {
        let bright = bright.unwrap_or(0.7);
        let sustain = sustain.unwrap_or(0.05);

        // Noise source
        let noise = graph.add_node(SignalNode::Noise { seed: 98765 });

        // High-pass filter for metallic sound
        let filtered = graph.add_node(SignalNode::HighPass {
            input: Signal::Node(noise),
            cutoff: Signal::Value(5000.0 + bright * 5000.0),
            q: Signal::Value(1.5),
            state: FilterState::default(),
        });

        // Sharp envelope
        let hat = graph.add_node(SignalNode::Envelope {
            input: Signal::Node(filtered),
            trigger: Signal::Value(1.0),
            attack: 0.001,
            decay: sustain,
            sustain: 0.0,
            release: sustain * 0.3,
            state: EnvState::default(),
        });

        hat
    }

    /// Add reverb effect
    pub fn add_reverb(
        &self,
        graph: &mut UnifiedSignalGraph,
        input: NodeId,
        room_size: f32,
        damping: f32,
        mix: f32,
    ) -> NodeId {
        graph.add_node(SignalNode::Reverb {
            input: Signal::Node(input),
            room_size: Signal::Value(room_size),
            damping: Signal::Value(damping),
            mix: Signal::Value(mix),
            state: crate::unified_graph::ReverbState::new(self.sample_rate),
        })
    }

    /// Add distortion effect
    pub fn add_distortion(
        &self,
        graph: &mut UnifiedSignalGraph,
        input: NodeId,
        drive: f32,
        mix: f32,
    ) -> NodeId {
        graph.add_node(SignalNode::Distortion {
            input: Signal::Node(input),
            drive: Signal::Value(drive),
            mix: Signal::Value(mix),
        })
    }

    /// Add bitcrusher effect
    pub fn add_bitcrush(
        &self,
        graph: &mut UnifiedSignalGraph,
        input: NodeId,
        bits: f32,
        sample_rate_reduction: f32,
    ) -> NodeId {
        graph.add_node(SignalNode::BitCrush {
            input: Signal::Node(input),
            bits: Signal::Value(bits),
            sample_rate: Signal::Value(sample_rate_reduction),
            state: crate::unified_graph::BitCrushState::default(),
        })
    }

    /// Add chorus effect
    pub fn add_chorus(
        &self,
        graph: &mut UnifiedSignalGraph,
        input: NodeId,
        rate: f32,
        depth: f32,
        mix: f32,
    ) -> NodeId {
        graph.add_node(SignalNode::Chorus {
            input: Signal::Node(input),
            rate: Signal::Value(rate),
            depth: Signal::Value(depth),
            mix: Signal::Value(mix),
            state: crate::unified_graph::ChorusState::new(self.sample_rate),
        })
    }

    /// Add compressor effect
    pub fn add_compressor(
        &self,
        graph: &mut UnifiedSignalGraph,
        input: NodeId,
        threshold_db: f32,
        ratio: f32,
        attack: f32,
        release: f32,
        makeup_gain_db: f32,
    ) -> NodeId {
        graph.add_node(SignalNode::Compressor {
            input: Signal::Node(input),
            threshold: Signal::Value(threshold_db),
            ratio: Signal::Value(ratio),
            attack: Signal::Value(attack),
            release: Signal::Value(release),
            makeup_gain: Signal::Value(makeup_gain_db),
            state: crate::unified_graph::CompressorState::new(),
        })
    }

    // Helper function to mix multiple signals
    fn mix_signals(
        &self,
        graph: &mut UnifiedSignalGraph,
        signals: Vec<Signal>,
        scale: f32,
    ) -> NodeId {
        if signals.is_empty() {
            return graph.add_node(SignalNode::Constant { value: 0.0 });
        }

        if signals.len() == 1 {
            return match signals.into_iter().next().unwrap() {
                Signal::Node(id) => id,
                other => graph.add_node(SignalNode::Output { input: other }),
            };
        }

        // Recursively mix pairs
        let mut current = signals;
        while current.len() > 1 {
            let mut next = Vec::new();
            let mut iter = current.into_iter();

            while let Some(a) = iter.next() {
                if let Some(b) = iter.next() {
                    let mixed = graph.add_node(SignalNode::Add { a, b });
                    next.push(Signal::Node(mixed));
                } else {
                    next.push(a);
                }
            }

            current = next;
        }

        let final_mix = current.into_iter().next().unwrap();

        // Scale the output
        let scaled = graph.add_node(SignalNode::Multiply {
            a: final_mix,
            b: Signal::Value(scale),
        });

        scaled
    }
}

impl Default for SynthLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_kick() {
        let mut graph = UnifiedSignalGraph::new(44100.0);
        let library = SynthLibrary::new();

        let kick = library.build_kick(&mut graph, Signal::Value(60.0), None, None, None);

        graph.set_output(kick);

        // Render and check we got audio
        let buffer = graph.render(2205); // 50ms
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        assert!(rms > 0.01, "Kick should produce audio");
    }

    #[test]
    fn test_build_supersaw() {
        let mut graph = UnifiedSignalGraph::new(44100.0);
        let library = SynthLibrary::new();

        let saw = library.build_supersaw(&mut graph, Signal::Value(220.0), Some(0.5), Some(5));

        graph.set_output(saw);

        // Render a full second to get stable RMS
        let buffer = graph.render(44100);
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        // SuperSaw with 5 voices produces RMS around 0.15-0.20 due to phase interference
        assert!(rms > 0.1, "SuperSaw should produce audio, got RMS={}", rms);

        let min = buffer.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = buffer.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(
            max - min > 0.5,
            "SuperSaw should have reasonable amplitude range"
        );
    }

    #[test]
    fn test_build_snare() {
        let mut graph = UnifiedSignalGraph::new(44100.0);
        let library = SynthLibrary::new();

        let snare = library.build_snare(&mut graph, Signal::Value(200.0), None, None);

        graph.set_output(snare);

        let buffer = graph.render(2205); // 50ms
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        assert!(rms > 0.01, "Snare should produce audio");
    }

    #[test]
    fn test_build_superpwm() {
        let mut graph = UnifiedSignalGraph::new(44100.0);
        let library = SynthLibrary::new();

        let pwm = library.build_superpwm(&mut graph, Signal::Value(110.0), Some(0.5), Some(0.8));

        graph.set_output(pwm);

        let buffer = graph.render(44100); // 1 second
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        // PWM should produce strong audio
        assert!(rms > 0.3, "PWM should produce audio, got RMS={}", rms);
    }

    #[test]
    fn test_build_superchip() {
        let mut graph = UnifiedSignalGraph::new(44100.0);
        let library = SynthLibrary::new();

        let chip = library.build_superchip(&mut graph, Signal::Value(440.0), Some(5.0), Some(0.05));

        graph.set_output(chip);

        let buffer = graph.render(44100); // 1 second
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        // Square wave should have RMS around 0.7 (1/sqrt(2))
        assert!(rms > 0.5, "SuperChip should produce audio, got RMS={}", rms);
    }

    #[test]
    fn test_build_superfm() {
        let mut graph = UnifiedSignalGraph::new(44100.0);
        let library = SynthLibrary::new();

        let fm = library.build_superfm(&mut graph, Signal::Value(440.0), Some(2.0), Some(1.0));

        graph.set_output(fm);

        let buffer = graph.render(4410); // 100ms
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        // FM should produce audio (bell-like sound)
        assert!(rms > 0.1, "SuperFM should produce audio, got RMS={}", rms);
    }

    #[test]
    fn test_build_hat() {
        let mut graph = UnifiedSignalGraph::new(44100.0);
        let library = SynthLibrary::new();

        let hat = library.build_hat(&mut graph, Some(0.7), Some(0.05));

        graph.set_output(hat);

        let buffer = graph.render(2205); // 50ms
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        // Hi-hat should produce audio
        assert!(rms > 0.01, "SuperHat should produce audio");
    }

    #[test]
    fn test_synth_characterization_kick() {
        let mut graph = UnifiedSignalGraph::new(44100.0);
        let library = SynthLibrary::new();

        let kick = library.build_kick(&mut graph, Signal::Value(60.0), None, None, None);
        graph.set_output(kick);

        let buffer = graph.render(22050); // 0.5 seconds

        // Find peak amplitude in first 100ms
        let attack_samples = 4410; // 100ms
        let max_attack = buffer[..attack_samples]
            .iter()
            .map(|x| x.abs())
            .fold(0.0f32, f32::max);

        // Kick should have strong attack
        assert!(max_attack > 0.3, "Kick should have strong attack transient");

        // Check that sound decays (tail should be quieter than attack)
        let tail_samples = &buffer[attack_samples..];
        let max_tail = tail_samples.iter().map(|x| x.abs()).fold(0.0f32, f32::max);

        // Kick should decay (tail quieter than attack) but may have sustain
        assert!(
            max_tail < max_attack * 0.8,
            "Kick should decay over time: attack={}, tail={}",
            max_attack,
            max_tail
        );
    }

    #[test]
    fn test_synth_characterization_supersaw() {
        let mut graph = UnifiedSignalGraph::new(44100.0);
        let library = SynthLibrary::new();

        // Test at A3 (220 Hz)
        let saw = library.build_supersaw(&mut graph, Signal::Value(220.0), Some(0.5), Some(5));
        graph.set_output(saw);

        let buffer = graph.render(44100); // 1 second

        // SuperSaw should be continuous (not decay)
        let first_half_rms: f32 =
            (buffer[..22050].iter().map(|x| x * x).sum::<f32>() / 22050.0).sqrt();
        let second_half_rms: f32 =
            (buffer[22050..].iter().map(|x| x * x).sum::<f32>() / 22050.0).sqrt();

        // Both halves should have similar RMS (continuous sound)
        assert!(
            (first_half_rms - second_half_rms).abs() < 0.05,
            "SuperSaw should be continuous, not decay"
        );

        // Check detuning creates slight variations (chorus effect)
        // With 5 detuned oscillators, we expect some amplitude modulation
        let chunk_size = 2205; // 50ms chunks
        let mut chunk_rms_values = Vec::new();
        for chunk in buffer.chunks(chunk_size) {
            let rms = (chunk.iter().map(|x| x * x).sum::<f32>() / chunk.len() as f32).sqrt();
            chunk_rms_values.push(rms);
        }

        // Detuned oscillators should create some variation
        let mean_rms = chunk_rms_values.iter().sum::<f32>() / chunk_rms_values.len() as f32;
        let variance = chunk_rms_values
            .iter()
            .map(|x| (x - mean_rms).powi(2))
            .sum::<f32>()
            / chunk_rms_values.len() as f32;

        assert!(
            variance > 0.0001,
            "Detuning should create amplitude variation"
        );
    }

    #[test]
    fn test_synth_characterization_snare() {
        let mut graph = UnifiedSignalGraph::new(44100.0);
        let library = SynthLibrary::new();

        let snare = library.build_snare(&mut graph, Signal::Value(200.0), None, None);
        graph.set_output(snare);

        let buffer = graph.render(11025); // 0.25 seconds

        // Snare should have strong attack
        let attack_samples = 441; // 10ms
        let max_attack = buffer[..attack_samples]
            .iter()
            .map(|x| x.abs())
            .fold(0.0f32, f32::max);

        assert!(max_attack > 0.2, "Snare should have strong attack");

        // Snare should decay quickly (faster than kick)
        let tail_start = 4410; // After 100ms
        let max_tail = buffer[tail_start..]
            .iter()
            .map(|x| x.abs())
            .fold(0.0f32, f32::max);

        assert!(max_tail < max_attack * 0.3, "Snare should decay quickly");
    }

    #[test]
    fn test_synth_with_effects() {
        let mut graph = UnifiedSignalGraph::new(44100.0);
        let library = SynthLibrary::new();

        // Build a supersaw
        let saw = library.build_supersaw(&mut graph, Signal::Value(110.0), Some(0.5), Some(5));

        // Add effects chain: distortion -> chorus -> reverb
        let distorted = library.add_distortion(&mut graph, saw, 3.0, 0.3);
        let chorused = library.add_chorus(&mut graph, distorted, 1.0, 0.5, 0.3);
        let reverbed = library.add_reverb(&mut graph, chorused, 0.7, 0.5, 0.4);

        graph.set_output(reverbed);

        let buffer = graph.render(22050); // 0.5 seconds
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        assert!(
            rms > 0.1,
            "Synth with effects should produce audio, got RMS={}",
            rms
        );
    }
}
