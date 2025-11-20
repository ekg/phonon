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
/// - [`min::MinNode`] - Minimum of two signals
/// - [`max::MaxNode`] - Maximum of two signals
/// - [`invert::InvertNode`] - Phase inversion (multiply by -1)
/// - [`gain::GainNode`] - Apply gain/volume control to a signal
/// - [`absolute::AbsoluteNode`] - Absolute value (full-wave rectification)
/// - [`sqrt::SquareRootNode`] - Square root with absolute value protection
/// - [`sign::SignNode`] - Sign function (returns -1, 0, or 1)
/// - [`mix::MixNode`] - Weighted sum of N signals
/// - [`lerp::LerpNode`] - Linear interpolation (crossfade) between two signals
///
/// ## Synthesis Nodes (generate audio)
/// - [`oscillator::OscillatorNode`] - Waveform generation (sine, saw, square, triangle)
/// - [`noise::NoiseNode`] - White noise generator
///
/// ## Filter Nodes (shape audio)
/// - [`lowpass_filter::LowPassFilterNode`] - 2nd-order Butterworth low-pass filter
/// - [`highpass_filter::HighPassFilterNode`] - 2nd-order Butterworth high-pass filter
/// - [`bandpass_filter::BandPassFilterNode`] - 2nd-order Butterworth band-pass filter
///
/// ## Distortion Nodes (shape audio)
/// - [`clip::ClipNode`] - Soft clipping/distortion using tanh
/// - [`clamp::ClampNode`] - Hard limiting to [min, max] range
/// - [`wrap::WrapNode`] - Wrapping/folding into [min, max] range using modulo
///
/// ## Effect Nodes (time-based effects)
/// - [`delay::DelayNode`] - Simple delay line with circular buffer
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
pub mod division;
pub mod invert;
pub mod absolute;
pub mod lerp;
pub mod sign;
pub mod sqrt;
pub mod power;
pub mod gain;
pub mod mix;
pub mod oscillator;
pub mod noise;
pub mod lowpass_filter;
pub mod highpass_filter;
pub mod bandpass_filter;
pub mod clip;
pub mod clamp;
pub mod max;
pub mod pan;
pub mod delay;
pub mod min;
pub mod wrap;
pub mod sample_hold;

pub use constant::ConstantNode;
pub use addition::AdditionNode;
pub use subtraction::SubtractionNode;
pub use multiplication::MultiplicationNode;
pub use division::DivisionNode;
pub use lerp::LerpNode;
pub use invert::InvertNode;
pub use absolute::AbsoluteNode;
pub use sqrt::SquareRootNode;
pub use power::PowerNode;
pub use gain::GainNode;
pub use mix::MixNode;
pub use oscillator::{OscillatorNode, Waveform};
pub use noise::NoiseNode;
pub use lowpass_filter::LowPassFilterNode;
pub use highpass_filter::HighPassFilterNode;
pub use pan::PanNode;
pub use bandpass_filter::BandPassFilterNode;
pub use delay::DelayNode;
pub use clip::ClipNode;
pub use clamp::ClampNode;
pub use max::MaxNode;
pub use min::MinNode;
pub use wrap::WrapNode;
pub use sample_hold::SampleAndHoldNode;
