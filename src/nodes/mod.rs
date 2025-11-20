/// Audio node implementations for DAW-style buffer passing
///
/// This module contains concrete implementations of the AudioNode trait.
///
/// # Node Categories
///
/// ## Source Nodes (no inputs)
/// - [`constant::ConstantNode`] - Output constant value
///
/// ## Math Nodes (combine signals)
/// - [`addition::AdditionNode`] - Add two signals
/// - [`subtraction::SubtractionNode`] - Subtract signal B from signal A
/// - [`multiplication::MultiplicationNode`] - Multiply two signals
/// - [`division::DivisionNode`] - Divide signal A by signal B (with zero protection)
/// - [`modulo::ModNode`] - Modulo operation (remainder) with positive results
/// - [`min::MinNode`] - Minimum of two signals
/// - [`max::MaxNode`] - Maximum of two signals
/// - [`invert::InvertNode`] - Phase inversion (multiply by -1)
/// - [`gain::GainNode`] - Apply gain/volume control to a signal
/// - [`absolute::AbsoluteNode`] - Absolute value (full-wave rectification)
/// - [`sqrt::SquareRootNode`] - Square root with absolute value protection
/// - [`log::LogNode`] - Natural logarithm (ln) with safety protections
/// - [`exp::ExpNode`] - Exponential function (e^x) with overflow protection
/// - [`cos::CosNode`] - Cosine function (waveshaper, not oscillator)
/// - [`sin::SinNode`] - Sine function (waveshaper, not oscillator)
/// - [`tan::TanNode`] - Tangent function (waveshaping/soft saturation)
/// - [`sign::SignNode`] - Sign function (returns -1, 0, or 1)
/// - [`mix::MixNode`] - Weighted sum of N signals
/// - [`lerp::LerpNode`] - Linear interpolation (crossfade) between two signals
/// - [`less_than::LessThanNode`] - Comparison operator (a < b returns 1.0, else 0.0)
/// - [`less_than_or_equal::LessThanOrEqualNode`] - Comparison operator (a <= b returns 1.0, else 0.0, with tolerance)
/// - [`greater_than::GreaterThanNode`] - Comparison operator (a > b returns 1.0, else 0.0)
/// - [`equal_to::EqualToNode`] - Comparison operator (a == b returns 1.0, else 0.0, with tolerance)
/// - [`not_equal_to::NotEqualToNode`] - Comparison operator (a != b returns 1.0, else 0.0, with tolerance)
/// - [`and::AndNode`] - Logical AND operator (both inputs > threshold returns 1.0, else 0.0)
/// - [`or::OrNode`] - Logical OR operator (either input > threshold returns 1.0, else 0.0)
///
/// ## Analysis Nodes (audio analysis)
/// - [`rms::RMSNode`] - Root Mean Square calculation with windowing
///
/// ## Synthesis Nodes (generate audio)
/// - [`oscillator::OscillatorNode`] - Waveform generation (sine, saw, square, triangle)
/// - [`pulse::PulseNode`] - Pulse wave oscillator with pulse width modulation (PWM)
/// - [`vco::VCONode`] - Voltage-controlled oscillator with polyBLEP anti-aliasing
/// - [`noise::NoiseNode`] - White noise generator
/// - [`brown_noise::BrownNoiseNode`] - Brown noise generator (Brownian/red noise with 6dB/octave rolloff)
/// - [`pink_noise::PinkNoiseNode`] - Pink noise generator (1/f spectrum)
/// - [`random::RandomNode`] - Random value generator (white noise with amplitude control)
/// - [`impulse::ImpulseNode`] - Periodic impulse/spike generator (single-sample spikes)
/// - [`blip::BlipNode`] - Band-limited impulse train (anti-aliased impulses)
/// - [`fm_oscillator::FMOscillatorNode`] - Frequency Modulation synthesis (classic FM)
/// - [`pm_oscillator::PMOscillatorNode`] - Phase Modulation synthesis (equivalent to FM, easier to implement)
///
/// ## Filter Nodes (shape audio)
/// - [`lowpass_filter::LowPassFilterNode`] - 2nd-order Butterworth low-pass filter
/// - [`highpass_filter::HighPassFilterNode`] - 2nd-order Butterworth high-pass filter
/// - [`bandpass_filter::BandPassFilterNode`] - 2nd-order Butterworth band-pass filter
/// - [`notch_filter::NotchFilterNode`] - 2nd-order notch (band-reject) filter
/// - [`moog_ladder::MoogLadderNode`] - 4-pole Moog ladder filter (classic analog sound)
/// - [`allpass_filter::AllPassFilterNode`] - 2nd-order all-pass filter (phase shifter)
///
/// ## Distortion Nodes (shape audio)
/// - [`clip::ClipNode`] - Soft clipping/distortion using tanh
/// - [`clamp::ClampNode`] - Hard limiting to [min, max] range
/// - [`wrap::WrapNode`] - Wrapping/folding into [min, max] range using modulo
/// - [`fold::FoldNode`] - Wave folding distortion (reflects signal at boundaries)
/// - [`quantizer::QuantizerNode`] - Snap values to grid/scale (quantization)
/// - [`rectifier::RectifierNode`] - Half-wave and full-wave rectification
///
/// ## Dynamics Nodes (control amplitude)
/// - [`limiter::LimiterNode`] - Hard limiting dynamics processor (prevents clipping)
/// - [`gate::GateNode`] - Threshold gate (passes signal above threshold, silences below)
///
/// ## Effect Nodes (time-based effects)
/// - [`delay::DelayNode`] - Simple delay line with circular buffer
/// - [`comb_filter::CombFilterNode`] - Feedback comb filter for resonance and reverb
/// - [`tremolo::TremoloNode`] - Amplitude modulation effect
/// - [`vibrato::VibratoNode`] - Pitch modulation effect using delay line
/// - [`flanger::FlangerNode`] - Flanging effect (short modulated delay)
/// - [`ring_mod::RingModNode`] - Ring modulation (creates sum/difference frequencies)
///
/// ## Envelope Nodes (amplitude shaping)
/// - [`adsr::ADSRNode`] - Attack-Decay-Sustain-Release envelope generator
/// - [`ar_envelope::AREnvelopeNode`] - Attack-Release envelope generator (simpler than ADSR)
/// - [`ad_envelope::ADEnvelopeNode`] - Attack-Decay envelope generator (one-shot, percussion)
/// - [`line::LineNode`] - Linear ramp generator (start to end over duration)
/// - [`asr_envelope::ASREnvelopeNode`] - Attack-Sustain-Release envelope generator (organ-style)
///
/// ## Smoothing Nodes (signal conditioning)
/// - [`slew_limiter::SlewLimiterNode`] - Rate-of-change limiter for smooth transitions
/// - [`lag::LagNode`] - Exponential slew limiter for portamento/glide effects
/// - [`xline::XLineNode`] - Exponential line generator (natural ramps for pitch/amplitude)
///
/// ## Analysis Nodes (signal analysis)
/// - [`peak_detector::PeakDetectorNode`] - Peak tracking with configurable decay
///
/// ## Spatial Nodes (stereo positioning)
/// - [`pan::PanNode`] - Equal-power stereo panning
///
/// # Usage
///
/// ```ignore
/// use phonon::nodes::{ConstantNode, OscillatorNode, Waveform};
/// use phonon::audio_node::AudioNode;
///
/// // Create a 440 Hz sine wave
/// let freq = Box::new(ConstantNode::new(440.0));  // Node 0
/// let osc = Box::new(OscillatorNode::new(0, Waveform::Sine));  // Node 1
///
/// let nodes: Vec<Box<dyn AudioNode>> = vec![freq, osc];
/// ```

