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
/// - [`xfade::XFadeNode`] - Crossfader with linear or equal-power curves
/// - [`less_than_or_equal::LessThanOrEqualNode`] - Comparison operator (a <= b returns 1.0, else 0.0, with tolerance)
/// - [`greater_than::GreaterThanNode`] - Comparison operator (a > b returns 1.0, else 0.0)
/// - [`greater_than_or_equal::GreaterThanOrEqualNode`] - Comparison operator (a >= b returns 1.0, else 0.0, with tolerance)
/// - [`equal_to::EqualToNode`] - Comparison operator (a == b returns 1.0, else 0.0, with tolerance)
/// - [`not_equal_to::NotEqualToNode`] - Comparison operator (a != b returns 1.0, else 0.0, with tolerance)
/// - [`and::AndNode`] - Logical AND operator (both inputs > threshold returns 1.0, else 0.0)
/// - [`or::OrNode`] - Logical OR operator (either input > threshold returns 1.0, else 0.0)
/// - [`xor::XorNode`] - Logical XOR operator (exactly one input > threshold returns 1.0, else 0.0)
/// - [`not::NotNode`] - Logical NOT operator (inverts boolean signal: input > threshold returns 0.0, else 1.0)
/// - [`when::WhenNode`] - Conditional signal router (audio if-statement: routes one of two inputs based on condition)
///
/// ## Analysis Nodes (audio analysis)
/// - [`rms::RMSNode`] - Root Mean Square calculation with windowing
///
/// ## Synthesis Nodes (generate audio)
/// - [`oscillator::OscillatorNode`] - Waveform generation (sine, saw, square, triangle)
/// - [`pulse::PulseNode`] - Pulse wave oscillator with pulse width modulation (PWM)
/// - [`vco::VCONode`] - Voltage-controlled oscillator with polyBLEP anti-aliasing
/// - [`polyblep_osc::PolyBLEPOscNode`] - Anti-aliased oscillator (saw, square, triangle) using PolyBLEP
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
/// - [`one_pole_filter::OnePoleFilterNode`] - 1st-order filter (6 dB/octave, very efficient, analog character)
/// - [`lowpass_filter::LowPassFilterNode`] - 2nd-order Butterworth low-pass filter
/// - [`highpass_filter::HighPassFilterNode`] - 2nd-order Butterworth high-pass filter
/// - [`rlpf::RLPFNode`] - Resonant low-pass filter with Q control (classic synth sound)
/// - [`rhpf::RHPFNode`] - Resonant high-pass filter with Q control
/// - [`bandpass_filter::BandPassFilterNode`] - 2nd-order Butterworth band-pass filter
/// - [`resonz::ResonzNode`] - Resonant bandpass filter with rq (reciprocal Q) control
/// - [`notch_filter::NotchFilterNode`] - 2nd-order notch (band-reject) filter
/// - [`moog_ladder::MoogLadderNode`] - 4-pole Moog ladder filter (classic analog sound)
/// - [`allpass_filter::AllPassFilterNode`] - 2nd-order all-pass filter (phase shifter)
/// - [`hilbert_transformer::HilbertTransformerNode`] - Hilbert transformer (90° phase shift for SSB modulation)
/// - [`formant::FormantNode`] - Formant filter for vowel synthesis (A, E, I, O, U)
/// - [`dj_filter::DJFilterNode`] - DJ-style crossfader filter (lowpass/highpass with resonance)
/// - [`crossover::CrossoverLowNode`] - Low band of Linkwitz-Riley 24dB/oct 3-band crossover
/// - [`crossover::CrossoverMidNode`] - Mid band of Linkwitz-Riley 24dB/oct 3-band crossover
/// - [`crossover::CrossoverHighNode`] - High band of Linkwitz-Riley 24dB/oct 3-band crossover
///
/// ## Distortion Nodes (shape audio)
/// - [`clip::ClipNode`] - Soft clipping/distortion using tanh
/// - [`clamp::ClampNode`] - Hard limiting to [min, max] range
/// - [`wrap::WrapNode`] - Wrapping/folding into [min, max] range using modulo
/// - [`fold::FoldNode`] - Threshold-based wavefolding distortion (classic synthesis technique)
/// - [`quantizer::QuantizerNode`] - Snap values to grid/scale (quantization)
/// - [`scale_quantize::ScaleQuantizeNode`] - Quantize frequencies to musical scales (major, minor, pentatonic, etc.)
/// - [`rectifier::RectifierNode`] - Half-wave and full-wave rectification
/// - [`decimator::DecimatorNode`] - Sample rate reduction for lo-fi/retro effects (bit-crush style)
/// - [`quantize::QuantizeNode`] - Bit depth reduction for lo-fi/digital distortion effects
///
/// ## Dynamics Nodes (control amplitude)
/// - [`limiter::LimiterNode`] - Hard limiting dynamics processor (prevents clipping)
/// - [`gate::GateNode`] - Threshold gate (passes signal above threshold, silences below)
/// - [`compressor::CompressorNode`] - Smooth dynamics compression with attack/release
/// - [`noise_gate::NoiseGateNode`] - Smooth noise gate with attack/release (production-ready gating)
///
/// ## Effect Nodes (time-based effects)
/// - [`delay::DelayNode`] - Simple delay line with circular buffer
/// - [`multitap_delay::MultiTapDelayNode`] - Multiple delay taps for rhythmic echo patterns
/// - [`comb_filter::CombFilterNode`] - Feedback comb filter for resonance and reverb
/// - [`reverb::ReverbNode`] - Schroeder reverb with room size and damping control
/// - [`dattorro_reverb::DattorroReverbNode`] - High-quality Dattorro plate reverb (superior to Schroeder)
/// - [`tremolo::TremoloNode`] - Amplitude modulation effect
/// - [`vibrato::VibratoNode`] - Pitch modulation effect using delay line
/// - [`flanger::FlangerNode`] - Flanging effect (short modulated delay)
/// - [`chorus::ChorusNode`] - Chorus effect (pitch-shifting delay, no feedback)
/// - [`ring_mod::RingModNode`] - Ring modulation (creates sum/difference frequencies)
/// - [`frequency_shifter::FrequencyShifterNode`] - Frequency shifting (linear shift, creates inharmonic content)
/// - [`karplus_strong::KarplusStrongNode`] - Karplus-Strong plucked string synthesis (physical modeling)
/// - [`waveguide::WaveguideNode`] - Digital waveguide synthesis for realistic physical modeling
/// - [`granular::GranularNode`] - Granular synthesis for texture and drone generation
/// - [`convolution::ConvolutionNode`] - FFT-based convolution reverb using impulse responses
/// - [`pitch_shifter::PitchShifterNode`] - Pitch shifting without time stretching (delay-based)
/// - [`resample::ResampleNode`] - High-quality sample rate conversion with linear interpolation
/// - [`slice::SliceNode`] - Sample slicing with trigger control (play portions of accumulated buffer)
///
/// ## Envelope Nodes (amplitude shaping)
/// - [`adsr::ADSRNode`] - Attack-Decay-Sustain-Release envelope generator
/// - [`ar_envelope::AREnvelopeNode`] - Attack-Release envelope generator (simpler than ADSR)
/// - [`ad_envelope::ADEnvelopeNode`] - Attack-Decay envelope generator (one-shot, percussion)
/// - [`line::LineNode`] - Linear ramp generator (start to end over duration)
/// - [`curve::CurveNode`] - Curved ramp generator (exponential/logarithmic curves)
/// - [`asr_envelope::ASREnvelopeNode`] - Attack-Sustain-Release envelope generator (organ-style)
/// - [`segments::SegmentsNode`] - Multi-segment envelope generator with configurable breakpoints
///
/// ## Smoothing Nodes (signal conditioning)
/// - [`slew_limiter::SlewLimiterNode`] - Rate-of-change limiter for smooth transitions
/// - [`lag::LagNode`] - Exponential slew limiter for portamento/glide effects
/// - [`xline::XLineNode`] - Exponential line generator (natural ramps for pitch/amplitude)
///
/// ## Analysis Nodes (signal analysis)
/// - [`peak_detector::PeakDetectorNode`] - Peak tracking with configurable decay
/// - [`envelope_follower::EnvelopeFollowerNode`] - Amplitude envelope extraction with attack/release
///
/// ## Spatial Nodes (stereo positioning)
/// - [`pan::PanNode`] - Equal-power stereo panning
/// - [`auto_pan::AutoPanNode`] - Automatic panning with LFO modulation
/// - [`stereo_widener::StereoWidenerNode`] - Stereo width control using Mid/Side processing
/// - [`stereo_splitter::StereoSplitterNode`] - Stereo signal splitter (identity passthrough, future L/R separation)
/// - [`stereo_merger::StereoMergerNode`] - Merge two mono signals into stereo (currently mono mix)
///
/// ## Utility Nodes (conversion and helper functions)
/// - [`tap::TapNode`] - Tap tempo converter (beats to seconds for tempo-synced parameters)
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
pub mod polyblep_osc;
pub mod noise;
pub mod random;
pub mod fm_oscillator;
pub mod pm_oscillator;
pub mod one_pole_filter;
pub mod lowpass_filter;
pub mod highpass_filter;
pub mod rlpf;
pub mod rhpf;
pub mod bandpass_filter;
pub mod notch_filter;
pub mod moog_ladder;
pub mod allpass_filter;
pub mod hilbert_transformer;
pub mod clip;
pub mod clamp;
pub mod max;
pub mod pan;
pub mod delay;
pub mod comb_filter;
pub mod min;
pub mod peak_detector;
pub mod envelope_follower;
pub mod tremolo;
pub mod wrap;
pub mod fold;
pub mod sample_hold;
pub mod latch;
pub mod less_than;
pub mod less_than_or_equal;
pub mod greater_than;
pub mod greater_than_or_equal;
pub mod equal_to;
pub mod scale_quantize;
pub mod not_equal_to;
pub mod quantizer;
pub mod decimator;
pub mod quantize;
pub mod rms;
pub mod ar_envelope;
pub mod ad_envelope;
pub mod adsr;
pub mod asr_envelope;
pub mod gate;
pub mod rectifier;
pub mod flanger;
pub mod chorus;
pub mod phaser;
pub mod vibrato;
pub mod slew_limiter;
pub mod sin;
pub mod cos;
pub mod tan;
pub mod impulse;
pub mod frequency_shifter;
pub mod blip;
pub mod ring_mod;
pub mod line;
pub mod lag;
pub mod wavetable;
pub mod xline;
pub mod curve;
pub mod and;
pub mod or;
pub mod xor;
pub mod resonz;
pub mod svf;
pub mod segments;
pub mod not;
pub mod karplus_strong;
pub mod waveguide;
pub mod xfade;
pub mod dattorro_reverb;
pub mod tap;
pub mod reverb;
pub mod resample;
pub mod when;
pub mod parametric_eq;
pub mod pattern_evaluator;
pub mod formant;
pub mod dj_filter;
pub mod pitch_shifter;
pub mod convolution;
pub mod spectral_freeze;
pub mod stereo_widener;
pub mod stereo_splitter;
pub mod auto_pan;
pub mod stereo_merger;
pub mod slice;
pub mod crossover;

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
pub use polyblep_osc::{PolyBLEPOscNode, PolyBLEPWaveform};
pub use brown_noise::BrownNoiseNode;
pub use pink_noise::PinkNoiseNode;
pub use pulse::PulseNode;
pub use noise::NoiseNode;
pub use rlpf::RLPFNode;
pub use random::RandomNode;
pub use fm_oscillator::FMOscillatorNode;
pub use pm_oscillator::PMOscillatorNode;
pub use one_pole_filter::{OnePoleFilterNode, OnePoleMode};
pub use lowpass_filter::LowPassFilterNode;
pub use highpass_filter::HighPassFilterNode;
pub use rhpf::RHPFNode;
pub use pan::PanNode;
pub use tremolo::TremoloNode;
pub use bandpass_filter::BandPassFilterNode;
pub use notch_filter::NotchFilterNode;
pub use allpass_filter::AllPassFilterNode;
pub use hilbert_transformer::HilbertTransformerNode;
pub use moog_ladder::MoogLadderNode;
pub use delay::DelayNode;
pub use comb_filter::CombFilterNode;
pub use clip::ClipNode;
pub use peak_detector::PeakDetectorNode;
pub use envelope_follower::EnvelopeFollowerNode;
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
pub use decimator::DecimatorNode;
pub use quantize::QuantizeNode;
pub use asr_envelope::ASREnvelopeNode;
pub use greater_than::GreaterThanNode;
pub use greater_than_or_equal::GreaterThanOrEqualNode;
pub use equal_to::EqualToNode;
pub use not_equal_to::NotEqualToNode;
pub use quantizer::QuantizerNode;
pub use scale_quantize::ScaleQuantizeNode;
pub use rms::RMSNode;
pub use adsr::ADSRNode;
pub mod limiter;
pub mod compressor;
pub mod expander;
pub mod schmidt;
pub mod timer;
pub mod transient;
pub mod distortion;
pub mod bitcrush;
pub mod biquad;
pub mod multitap_delay;
pub mod noise_gate;
pub use limiter::LimiterNode;
pub use compressor::CompressorNode;
pub use expander::ExpanderNode;
pub use schmidt::SchmidtNode;
pub use timer::TimerNode;
pub use transient::TransientNode;
pub use distortion::DistortionNode;
pub use bitcrush::BitCrushNode;
pub use biquad::{BiquadNode, FilterMode};
pub use multitap_delay::MultiTapDelayNode;
pub use gate::GateNode;
pub use noise_gate::NoiseGateNode;
pub use chorus::ChorusNode;
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
pub use frequency_shifter::FrequencyShifterNode;
pub use lag::LagNode;
pub use wavetable::WavetableNode;
pub use and::AndNode;
pub use xline::XLineNode;
pub use curve::CurveNode;
pub use or::OrNode;
pub use xor::XorNode;
pub use not::NotNode;
pub use karplus_strong::KarplusStrongNode;
pub use waveguide::WaveguideNode;
pub use svf::{SVFNode, SVFMode};
pub use segments::{SegmentsNode, Segment, CurveType};
pub use resonz::ResonzNode;
pub use resample::ResampleNode;
pub use convolution::{ConvolutionNode, create_simple_ir};
pub use xfade::{XFadeNode, XFadeCurve};
pub use tap::TapNode;
pub use reverb::ReverbNode;
pub use dattorro_reverb::DattorroReverbNode;
pub use dj_filter::DJFilterNode;
pub use when::WhenNode;
pub use formant::FormantNode;
pub use parametric_eq::ParametricEQNode;
pub use pitch_shifter::PitchShifterNode;
pub use spectral_freeze::SpectralFreezeNode;
pub mod sample_playback;
pub use sample_playback::SamplePlaybackNode;
pub use pattern_evaluator::PatternEvaluatorNode;
pub mod granular;
pub use stereo_widener::StereoWidenerNode;
pub use granular::GranularNode;
pub mod additive;
pub use additive::AdditiveNode;
pub mod vocoder;
pub use vocoder::VocoderNode;
pub use auto_pan::{AutoPanNode, AutoPanWaveform};
pub use stereo_splitter::StereoSplitterNode;
pub use stereo_merger::StereoMergerNode;
pub use crossover::{CrossoverLowNode, CrossoverMidNode, CrossoverHighNode};
pub use slice::SliceNode;
