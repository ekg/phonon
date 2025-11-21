/// Pitch detection node using autocorrelation
///
/// This node analyzes an incoming audio signal and estimates its fundamental
/// frequency (pitch) using the autocorrelation method.
///
/// # Implementation Details
///
/// Uses autocorrelation to find periodicity in the signal:
/// 1. Computes autocorrelation function across a range of delays
/// 2. Finds peaks in the autocorrelation that indicate period length
/// 3. Converts period to frequency (Hz)
/// 4. Uses energy threshold to detect silence/noise
///
/// # Algorithm
///
/// The autocorrelation method is robust for:
/// - Periodic signals (sine, saw, square, triangle waves)
/// - Complex harmonic sounds (voice, musical instruments)
/// - Polyphonic signals (will detect strongest fundamental)
///
/// Limitations:
/// - Requires sufficient buffer size (2048+ samples for low frequencies)
/// - Not suitable for very fast pitch tracking (1-2 block latency)
/// - May octave-confuse on very complex timbres
///
/// # References
///
/// - "A Smarter Way to Find Pitch" - Philip McLeod & Geoff Wyvill (2005)
/// - "YIN, a fundamental frequency estimator" - de Cheveigné & Kawahara (2002)
/// - Classic autocorrelation pitch detection (Rabiner, 1977)
///
/// # Musical Characteristics
///
/// - Detects pitch from any periodic signal
/// - Outputs 0 Hz for silence or noise
/// - Smooth tracking with exponential smoothing
/// - Works well with oscillators, samples, voice
/// - Can be used to track melody, create harmonizers, or analyze content

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Minimum frequency to detect (Hz)
const MIN_FREQ: f32 = 40.0; // Below low E on bass guitar

/// Maximum frequency to detect (Hz)
const MAX_FREQ: f32 = 4000.0; // Well above typical musical range

/// Energy threshold for silence detection (RMS)
const SILENCE_THRESHOLD: f32 = 0.001;

/// Smoothing factor for pitch tracking (0.0 = no smooth, 1.0 = full smooth)
const SMOOTHING: f32 = 0.9;

/// Minimum autocorrelation peak height to accept pitch
const AUTOCORR_THRESHOLD: f32 = 0.3;

/// Internal state for pitch detection
#[derive(Debug, Clone)]
struct PitchState {
    /// Last detected pitch (Hz) for smoothing
    last_pitch: f32,
    /// Ring buffer for signal history
    buffer: Vec<f32>,
    /// Current write position in ring buffer
    buffer_pos: usize,
}

impl PitchState {
    fn new(buffer_size: usize) -> Self {
        Self {
            last_pitch: 0.0,
            buffer: vec![0.0; buffer_size],
            buffer_pos: 0,
        }
    }

    /// Compute autocorrelation at a specific lag
    fn autocorrelation(&self, lag: usize, len: usize) -> f32 {
        let mut sum = 0.0;
        for i in 0..len {
            sum += self.buffer[i] * self.buffer[(i + lag) % self.buffer.len()];
        }
        sum
    }

    /// Calculate RMS energy of buffer
    fn calculate_rms(&self, len: usize) -> f32 {
        let sum_squares: f32 = self.buffer[0..len].iter().map(|&x| x * x).sum();
        (sum_squares / len as f32).sqrt()
    }

    /// Detect pitch using autocorrelation
    ///
    /// Returns detected frequency in Hz, or 0.0 if no pitch detected
    fn detect_pitch(&mut self, sample_rate: f32, block_size: usize) -> f32 {
        // Check if we have enough signal energy
        let rms = self.calculate_rms(block_size);
        if rms < SILENCE_THRESHOLD {
            self.last_pitch = 0.0;
            return 0.0;
        }

        // Calculate lag range based on frequency limits
        let min_lag = (sample_rate / MAX_FREQ).ceil() as usize;
        let max_lag = (sample_rate / MIN_FREQ).floor() as usize;

        // Ensure we don't exceed buffer size
        let max_lag = max_lag.min(block_size / 2);

        if min_lag >= max_lag {
            return 0.0; // Can't detect in this range
        }

        // Compute autocorrelation at lag 0 for normalization
        let r0 = self.autocorrelation(0, block_size);
        if r0 < 1e-10 {
            return 0.0; // Avoid division by zero
        }

        // Find the lag with maximum autocorrelation (excluding lag 0)
        let mut max_corr = 0.0;
        let mut best_lag = 0;

        for lag in min_lag..max_lag {
            let corr = self.autocorrelation(lag, block_size - lag);
            let normalized_corr = corr / r0; // Normalize

            if normalized_corr > max_corr {
                max_corr = normalized_corr;
                best_lag = lag;
            }
        }

        // Check if peak is strong enough
        if max_corr < AUTOCORR_THRESHOLD {
            // No clear pitch detected
            self.last_pitch *= SMOOTHING; // Decay towards zero
            return self.last_pitch;
        }

        // Convert lag to frequency
        let detected_pitch = sample_rate / best_lag as f32;

        // Apply exponential smoothing
        let smoothed_pitch = if self.last_pitch > 0.0 {
            self.last_pitch * SMOOTHING + detected_pitch * (1.0 - SMOOTHING)
        } else {
            detected_pitch
        };

        self.last_pitch = smoothed_pitch;
        smoothed_pitch
    }
}