pub mod constant;
pub mod addition;
pub mod subtraction;
pub mod multiplication;
pub mod modulo;
pub mod division;
pub mod invert;
pub mod absolute;
pub mod lerp;
pub mod sign;
pub mod sqrt;
pub mod log;
pub mod exp;
pub mod power;
pub mod gain;
pub mod mix;
pub mod oscillator;
pub mod brown_noise;
pub mod pink_noise;
pub mod vco;
pub mod pulse;
pub mod noise;
pub mod random;
pub mod fm_oscillator;
pub mod pm_oscillator;
pub mod lowpass_filter;
pub mod highpass_filter;
pub mod bandpass_filter;
pub mod notch_filter;
pub mod moog_ladder;
pub mod allpass_filter;
pub mod clip;
pub mod clamp;
pub mod max;
pub mod pan;
pub mod delay;
pub mod comb_filter;
pub mod min;
pub mod peak_detector;
pub mod tremolo;
pub mod wrap;
pub mod fold;
pub mod sample_hold;
pub mod latch;
pub mod less_than;
pub mod less_than_or_equal;
pub mod greater_than;
pub mod equal_to;
pub mod not_equal_to;
pub mod quantizer;
pub mod rms;
pub mod ar_envelope;
pub mod ad_envelope;
pub mod adsr;
pub mod asr_envelope;
pub mod gate;
pub mod rectifier;
pub mod flanger;
pub mod phaser;
pub mod vibrato;
pub mod slew_limiter;
pub mod sin;
pub mod cos;
pub mod tan;
pub mod impulse;
pub mod blip;
pub mod ring_mod;
pub mod line;
pub mod lag;
pub mod wavetable;
pub mod xline;
pub mod and;
pub mod or;
pub mod svf;

