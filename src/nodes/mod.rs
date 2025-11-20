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
/// - [`multiplication::MultiplicationNode`] - Multiply two signals
/// - [`min::MinNode`] - Minimum of two signals
/// - [`max::MaxNode`] - Maximum of two signals
/// - [`invert::InvertNode`] - Phase inversion (multiply by -1)
/// - [`gain::GainNode`] - Apply gain/volume control to a signal
/// - [`absolute::AbsoluteNode`] - Absolute value (full-wave rectification)
/// - [`mix::MixNode`] - Weighted sum of N signals
///
/// ## Synthesis Nodes (generate audio)
/// - [`oscillator::OscillatorNode`] - Waveform generation (sine, saw, square, triangle)
///
/// ## Filter Nodes (shape audio)
/// - [`lowpass_filter::LowPassFilterNode`] - 2nd-order Butterworth low-pass filter
/// - [`highpass_filter::HighPassFilterNode`] - 2nd-order Butterworth high-pass filter
/// - [`bandpass_filter::BandPassFilterNode`] - 2nd-order Butterworth band-pass filter
///
/// ## Distortion Nodes (shape audio)
/// - [`clip::ClipNode`] - Soft clipping/distortion using tanh
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
pub mod multiplication;
pub mod invert;
pub mod absolute;
pub mod gain;
pub mod mix;
pub mod oscillator;
pub mod lowpass_filter;
pub mod highpass_filter;
pub mod bandpass_filter;
pub mod clip;
pub mod max;
pub mod min;

pub use constant::ConstantNode;
pub use addition::AdditionNode;
pub use multiplication::MultiplicationNode;
pub use invert::InvertNode;
pub use absolute::AbsoluteNode;
pub use gain::GainNode;
pub use mix::MixNode;
pub use oscillator::{OscillatorNode, Waveform};
pub use lowpass_filter::LowPassFilterNode;
pub use highpass_filter::HighPassFilterNode;
pub use bandpass_filter::BandPassFilterNode;
pub use clip::ClipNode;
pub use max::MaxNode;
pub use min::MinNode;
