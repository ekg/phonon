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
///
/// ## Synthesis Nodes (generate audio)
/// - [`oscillator::OscillatorNode`] - Waveform generation (sine, saw, square, triangle)
///
/// ## Filter Nodes (shape audio)
/// - [`lowpass_filter::LowPassFilterNode`] - 2nd-order Butterworth low-pass filter
/// - [`highpass_filter::HighPassFilterNode`] - 2nd-order Butterworth high-pass filter
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
pub mod oscillator;
pub mod lowpass_filter;
pub mod highpass_filter;

pub use constant::ConstantNode;
pub use addition::AdditionNode;
pub use multiplication::MultiplicationNode;
pub use oscillator::{OscillatorNode, Waveform};
pub use lowpass_filter::LowPassFilterNode;
pub use highpass_filter::HighPassFilterNode;