/// Pitch detection node
///
/// Analyzes input signal and outputs detected frequency in Hz.
/// Outputs 0 Hz for silence or noise (no clear pitch).
///
/// # Example
/// ```ignore
/// // Detect pitch from microphone input
/// let mic_input = ...;                                    // NodeId 0
/// let pitch_detector = PitchNode::new(0);                 // NodeId 1
/// let frequency_display = pitch_detector.output();
/// // frequency_display contains detected pitch in Hz
/// ```
///
/// # Musical Applications
/// - Auto-tuning and pitch correction
/// - Melody tracking and transcription
/// - Harmonizer effects (detect + pitch shift)
/// - Pitch-to-MIDI conversion
/// - Vocal analysis
/// - Instrument tuning
pub struct PitchNode {
    /// Input signal to analyze
    input: NodeId,
    /// Pitch detection state
    state: PitchState,
}

impl PitchNode {
    /// PitchNode - Fundamental frequency detection via autocorrelation
    ///
    /// Analyzes input signal to detect fundamental frequency using autocorrelation
    /// method. Used for pitch tracking, monophonic source analysis, and creative
    /// synthesis modulation from audio input.
    ///
    /// # Parameters
    /// - `input`: NodeId of signal to analyze for pitch
    ///
    /// # Example
    /// ```phonon
    /// ~voice: sample "voice"
    /// ~detected_freq: pitch ~voice
    /// ~synth: sine ~detected_freq * 2
    /// ```
    pub fn new(input: NodeId) -> Self {
        Self {
            input,
            // Use large buffer to detect low frequencies (4096 samples = ~93ms at 44.1kHz)
            state: PitchState::new(4096),
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Reset the pitch detector state
    pub fn reset(&mut self) {
        self.state = PitchState::new(4096);
    }
}

impl AudioNode for PitchNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert_eq!(
            inputs.len(),
            1,
            "PitchNode requires 1 input: signal"
        );

        let input_buffer = inputs[0];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        let block_size = output.len();

        // Fill ring buffer with new samples
        for &sample in input_buffer.iter() {
            self.state.buffer[self.state.buffer_pos] = sample;
            self.state.buffer_pos = (self.state.buffer_pos + 1) % self.state.buffer.len();
        }

        // Detect pitch once per block (more efficient than per-sample)
        let detected_pitch = self.state.detect_pitch(sample_rate, block_size);

        // Output constant detected pitch for entire block
        for sample in output.iter_mut() {
            *sample = detected_pitch;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input]
    }

    fn name(&self) -> &str {
        "PitchNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::nodes::oscillator::{OscillatorNode, Waveform};
    use crate::pattern::Fraction;

    /// Helper: Create test context
    fn test_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_pitch_basic_functionality() {
        // Basic smoke test: Create node and process without panicking
        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut freq_const = ConstantNode::new(440.0);
        let mut sine = OscillatorNode::new(0, Waveform::Sine);
        let mut pitch = PitchNode::new(1);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut pitch_buf = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

        let sine_inputs = vec![freq_buf.as_slice()];
        sine.process_block(&sine_inputs, &mut signal_buf, sample_rate, &context);

        let pitch_inputs = vec![signal_buf.as_slice()];
        pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);

        // Should produce some output
        assert!(pitch_buf.len() == block_size);
    }

    #[test]
    fn test_pitch_state_initialization() {
        // State should initialize with zeros
        let state = PitchState::new(1024);
        assert_eq!(state.last_pitch, 0.0);
        assert_eq!(state.buffer.len(), 1024);
        assert_eq!(state.buffer_pos, 0);
    }

    #[test]
    fn test_pitch_detects_positive_values() {
        // Pitch should always be non-negative
        let sample_rate = 44100.0;
        let block_size = 2048;

        let mut freq_const = ConstantNode::new(220.0);
        let mut sine = OscillatorNode::new(0, Waveform::Sine);
        let mut pitch = PitchNode::new(1);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut pitch_buf = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

        let sine_inputs = vec![freq_buf.as_slice()];
        sine.process_block(&sine_inputs, &mut signal_buf, sample_rate, &context);

        for _ in 0..10 {
            sine.process_block(&sine_inputs, &mut signal_buf, sample_rate, &context);
            let pitch_inputs = vec![signal_buf.as_slice()];
            pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);

            // All pitch values should be non-negative
            for &pitch_val in pitch_buf.iter() {
                assert!(
                    pitch_val >= 0.0,
                    "Pitch should be non-negative: {}",
                    pitch_val
                );
            }
        }
    }

    #[test]
    fn test_pitch_outputs_finite_values() {
        // All outputs should be finite (no NaN or Inf)
        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut freq_const = ConstantNode::new(330.0);
        let mut saw = OscillatorNode::new(0, Waveform::Saw);
        let mut pitch = PitchNode::new(1);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut pitch_buf = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

        let saw_inputs = vec![freq_buf.as_slice()];

        for _ in 0..20 {
            saw.process_block(&saw_inputs, &mut signal_buf, sample_rate, &context);
            let pitch_inputs = vec![signal_buf.as_slice()];
            pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);

            for (i, &pitch_val) in pitch_buf.iter().enumerate() {
                assert!(
                    pitch_val.is_finite(),
                    "Sample {} not finite: {}",
                    i,
                    pitch_val
                );
            }
        }
    }

    #[test]
    fn test_pitch_input_nodes() {
        let pitch = PitchNode::new(123);
        let deps = pitch.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 123);
    }

    #[test]
    fn test_pitch_name() {
        let pitch = PitchNode::new(0);
        assert_eq!(pitch.name(), "PitchNode");
    }
}