pub use constant::ConstantNode;
pub use addition::AdditionNode;
pub use subtraction::SubtractionNode;
pub use multiplication::MultiplicationNode;
pub use modulo::ModNode;
pub use division::DivisionNode;
pub use lerp::LerpNode;
pub use invert::InvertNode;
pub use sign::SignNode;
pub use exp::ExpNode;
pub use absolute::AbsoluteNode;
pub use sqrt::SquareRootNode;
pub use log::LogNode;
pub use power::PowerNode;
pub use gain::GainNode;
pub use mix::MixNode;
pub use oscillator::{OscillatorNode, Waveform};
pub use vco::{VCONode, VCOWaveform};
pub use brown_noise::BrownNoiseNode;
pub use pink_noise::PinkNoiseNode;
pub use pulse::PulseNode;
pub use noise::NoiseNode;
pub use random::RandomNode;
pub use fm_oscillator::FMOscillatorNode;
pub use pm_oscillator::PMOscillatorNode;
pub use lowpass_filter::LowPassFilterNode;
pub use highpass_filter::HighPassFilterNode;
pub use pan::PanNode;
pub use tremolo::TremoloNode;
pub use bandpass_filter::BandPassFilterNode;
pub use notch_filter::NotchFilterNode;
pub use allpass_filter::AllPassFilterNode;
pub use moog_ladder::MoogLadderNode;
pub use delay::DelayNode;
pub use comb_filter::CombFilterNode;
pub use clip::ClipNode;
pub use peak_detector::PeakDetectorNode;
pub use clamp::ClampNode;
pub use max::MaxNode;
pub use min::MinNode;
pub use wrap::WrapNode;
pub use fold::FoldNode;
pub use sample_hold::SampleAndHoldNode;
pub use latch::LatchNode;
pub use less_than::LessThanNode;
pub use less_than_or_equal::LessThanOrEqualNode;
pub use ar_envelope::AREnvelopeNode;
pub use ad_envelope::ADEnvelopeNode;
pub use asr_envelope::ASREnvelopeNode;
pub use greater_than::GreaterThanNode;
pub use equal_to::EqualToNode;
pub use not_equal_to::NotEqualToNode;
pub use quantizer::QuantizerNode;
pub use rms::RMSNode;
pub use adsr::ADSRNode;
pub mod limiter;
pub use limiter::LimiterNode;
pub use gate::GateNode;
pub use flanger::FlangerNode;
pub use phaser::PhaserNode;
pub use slew_limiter::SlewLimiterNode;
pub use rectifier::{RectifierNode, RectifierMode};
pub use vibrato::VibratoNode;
pub use cos::CosNode;
pub use sin::SinNode;
pub use line::LineNode;
pub use tan::TanNode;
pub use impulse::ImpulseNode;
pub use blip::BlipNode;
pub use ring_mod::RingModNode;
pub use lag::LagNode;
pub use wavetable::WavetableNode;
pub use and::AndNode;
pub use xline::XLineNode;
pub use or::OrNode;
pub use svf::{SVFNode, SVFMode};
