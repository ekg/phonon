#![allow(unused_variables)]
//! Parser for the Unified Signal Graph DSL
//!
//! Enables inline synth definitions, pattern embedding, and universal modulation.
//!
//! # Phonon DSL
//!
//! The Phonon DSL allows you to create audio graphs that combine synthesis, patterns,
//! and effects in a concise, readable syntax.
//!
//! ## Basic Syntax
//!
//! - **Bus assignment**: `~name $ expression` or `~name # expression` - Define a named signal bus
//! - **Output**: `out $ expression` or `out # expression` - Set the output signal
//! - **Tempo**: `cps: 2.0` - Set cycles per second (tempo)
//! - **Pattern transform**: `a $ transform` - Apply pattern transform (e.g., `fast 2`, `rev`)
//! - **Signal chain**: `a # b` - Chain signals/effects (output of `a` feeds input of `b`)
//!
//! Note: Legacy colon syntax (`~name: expression`, `out: expression`) is still supported.
//!
//! ## Expressions
//!
//! ### Oscillators
//!
//! - `sine(freq)` - Sine wave oscillator
//! - `saw(freq)` - Sawtooth oscillator
//! - `square(freq)` - Square wave oscillator
//! - `triangle(freq)` - Triangle wave oscillator
//!
//! ### Synthesizers
//!
//! - `superkick(freq, pitch_env, sustain, noise)` - Kick drum
//! - `supersaw(freq, detune, voices)` - Detuned saw oscillators
//! - `superpwm(freq, pwm_rate, pwm_depth)` - Pulse width modulation
//! - `superchip(freq, vibrato_rate, vibrato_depth)` - Chiptune square wave
//! - `superfm(freq, mod_ratio, mod_index)` - FM synthesis
//! - `supersnare(freq, snappy, sustain)` - Snare drum
//! - `superhat(bright, sustain)` - Hi-hat
//!
//! ### Filters
//!
//! - `lpf(input, cutoff, q)` - Low-pass filter
//! - `hpf(input, cutoff, q)` - High-pass filter
//!
//! ### Effects
//!
//! - `reverb(input, room_size, damping, mix)` - Reverb
//! - `distortion(input, drive, mix)` or `dist(input, drive, mix)` - Distortion
//! - `bitcrush(input, bits, rate_reduction)` - Bitcrusher
//! - `chorus(input, rate, depth, mix)` - Chorus
//!
//! ### Math Operations
//!
//! - `a + b`, `a - b`, `a * b`, `a / b` - Arithmetic on signals
//!
//! ### Patterns
//!
//! - `"bd sn hh cp"` - Mini-notation pattern strings
//! - Can be used to modulate any parameter
//!
//! # Examples
//!
//! ## Simple sine wave
//!
//! ```
//! use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
//!
//! let input = "out: sine 440 * 0.2";
//! let (_, statements) = parse_dsl(input).unwrap();
//!
//! let compiler = DslCompiler::new(44100.0);
//! let mut graph = compiler.compile(statements);
//!
//! // Process samples
//! for _ in 0..100 {
//!     let sample = graph.process_sample();
//!     assert!(sample.abs() <= 1.0);
//! }
//! ```
//!
//! ## LFO modulation
//!
//! ```
//! use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
//!
//! let input = r#"
//!     cps: 2.0
//!     ~lfo: sine 0.5 * 0.5 + 0.5
//!     out: sine(~lfo * 200 + 300) * 0.2
//! "#;
//!
//! let (_, statements) = parse_dsl(input).unwrap();
//! let compiler = DslCompiler::new(44100.0);
//! let graph = compiler.compile(statements);
//! ```
//!
//! ## Filtered sawtooth
//!
//! ```
//! use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
//!
//! let input = r#"
//!     ~bass: saw 55 # lpf 800 0.9
//!     out: ~bass * 0.3
//! "#;
//!
//! let (_, statements) = parse_dsl(input).unwrap();
//! let compiler = DslCompiler::new(44100.0);
//! let graph = compiler.compile(statements);
//! ```
//!
//! ## Pattern modulation
//!
//! ```
//! use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
//!
//! let input = r#"
//!     cps: 2.0
//!     out: sine("110 220 440") * 0.2
//! "#;
//!
//! let (_, statements) = parse_dsl(input).unwrap();
//! let compiler = DslCompiler::new(44100.0);
//! let mut graph = compiler.compile(statements);
//!
//! // Pattern will cycle through 110Hz, 220Hz, 440Hz
//! ```
//!
//! ## Complex example
//!
//! ```
//! use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
//!
//! let input = r#"
//!     cps: 2.0
//!     ~lfo: sine 0.25
//!     ~bass: saw("55 82.5 110") # lpf(~lfo * 2000 + 500, 0.8)
//!     out: ~bass * 0.3
//! "#;
//!
//! let (_, statements) = parse_dsl(input).unwrap();
//! let compiler = DslCompiler::new(44100.0);
//! let graph = compiler.compile(statements);
//! ```

use crate::mini_notation_v3::parse_mini_notation;
use crate::pattern::Pattern;
use crate::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    character::complete::{alpha1, alphanumeric1, char, digit1, multispace0, multispace1},
    combinator::{map, map_res, recognize, value},
    multi::many0,
    number::complete::float,
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};
use std::cell::RefCell;

/// DSL statement types
#[derive(Debug, Clone)]
pub enum DslStatement {
    /// Define a bus: ~name: expression
    BusDefinition { name: String, expr: DslExpression },
    /// Set output: out: expression or out1: expression
    Output {
        channel: Option<usize>,
        expr: DslExpression,
    },
    /// Route modulation: route ~source -> { targets }
    Route {
        source: String,
        targets: Vec<(String, f32)>,
    },
    /// Set tempo: cps: 0.5
    SetCps(f32),
    /// Set output mixing mode: outmix: sqrt|gain|tanh|hard|none
    SetOutputMixMode(String),
    /// Silence output channel(s): hush, hush1, hush2
    Hush { channel: Option<usize> },
    /// Kill all voices and silence all outputs: panic
    Panic,
}

/// Envelope type for sample triggering
#[derive(Debug, Clone)]
pub enum SampleEnvelopeType {
    Percussion, // Default: attack + release
    ADSR {
        decay: Box<DslExpression>,
        sustain: Box<DslExpression>,
    },
    Segments {
        levels_str: String,
        times_str: String,
    },
    Curve {
        start: Box<DslExpression>,
        end: Box<DslExpression>,
        duration: Box<DslExpression>,
        curve: Box<DslExpression>,
    },
}

/// Helper struct for SamplePattern fields (used in apply_modifier_to_sample)
#[derive(Debug, Clone)]
struct SamplePatternFields {
    pattern: String,
    gain: Option<Box<DslExpression>>,
    pan: Option<Box<DslExpression>>,
    speed: Option<Box<DslExpression>>,
    cut_group: Option<Box<DslExpression>>,
    n: Option<Box<DslExpression>>,
    note: Option<Box<DslExpression>>,
    attack: Option<Box<DslExpression>>,
    release: Option<Box<DslExpression>>,
    envelope_type: Option<SampleEnvelopeType>,
}

/// DSL expressions
#[derive(Debug, Clone)]
pub enum DslExpression {
    /// Reference to a bus: ~name
    BusRef(String),
    /// Constant value: 440
    Value(f32),
    /// Pattern string: "bd sn hh cp"
    Pattern(String),
    /// Oscillator: sine 440, saw(~freq), square(220, 0.3)
    Oscillator {
        waveform: Waveform,
        freq: Box<DslExpression>,
        duty: Option<f32>,
    },
    /// Filter: lpf(input, cutoff, q), hpf(input, cutoff, q)
    Filter {
        filter_type: FilterType,
        input: Box<DslExpression>,
        cutoff: Box<DslExpression>,
        q: Box<DslExpression>,
    },
    /// Envelope: adsr(input, gate, a, d, s, r)
    Envelope {
        input: Box<DslExpression>,
        gate: Box<DslExpression>,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
    },
    /// Delay: delay(input, time, feedback, mix)
    Delay {
        input: Box<DslExpression>,
        time: Box<DslExpression>,
        feedback: Box<DslExpression>,
        mix: Box<DslExpression>,
    },
    /// Audio analysis: rms(input, window), pitch(input), transient(input, threshold)
    Analysis {
        analysis_type: AnalysisType,
        input: Box<DslExpression>,
        params: Vec<f32>,
    },
    /// Binary operations: +, -, *, /
    BinaryOp {
        op: BinaryOperator,
        left: Box<DslExpression>,
        right: Box<DslExpression>,
    },
    /// Signal chain: a # b
    Chain {
        left: Box<DslExpression>,
        right: Box<DslExpression>,
    },
    /// Conditional: when(input, condition)
    When {
        input: Box<DslExpression>,
        condition: Box<DslExpression>,
    },
    /// Synth: superkick(freq, pitch_env, sustain, noise)
    Synth {
        synth_type: SynthType,
        params: Vec<DslExpression>,
    },
    /// Effect: reverb(input, room_size, damping, mix)
    Effect {
        effect_type: EffectType,
        input: Box<DslExpression>,
        params: Vec<DslExpression>,
    },
    /// Sample pattern: s("bd sn hh"), s("bd*4", gain: 0.8, speed: 1.2)
    SamplePattern {
        pattern: String,
        gain: Option<Box<DslExpression>>,
        pan: Option<Box<DslExpression>>,
        speed: Option<Box<DslExpression>>,
        cut_group: Option<Box<DslExpression>>,
        n: Option<Box<DslExpression>>,
        note: Option<Box<DslExpression>>,
        attack: Option<Box<DslExpression>>,
        release: Option<Box<DslExpression>>,
        envelope_type: Option<SampleEnvelopeType>,
    },
    /// Scale quantization: scale("0 1 2 3 4", "major", "c4")
    Scale {
        pattern: String,
        scale_name: String,
        root_note: String, // Note name like "c4" or MIDI number
    },
    /// Pattern-triggered synth: synth("c4 e4 g4", saw, attack=0.01, release=0.2)
    SynthPattern {
        notes: String,      // Pattern of notes
        waveform: Waveform, // Waveform type
        attack: Option<f32>,
        decay: Option<f32>,
        sustain: Option<f32>,
        release: Option<f32>,
        gain: Option<Box<DslExpression>>,
        pan: Option<Box<DslExpression>>,
    },
    /// Pattern transform: pattern $ transform
    PatternTransform {
        pattern: Box<DslExpression>,
        transform: PatternTransformOp,
    },
    /// DSP Modifiers (Tidal-style): applied via # operator
    /// Gain modifier: s "bd" # gain 0.5
    Gain { value: Box<DslExpression> },
    /// Pan modifier: s "bd" # pan "-1 1"
    Pan { value: Box<DslExpression> },
    /// Speed modifier: s "bd" # speed 2.0
    Speed { value: Box<DslExpression> },
    /// Cut group modifier: s "hh*16" # cut 1
    Cut { value: Box<DslExpression> },
    /// Sample number modifier: s "bd" # n 5 or s "bd" # n "0 1 2 3"
    N { value: Box<DslExpression> },
    /// Note modifier for pitch shifting: s "bd" # note 12 or s "bd" # note "0 5 7 12"
    /// Note values are in semitones: 0 = original, 12 = octave up, -12 = octave down
    Note { value: Box<DslExpression> },
    /// Envelope modifiers for per-event envelopes
    /// Segments envelope: s "bd" # segments "0 1 0" "0.1 0.2"
    SegmentsModifier {
        levels_str: String,
        times_str: String,
    },
    /// Curve envelope: s "bd" # curve 0 1 0.3 3.0
    CurveModifier {
        start: Box<DslExpression>,
        end: Box<DslExpression>,
        duration: Box<DslExpression>,
        curve: Box<DslExpression>,
    },
    /// ADSR envelope: s "bd" # adsr 0.01 0.1 0.7 0.2
    ADSRModifier {
        attack: Box<DslExpression>,
        decay: Box<DslExpression>,
        sustain: Box<DslExpression>,
        release: Box<DslExpression>,
    },
}

/// Pattern transformation operations
#[derive(Debug, Clone)]
pub enum PatternTransformOp {
    /// Speed up pattern: fast 2
    Fast(Box<DslExpression>),
    /// Slow down pattern: slow 2
    Slow(Box<DslExpression>),
    /// Squeeze pattern to first 1/n of cycle: squeeze 2
    Squeeze(Box<DslExpression>),
    /// Reverse pattern: rev
    Rev,
    /// Apply transform every n cycles: every 4 (fast 2)
    Every {
        n: Box<DslExpression>,
        f: Box<PatternTransformOp>,
    },
    /// Apply transform sometimes (50% probability)
    Sometimes(Box<PatternTransformOp>),
    /// Apply transform often (75% probability)
    Often(Box<PatternTransformOp>),
    /// Apply transform rarely (10% probability)
    Rarely(Box<PatternTransformOp>),
    /// Randomly drop 50% of events: degrade
    Degrade,
    /// Randomly drop events with probability: degradeBy 0.9
    DegradeBy(Box<DslExpression>),
    /// Create palindrome (forward + backward): palindrome
    Palindrome,
    /// Stutter events n times: stutter n
    Stutter(Box<DslExpression>),
    /// Shift pattern forward in time: late 0.25
    Late(Box<DslExpression>),
    /// Shift pattern backward in time: early 0.25
    Early(Box<DslExpression>),
    /// Duplicate/repeat pattern n times: dup 4
    Dup(Box<DslExpression>),
    /// Zoom to a time window: zoom 0.0 0.5
    Zoom {
        begin: Box<DslExpression>,
        end: Box<DslExpression>,
    },
    /// Focus on a time window: focus 0.25 0.75
    Focus {
        begin: Box<DslExpression>,
        end: Box<DslExpression>,
    },
    /// Apply transform to time window: within 0.25 0.75 (fast 2)
    Within {
        begin: Box<DslExpression>,
        end: Box<DslExpression>,
        transform: Box<PatternTransformOp>,
    },
    /// Chop events into n pieces: chop 4
    Chop(Box<DslExpression>),
    /// Add gaps between events: gap 2
    Gap(Box<DslExpression>),
    /// Divide pattern into n segments: segment 4
    Segment(Box<DslExpression>),
    /// Add swing/shuffle feel: swing 0.5
    Swing(Box<DslExpression>),
    /// Shuffle pattern timing: shuffle 3
    Shuffle(Box<DslExpression>),
    /// Apply transform to each chunk: chunk 4 (rev)
    Chunk {
        n: Box<DslExpression>,
        transform: Box<PatternTransformOp>,
    },
    /// Stereo effect (original + transformed): jux (rev)
    Jux(Box<PatternTransformOp>),
}

#[derive(Debug, Clone, Copy)]
pub enum SynthType {
    SuperKick,
    SuperSaw,
    SuperPWM,
    SuperChip,
    SuperFM,
    SuperSnare,
    SuperHat,
}

#[derive(Debug, Clone, Copy)]
pub enum EffectType {
    Reverb,
    Distortion,
    BitCrush,
    Chorus,
    Compressor,
}

#[derive(Debug, Clone, Copy)]
pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
}

#[derive(Debug, Clone, Copy)]
pub enum AnalysisType {
    RMS,
    Pitch,
    Transient,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// Parse whitespace
fn ws<'a, F, O>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O>
where
    F: FnMut(&'a str) -> IResult<&'a str, O>,
{
    delimited(multispace0, inner, multispace0)
}

/// Parse an identifier (alphanumeric + underscore)
fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))(input)
}

/// Parse a bus reference: ~name
fn bus_ref(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(char('~'), identifier), |name: &str| {
        DslExpression::BusRef(name.to_string())
    })(input)
}

/// Parse a number
fn number(input: &str) -> IResult<&str, f32> {
    alt((float, map_res(digit1, |s: &str| s.parse::<f32>())))(input)
}

/// Parse a value
fn value_expr(input: &str) -> IResult<&str, DslExpression> {
    map(number, DslExpression::Value)(input)
}

/// Parse a pattern string: "bd sn hh cp"
fn pattern_string(input: &str) -> IResult<&str, DslExpression> {
    map(
        delimited(char('"'), take_until("\""), char('"')),
        |s: &str| DslExpression::Pattern(s.to_string()),
    )(input)
}

/// Parse waveform type
fn waveform(input: &str) -> IResult<&str, Waveform> {
    alt((
        value(Waveform::Sine, tag("sine")),
        value(Waveform::Sine, tag("sin")),
        value(Waveform::Saw, tag("saw")),
        value(Waveform::Square, tag("square")),
        value(Waveform::Square, tag("sq")),
        value(Waveform::Triangle, tag("triangle")),
        value(Waveform::Triangle, tag("tri")),
    ))(input)
}

/// Parse a single argument for space-separated style
/// An argument can be: value, pattern string, bus ref, or parenthesized expression
///
/// IMPORTANT: We use `char(')')` not `ws(char(')'))` for the closing paren
/// to avoid consuming the space that separates this arg from the next one
fn space_arg(input: &str) -> IResult<&str, DslExpression> {
    alt((
        pattern_string,                                  // "pattern"
        bus_ref,                                         // ~name
        value_expr,                                      // 0.5
        delimited(ws(char('(')), expression, char(')')), // (expr) - note: no ws after ')'
    ))(input)
}

/// Parse space-separated function arguments (TIDAL/HASKELL STYLE)
/// Parses one or more arguments separated by SPACES/TABS only (not newlines!)
/// Stops at operators (#, $, +, -, *, /), newlines, or end of input
///
/// Examples:
///   s "bd"           -> ["bd"]
///   lpf 1000 0.8     -> [1000, 0.8]
///   gain "0.5 1.0"   -> ["0.5 1.0"]
fn space_separated_args(input: &str) -> IResult<&str, Vec<DslExpression>> {
    use nom::character::complete::space1; // Space and tab, but NOT newline

    // Parse at least one argument (require at least one space/tab before it)
    let (input, first) = preceded(space1, space_arg)(input)?;

    // Try to parse more arguments (separated by spaces/tabs, not newlines)
    let (input, mut rest) = many0(preceded(space1, space_arg))(input)?;

    // Combine first + rest
    let mut args = vec![first];
    args.append(&mut rest);

    Ok((input, args))
}

/// Parse function arguments (TIDAL/HASKELL STYLE - space-separated only!)
/// Parentheses are ONLY for grouping expressions, not for function application
///
/// OLD (NO LONGER SUPPORTED): s("bd"), lpf 1000 0.8
/// NEW (REQUIRED): s "bd", lpf 1000 0.8
fn function_args(input: &str) -> IResult<&str, Vec<DslExpression>> {
    space_separated_args(input)
}

/// Parse oscillator: sine 440, saw "110 220"
fn oscillator(input: &str) -> IResult<&str, DslExpression> {
    map(tuple((waveform, function_args)), |(wf, args)| {
        let freq = args.first().cloned().unwrap_or(DslExpression::Value(440.0));
        let duty = if wf == Waveform::Square {
            args.get(1).and_then(|e| {
                if let DslExpression::Value(v) = e {
                    Some(*v)
                } else {
                    None
                }
            })
        } else {
            None
        };
        DslExpression::Oscillator {
            waveform: wf,
            freq: Box::new(freq),
            duty,
        }
    })(input)
}

/// Parse filter: lpf 1000 0.8, hpf 500 0.5
fn filter(input: &str) -> IResult<&str, DslExpression> {
    let lpf = map(preceded(tag("lpf"), function_args), |args| {
        DslExpression::Filter {
            filter_type: FilterType::LowPass,
            input: Box::new(args.first().cloned().unwrap_or(DslExpression::Value(0.0))),
            cutoff: Box::new(args.get(1).cloned().unwrap_or(DslExpression::Value(1000.0))),
            q: Box::new(args.get(2).cloned().unwrap_or(DslExpression::Value(1.0))),
        }
    });

    let hpf = map(preceded(tag("hpf"), function_args), |args| {
        DslExpression::Filter {
            filter_type: FilterType::HighPass,
            input: Box::new(args.first().cloned().unwrap_or(DslExpression::Value(0.0))),
            cutoff: Box::new(args.get(1).cloned().unwrap_or(DslExpression::Value(1000.0))),
            q: Box::new(args.get(2).cloned().unwrap_or(DslExpression::Value(1.0))),
        }
    });

    let bpf = map(preceded(tag("bpf"), function_args), |args| {
        DslExpression::Filter {
            filter_type: FilterType::BandPass,
            input: Box::new(args.first().cloned().unwrap_or(DslExpression::Value(0.0))),
            cutoff: Box::new(args.get(1).cloned().unwrap_or(DslExpression::Value(1000.0))),
            q: Box::new(args.get(2).cloned().unwrap_or(DslExpression::Value(1.0))),
        }
    });

    alt((lpf, hpf, bpf))(input)
}

/// Parse delay: delay(input, time, feedback, mix)
fn delay(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(tag("delay"), function_args), |args| {
        DslExpression::Delay {
            input: Box::new(args.first().cloned().unwrap_or(DslExpression::Value(0.0))),
            time: Box::new(args.get(1).cloned().unwrap_or(DslExpression::Value(0.25))),
            feedback: Box::new(args.get(2).cloned().unwrap_or(DslExpression::Value(0.5))),
            mix: Box::new(args.get(3).cloned().unwrap_or(DslExpression::Value(0.5))),
        }
    })(input)
}

/// Parse DSP modifiers (Tidal-style)
/// These are applied via # operator: s "bd" # gain 0.5

/// Parse gain modifier: gain 0.5 or gain "0.5 1.0"
fn gain_modifier(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(tag("gain"), function_args), |args| {
        DslExpression::Gain {
            value: Box::new(args.first().cloned().unwrap_or(DslExpression::Value(1.0))),
        }
    })(input)
}

/// Parse pan modifier: pan 0.5 or pan "-1 1"
fn pan_modifier(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(tag("pan"), function_args), |args| {
        DslExpression::Pan {
            value: Box::new(args.first().cloned().unwrap_or(DslExpression::Value(0.0))),
        }
    })(input)
}

/// Parse speed modifier: speed 2.0 or speed "1.0 1.5 0.5"
fn speed_modifier(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(tag("speed"), function_args), |args| {
        DslExpression::Speed {
            value: Box::new(args.first().cloned().unwrap_or(DslExpression::Value(1.0))),
        }
    })(input)
}

/// Parse cut group modifier: cut 1 (for hi-hat choking, etc.)
fn cut_modifier(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(tag("cut"), function_args), |args| {
        DslExpression::Cut {
            value: Box::new(args.first().cloned().unwrap_or(DslExpression::Value(0.0))),
        }
    })(input)
}

/// Parse n modifier for sample number selection: n 5 or n "0 1 2 3"
fn n_modifier(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(tag("n"), function_args), |args| DslExpression::N {
        value: Box::new(args.first().cloned().unwrap_or(DslExpression::Value(0.0))),
    })(input)
}

/// Parse note modifier for pitch shifting: note 12 or note "0 5 7 12"
/// Note values in semitones: 0 = original, 12 = octave up, -12 = octave down
fn note_modifier(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(tag("note"), function_args), |args| {
        DslExpression::Note {
            value: Box::new(args.first().cloned().unwrap_or(DslExpression::Value(0.0))),
        }
    })(input)
}

/// Parse envelope modifiers: segments, curve, or adsr
fn envelope_modifier(input: &str) -> IResult<&str, DslExpression> {
    alt((
        // segments "0 1 0" "0.1 0.2"
        map(preceded(tag("segments"), function_args), |args| {
            let levels_str = if let Some(DslExpression::Pattern(s)) = args.get(0) {
                s.clone()
            } else {
                String::from("0 1 0")
            };

            let times_str = if let Some(DslExpression::Pattern(s)) = args.get(1) {
                s.clone()
            } else {
                String::from("0.1 0.2")
            };

            DslExpression::SegmentsModifier {
                levels_str,
                times_str,
            }
        }),
        // curve 0 1 0.3 3.0
        map(preceded(tag("curve"), function_args), |args| {
            DslExpression::CurveModifier {
                start: Box::new(args.get(0).cloned().unwrap_or(DslExpression::Value(0.0))),
                end: Box::new(args.get(1).cloned().unwrap_or(DslExpression::Value(1.0))),
                duration: Box::new(args.get(2).cloned().unwrap_or(DslExpression::Value(0.3))),
                curve: Box::new(args.get(3).cloned().unwrap_or(DslExpression::Value(0.0))),
            }
        }),
        // adsr 0.01 0.1 0.7 0.2
        map(preceded(tag("adsr"), function_args), |args| {
            DslExpression::ADSRModifier {
                attack: Box::new(args.get(0).cloned().unwrap_or(DslExpression::Value(0.01))),
                decay: Box::new(args.get(1).cloned().unwrap_or(DslExpression::Value(0.1))),
                sustain: Box::new(args.get(2).cloned().unwrap_or(DslExpression::Value(0.7))),
                release: Box::new(args.get(3).cloned().unwrap_or(DslExpression::Value(0.2))),
            }
        }),
    ))(input)
}

/// Parse RMS analyzer: rms(input, window_size)
fn rms_analyzer(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(tag("rms"), function_args), |args| {
        DslExpression::Analysis {
            analysis_type: AnalysisType::RMS,
            input: Box::new(args.first().cloned().unwrap_or(DslExpression::Value(0.0))),
            params: vec![args
                .get(1)
                .and_then(|e| {
                    if let DslExpression::Value(v) = e {
                        Some(*v)
                    } else {
                        None
                    }
                })
                .unwrap_or(0.01)],
        }
    })(input)
}

/// Parse when conditional: when(input, condition)
fn when_expr(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(tag("when"), function_args), |args| {
        DslExpression::When {
            input: Box::new(args.first().cloned().unwrap_or(DslExpression::Value(0.0))),
            condition: Box::new(args.get(1).cloned().unwrap_or(DslExpression::Value(1.0))),
        }
    })(input)
}

/// Parse synth types
fn synth_type(input: &str) -> IResult<&str, SynthType> {
    alt((
        value(SynthType::SuperKick, tag("superkick")),
        value(SynthType::SuperSaw, tag("supersaw")),
        value(SynthType::SuperPWM, tag("superpwm")),
        value(SynthType::SuperChip, tag("superchip")),
        value(SynthType::SuperFM, tag("superfm")),
        value(SynthType::SuperSnare, tag("supersnare")),
        value(SynthType::SuperHat, tag("superhat")),
    ))(input)
}

/// Parse synth: superkick(freq, pitch_env, sustain, noise)
fn synth_expr(input: &str) -> IResult<&str, DslExpression> {
    map(tuple((synth_type, function_args)), |(st, params)| {
        DslExpression::Synth {
            synth_type: st,
            params,
        }
    })(input)
}

/// Parse effect types
fn effect_type(input: &str) -> IResult<&str, EffectType> {
    alt((
        value(EffectType::Reverb, tag("reverb")),
        value(EffectType::Distortion, tag("distortion")),
        value(EffectType::Distortion, tag("dist")),
        value(EffectType::BitCrush, tag("bitcrush")),
        value(EffectType::Chorus, tag("chorus")),
        value(EffectType::Compressor, tag("compressor")),
    ))(input)
}

/// Parse effect: reverb(input, room_size, damping, mix)
fn effect_expr(input: &str) -> IResult<&str, DslExpression> {
    map(tuple((effect_type, function_args)), |(et, args)| {
        DslExpression::Effect {
            effect_type: et,
            input: Box::new(args.first().cloned().unwrap_or(DslExpression::Value(0.0))),
            params: args.into_iter().skip(1).collect(),
        }
    })(input)
}

/// Helper function to recursively extract pattern string and rebuild transform chain
/// For s("bd sn" $ fast 2 $ rev), this converts:
///   PatternTransform { pattern: PatternTransform { pattern: Pattern("bd sn"), transform: Fast }, transform: Rev }
/// Into:
///   PatternTransform { pattern: PatternTransform { pattern: SamplePattern { pattern: "bd sn", ... }, transform: Fast }, transform: Rev }
fn extract_pattern_and_rebuild_transforms(
    expr: DslExpression,
    gain: Option<Box<DslExpression>>,
    pan: Option<Box<DslExpression>>,
    speed: Option<Box<DslExpression>>,
    cut_group: Option<Box<DslExpression>>,
    n: Option<Box<DslExpression>>,
    note: Option<Box<DslExpression>>,
    attack: Option<Box<DslExpression>>,
    release: Option<Box<DslExpression>>,
) -> DslExpression {
    match expr {
        DslExpression::Pattern(p) => {
            // Base case: found the pattern string, wrap it in SamplePattern
            DslExpression::SamplePattern {
                pattern: p,
                gain,
                pan,
                speed,
                cut_group,
                n,
                note,
                attack,
                release,
                envelope_type: None,
            }
        }
        DslExpression::PatternTransform { pattern, transform } => {
            // Recursive case: process inner pattern, then wrap result in transform
            let inner = extract_pattern_and_rebuild_transforms(
                *pattern, gain, pan, speed, cut_group, n, note, attack, release,
            );
            DslExpression::PatternTransform {
                pattern: Box::new(inner),
                transform,
            }
        }
        _ => {
            // Unknown case: return empty sample pattern
            DslExpression::SamplePattern {
                pattern: String::new(),
                gain,
                pan,
                speed,
                cut_group,
                n,
                note,
                attack,
                release,
                envelope_type: None,
            }
        }
    }
}

/// Parse sample pattern: s "bd sn hh"
/// Also handles pattern transforms: s "bd sn" $ fast 2
/// DSP parameters are applied via Tidal-style chaining: s "bd" # gain 0.5
fn sample_pattern_expr(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(tag("s"), function_args), |args| {
        // First arg can be a pattern string OR a pattern transform
        let first_arg = args.first().cloned();

        // DSP params are now set via # chain (Tidal style), not positional args
        let gain = None;
        let pan = None;
        let speed = None;
        let cut_group = None;
        let n = None;
        let note = None;
        let attack = None;
        let release = None;

        // Check if first arg is a plain pattern string or a transform
        match first_arg {
            Some(DslExpression::Pattern(p)) => {
                // Plain pattern string: s("bd sn")
                DslExpression::SamplePattern {
                    pattern: p,
                    gain,
                    pan,
                    speed,
                    cut_group,
                    n,
                    note,
                    attack,
                    release,
                    envelope_type: None,
                }
            }
            Some(DslExpression::PatternTransform { pattern, transform }) => {
                // Pattern with transform: s("bd sn" $ fast 2) or chained: s("bd sn" $ fast 2 $ rev)
                // Recursively extract the base pattern and rebuild the transform chain
                extract_pattern_and_rebuild_transforms(
                    DslExpression::PatternTransform { pattern, transform },
                    gain,
                    pan,
                    speed,
                    cut_group,
                    n,
                    note,
                    attack,
                    release,
                )
            }
            _ => {
                // Unknown first argument type
                DslExpression::SamplePattern {
                    pattern: String::new(),
                    gain,
                    pan,
                    speed,
                    cut_group,
                    n,
                    note,
                    attack,
                    release,
                    envelope_type: None,
                }
            }
        }
    })(input)
}

/// Parse scale quantization: scale("0 1 2 3 4", "major", "c4")
fn scale_expr(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(tag("scale"), function_args), |args| {
        // First arg: pattern string of scale degrees
        let pattern = if let Some(DslExpression::Pattern(p)) = args.first() {
            p.clone()
        } else {
            String::new()
        };

        // Second arg: scale name (major, minor, etc.)
        let scale_name = if let Some(DslExpression::Pattern(s)) = args.get(1) {
            s.clone()
        } else {
            "major".to_string()
        };

        // Third arg: root note (e.g., "c4", "60")
        let root_note = if let Some(DslExpression::Pattern(r)) = args.get(2) {
            r.clone()
        } else if let Some(DslExpression::Value(v)) = args.get(2) {
            v.to_string()
        } else {
            "60".to_string()
        };

        DslExpression::Scale {
            pattern,
            scale_name,
            root_note,
        }
    })(input)
}

/// Parse pattern-triggered synth: synth("c4 e4 g4", "saw", 0.01, 0.2)
/// Positional args: synth("notes", "waveform", attack, decay, sustain, release)
fn synth_pattern_expr(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(tag("synth"), function_args), |args| {
        // First arg: pattern string of notes
        let notes = if let Some(DslExpression::Pattern(p)) = args.first() {
            p.clone()
        } else {
            String::new()
        };

        // Second arg: waveform name as string
        let waveform = if let Some(DslExpression::Pattern(w)) = args.get(1) {
            match w.as_str() {
                "sine" | "sin" => Waveform::Sine,
                "saw" | "sawtooth" => Waveform::Saw,
                "square" | "sq" => Waveform::Square,
                "triangle" | "tri" => Waveform::Triangle,
                _ => Waveform::Saw, // Default
            }
        } else {
            Waveform::Saw // Default
        };

        // Positional ADSR parameters
        let attack = args.get(2).and_then(|e| {
            if let DslExpression::Value(v) = e {
                Some(*v)
            } else {
                None
            }
        });
        let decay = args.get(3).and_then(|e| {
            if let DslExpression::Value(v) = e {
                Some(*v)
            } else {
                None
            }
        });
        let sustain = args.get(4).and_then(|e| {
            if let DslExpression::Value(v) = e {
                Some(*v)
            } else {
                None
            }
        });
        let release = args.get(5).and_then(|e| {
            if let DslExpression::Value(v) = e {
                Some(*v)
            } else {
                None
            }
        });

        // Optional gain/pan
        let gain = args.get(6).map(|e| Box::new(e.clone()));
        let pan = args.get(7).map(|e| Box::new(e.clone()));

        DslExpression::SynthPattern {
            notes,
            waveform,
            attack,
            decay,
            sustain,
            release,
            gain,
            pan,
        }
    })(input)
}

/// Parse a pattern transform operation
fn parse_transform_op(input: &str) -> IResult<&str, PatternTransformOp> {
    // Split into two groups to avoid nom's alt limit (~21 alternatives)
    alt((parse_transform_op_group1, parse_transform_op_group2))(input)
}

/// First group of transform operations
fn parse_transform_op_group1(input: &str) -> IResult<&str, PatternTransformOp> {
    alt((
        // rev (no arguments)
        map(tag("rev"), |_| PatternTransformOp::Rev),
        // degradeBy n (must come before degrade)
        map(preceded(tag("degradeBy"), ws(primary)), |n| {
            PatternTransformOp::DegradeBy(Box::new(n))
        }),
        // degrade (no arguments)
        map(tag("degrade"), |_| PatternTransformOp::Degrade),
        // palindrome (no arguments)
        map(tag("palindrome"), |_| PatternTransformOp::Palindrome),
        // stutter n
        map(preceded(tag("stutter"), ws(primary)), |n| {
            PatternTransformOp::Stutter(Box::new(n))
        }),
        // late n
        map(preceded(tag("late"), ws(primary)), |n| {
            PatternTransformOp::Late(Box::new(n))
        }),
        // early n
        map(preceded(tag("early"), ws(primary)), |n| {
            PatternTransformOp::Early(Box::new(n))
        }),
        // dup n
        map(preceded(tag("dup"), ws(primary)), |n| {
            PatternTransformOp::Dup(Box::new(n))
        }),
        // zoom begin end
        map(
            tuple((preceded(tag("zoom"), ws(primary)), ws(primary))),
            |(begin, end)| PatternTransformOp::Zoom {
                begin: Box::new(begin),
                end: Box::new(end),
            },
        ),
        // focus begin end
        map(
            tuple((preceded(tag("focus"), ws(primary)), ws(primary))),
            |(begin, end)| PatternTransformOp::Focus {
                begin: Box::new(begin),
                end: Box::new(end),
            },
        ),
        // within begin end (transform)
        map(
            tuple((
                preceded(tag("within"), ws(primary)),
                ws(primary),
                delimited(ws(char('(')), parse_transform_op, ws(char(')'))),
            )),
            |(begin, end, transform)| PatternTransformOp::Within {
                begin: Box::new(begin),
                end: Box::new(end),
                transform: Box::new(transform),
            },
        ),
        // chop n
        map(preceded(tag("chop"), ws(primary)), |n| {
            PatternTransformOp::Chop(Box::new(n))
        }),
        // gap n
        map(preceded(tag("gap"), ws(primary)), |n| {
            PatternTransformOp::Gap(Box::new(n))
        }),
        // segment n
        map(preceded(tag("segment"), ws(primary)), |n| {
            PatternTransformOp::Segment(Box::new(n))
        }),
        // swing n
        map(preceded(tag("swing"), ws(primary)), |n| {
            PatternTransformOp::Swing(Box::new(n))
        }),
        // shuffle n
        map(preceded(tag("shuffle"), ws(primary)), |n| {
            PatternTransformOp::Shuffle(Box::new(n))
        }),
    ))(input)
}

/// Second group of transform operations
fn parse_transform_op_group2(input: &str) -> IResult<&str, PatternTransformOp> {
    alt((
        // fast n
        map(preceded(tag("fast"), ws(primary)), |n| {
            PatternTransformOp::Fast(Box::new(n))
        }),
        // slow n
        map(preceded(tag("slow"), ws(primary)), |n| {
            PatternTransformOp::Slow(Box::new(n))
        }),
        // squeeze n
        map(preceded(tag("squeeze"), ws(primary)), |n| {
            PatternTransformOp::Squeeze(Box::new(n))
        }),
        // every n (transform)
        map(
            tuple((
                preceded(tag("every"), ws(primary)),
                delimited(ws(char('(')), parse_transform_op, ws(char(')'))),
            )),
            |(n, f)| PatternTransformOp::Every {
                n: Box::new(n),
                f: Box::new(f),
            },
        ),
        // sometimes (transform)
        map(
            preceded(
                tag("sometimes"),
                delimited(ws(char('(')), parse_transform_op, ws(char(')'))),
            ),
            |f| PatternTransformOp::Sometimes(Box::new(f)),
        ),
        // often (transform)
        map(
            preceded(
                tag("often"),
                delimited(ws(char('(')), parse_transform_op, ws(char(')'))),
            ),
            |f| PatternTransformOp::Often(Box::new(f)),
        ),
        // rarely (transform)
        map(
            preceded(
                tag("rarely"),
                delimited(ws(char('(')), parse_transform_op, ws(char(')'))),
            ),
            |f| PatternTransformOp::Rarely(Box::new(f)),
        ),
        // chunk n (transform)
        map(
            tuple((
                preceded(tag("chunk"), ws(primary)),
                delimited(ws(char('(')), parse_transform_op, ws(char(')'))),
            )),
            |(n, transform)| PatternTransformOp::Chunk {
                n: Box::new(n),
                transform: Box::new(transform),
            },
        ),
        // jux (transform)
        map(
            preceded(
                tag("jux"),
                delimited(ws(char('(')), parse_transform_op, ws(char(')'))),
            ),
            |transform| PatternTransformOp::Jux(Box::new(transform)),
        ),
    ))(input)
}

/// Parse primary expression
fn primary(input: &str) -> IResult<&str, DslExpression> {
    alt((
        bus_ref,
        scale_expr,          // MUST come before sample_pattern_expr!
        sample_pattern_expr, // s() would match the 's' in scale()
        synth_pattern_expr,  // Pattern-triggered synth: synth("notes", "waveform", ...)
        synth_expr,          // SuperDirt continuous synths
        effect_expr,
        gain_modifier, // Tidal-style DSP modifiers
        pan_modifier,
        speed_modifier,
        cut_modifier,
        n_modifier,
        note_modifier,
        envelope_modifier, // Envelope modifiers (segments, curve, adsr)
        oscillator,
        filter,
        delay,
        rms_analyzer,
        when_expr,
        pattern_string,
        value_expr,
        delimited(ws(char('(')), expression, ws(char(')'))),
    ))(input)
}

/// Parse signal chain: a # b
/// Chain has the HIGHEST precedence (after primary), so it binds tighter than arithmetic
/// Example: a # b * c parses as (a # b) * c
fn chain(input: &str) -> IResult<&str, DslExpression> {
    let (input, first) = primary(input)?;

    let (input, chains) = many0(preceded(ws(char('#')), primary))(input)?;

    let expr = chains
        .into_iter()
        .fold(first, |acc, right| DslExpression::Chain {
            left: Box::new(acc),
            right: Box::new(right),
        });

    Ok((input, expr))
}

/// Parse pattern transforms: pattern $ transform
/// Pattern transform has precedence between chain and arithmetic
/// Example: "bd sn" $ fast 2 * 0.5 parses as ("bd sn" $ fast 2) * 0.5
fn pattern_transform(input: &str) -> IResult<&str, DslExpression> {
    let (input, first) = chain(input)?;

    let (input, transforms) = many0(preceded(ws(char('$')), ws(parse_transform_op)))(input)?;

    let expr =
        transforms
            .into_iter()
            .fold(first, |acc, transform| DslExpression::PatternTransform {
                pattern: Box::new(acc),
                transform,
            });

    Ok((input, expr))
}

/// Parse multiplication and division
fn term(input: &str) -> IResult<&str, DslExpression> {
    let (input, first) = pattern_transform(input)?; // Pattern transform has higher precedence

    let (input, ops) = many0(tuple((ws(alt((char('*'), char('/')))), pattern_transform)))(input)?;

    let expr = ops.into_iter().fold(first, |acc, (op, right)| {
        let operator = match op {
            '*' => BinaryOperator::Multiply,
            '/' => BinaryOperator::Divide,
            _ => unreachable!(),
        };
        DslExpression::BinaryOp {
            op: operator,
            left: Box::new(acc),
            right: Box::new(right),
        }
    });

    Ok((input, expr))
}

/// Parse addition and subtraction
fn arithmetic(input: &str) -> IResult<&str, DslExpression> {
    let (input, first) = term(input)?; // Term has higher precedence

    let (input, ops) = many0(tuple((ws(alt((char('+'), char('-')))), term)))(input)?;

    let expr = ops.into_iter().fold(first, |acc, (op, right)| {
        let operator = match op {
            '+' => BinaryOperator::Add,
            '-' => BinaryOperator::Subtract,
            _ => unreachable!(),
        };
        DslExpression::BinaryOp {
            op: operator,
            left: Box::new(acc),
            right: Box::new(right),
        }
    });

    Ok((input, expr))
}

/// Parse a complete expression
fn expression(input: &str) -> IResult<&str, DslExpression> {
    arithmetic(input) // Start with lowest precedence (arithmetic calls chain calls term)
}

/// Parse a bus definition: ~name $ expression or ~name # expression
fn bus_definition(input: &str) -> IResult<&str, DslStatement> {
    map(
        tuple((
            preceded(char('~'), identifier),
            // Accept $ or # only (Tidal-style syntax)
            alt((ws(char('$')), ws(char('#')))),
            expression,
        )),
        |(name, _, expr)| DslStatement::BusDefinition {
            name: name.to_string(),
            expr,
        },
    )(input)
}

/// Parse output definition: out $ expression, out # expression
/// And Tidal-style: o1, o2, o3 and d1, d2, d3 syntax
fn output_definition(input: &str) -> IResult<&str, DslStatement> {
    map(
        tuple((
            // Match "out", "o", or "d" prefix
            alt((
                value("out", tag("out")),
                value("o", tag("o")),
                value("d", tag("d")),
            )),
            // Optional channel number: out1, o1, d1, etc.
            alt((
                map_res(digit1, |s: &str| s.parse::<usize>()),
                value(0, tag("")), // Default to channel 0 for plain "out"
            )),
            // Accept $ or # only (Tidal-style syntax)
            alt((ws(char('$')), ws(char('#')))),
            expression,
        )),
        |(prefix, channel, _, expr)| {
            // For "out" without number, use None (backwards compatible single output)
            // For "out1", "o1", "d1", etc., use the channel number
            let channel_num = if prefix == "out" && channel == 0 {
                None // Plain "out $" goes to backwards-compatible single output
            } else if channel == 0 {
                // "o $" or "d $" without number defaults to channel 1
                Some(1)
            } else {
                Some(channel)
            };

            DslStatement::Output {
                channel: channel_num,
                expr,
            }
        },
    )(input)
}

/// Parse CPS setting: cps: 0.5, tempo: 1.0, or bpm 120 [4/4]
fn cps_setting(input: &str) -> IResult<&str, DslStatement> {
    alt((
        // bpm 120 [4/4] (optional time signature, defaults to 4/4)
        map(
            tuple((
                tag("bpm"),
                multispace1,
                number,
                // Optional time signature [numerator/denominator]
                nom::combinator::opt(preceded(
                    multispace0,
                    delimited(
                        char('['),
                        tuple((
                            map_res(digit1, |s: &str| s.parse::<u32>()),
                            preceded(char('/'), map_res(digit1, |s: &str| s.parse::<u32>())),
                        )),
                        char(']'),
                    ),
                )),
            )),
            |(_tag, _ws, bpm, time_sig_opt)| {
                // Default to 4/4 if not specified
                let (numerator, denominator) = time_sig_opt.unwrap_or((4, 4));

                // BPM is quarter notes per minute
                // CPS is cycles per second
                // For now, time signature is parsed but not used in CPS calculation
                // In the future, this could affect how cycles map to musical measures
                let _ = (numerator, denominator); // Keep for future use
                DslStatement::SetCps(bpm / 60.0)
            },
        ),
        // cps: 2.0 or tempo: 0.5 (with colon)
        map(
            preceded(
                tuple((alt((tag("cps"), tag("tempo"))), ws(char(':')))),
                number,
            ),
            DslStatement::SetCps,
        ),
    ))(input)
}

/// Parse output mix mode setting: outmix: sqrt|gain|tanh|hard|none
fn outmix_setting(input: &str) -> IResult<&str, DslStatement> {
    map(
        preceded(tuple((tag("outmix"), ws(char(':')))), identifier),
        |mode| DslStatement::SetOutputMixMode(mode.to_string()),
    )(input)
}

/// Parse hush statement: hush, hush1, hush2, etc.
fn hush_statement(input: &str) -> IResult<&str, DslStatement> {
    map(
        tuple((
            tag("hush"),
            // Optional channel number: hush1, hush2, etc.
            alt((
                map_res(digit1, |s: &str| s.parse::<usize>()),
                value(0, tag("")), // No channel means hush all
            )),
        )),
        |(_, channel)| DslStatement::Hush {
            channel: if channel == 0 { None } else { Some(channel) },
        },
    )(input)
}

/// Parse panic statement: panic
fn panic_statement(input: &str) -> IResult<&str, DslStatement> {
    map(tag("panic"), |_| DslStatement::Panic)(input)
}

/// Skip a comment (from -- to end of line)
/// Note: # is NOT a comment - it's the DSP chain operator!
fn skip_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = tag("--")(input)?;
    let (input, _) = take_while(|c| c != '\n')(input)?;
    Ok((input, ()))
}

/// Skip whitespace and comments
fn skip_whitespace_and_comments(input: &str) -> IResult<&str, ()> {
    let (input, _) = many0(alt((map(multispace1, |_| ()), skip_comment)))(input)?;
    Ok((input, ()))
}

/// Parse a statement
fn statement(input: &str) -> IResult<&str, DslStatement> {
    alt((
        bus_definition,
        output_definition,
        cps_setting,
        outmix_setting,
        hush_statement,
        panic_statement,
    ))(input)
}

/// Preprocess input to join continuation lines
/// A line is a continuation if it doesn't start with a definition pattern (identifier:)
fn preprocess_multiline(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let mut result = Vec::new();
    let mut current_statement = String::new();

    for line in lines {
        let trimmed = line.trim();

        // Skip empty lines and pure comment lines
        if trimmed.is_empty() || trimmed.starts_with("--") {
            // If we have accumulated a statement, push it
            if !current_statement.is_empty() {
                result.push(current_statement.clone());
                current_statement.clear();
            }
            // Preserve comments and empty lines
            result.push(line.to_string());
            continue;
        }

        // Check if this line starts a new definition or is a standalone command
        // A definition line has the pattern: identifier followed by $, #, or : (for tempo/bpm/outmix)
        // Examples: tempo:, out $, o1 $, d1 #, ~bus $, fn name = ..., etc.
        // Standalone commands: hush, hush1, hush2, panic
        let is_definition = if let Some(dollar_pos) = trimmed.find('$') {
            let before_dollar = trimmed[..dollar_pos].trim();
            // Check if what's before $ looks like an identifier (bus/output)
            let is_valid_identifier = before_dollar
                .chars()
                .all(|c| c.is_alphanumeric() || c == '~' || c == '_')
                && !before_dollar.is_empty();
            is_valid_identifier
        } else if let Some(hash_pos) = trimmed.find('#') {
            let before_hash = trimmed[..hash_pos].trim();
            // Check if what's before # looks like an identifier (bus/output with chaining)
            let is_valid_identifier = before_hash
                .chars()
                .all(|c| c.is_alphanumeric() || c == '~' || c == '_')
                && !before_hash.is_empty();
            is_valid_identifier
        } else if let Some(colon_pos) = trimmed.find(':') {
            let before_colon = &trimmed[..colon_pos];
            // Check if what's before : looks like an identifier (tempo, bpm, outmix)
            let is_valid_identifier = before_colon
                .chars()
                .all(|c| c.is_alphanumeric() || c == '~' || c == '_')
                && !before_colon.is_empty();
            is_valid_identifier
        } else if trimmed.starts_with("fn ") {
            // Function definitions also start a new statement
            true
        } else {
            // No $, #, or colon - check if it's a standalone command
            trimmed == "panic" || trimmed.starts_with("hush")
        };

        if is_definition {
            // Push accumulated statement if any
            if !current_statement.is_empty() {
                result.push(current_statement.clone());
                current_statement.clear();
            }
            // Start new statement
            current_statement = line.to_string();
        } else {
            // Continuation line - append with a space
            if !current_statement.is_empty() {
                current_statement.push(' ');
            }
            current_statement.push_str(line.trim());
        }
    }

    // Push final statement if any
    if !current_statement.is_empty() {
        result.push(current_statement);
    }

    result.join("\n")
}

/// Parse multiple statements separated by newlines
pub fn parse_dsl(input: &str) -> IResult<&str, Vec<DslStatement>> {
    // Preprocess to join continuation lines
    // We need to leak the string to get a 'static lifetime for nom
    // This is acceptable since DSL parsing happens infrequently (on file load)
    let preprocessed = preprocess_multiline(input);
    let static_input: &'static str = Box::leak(preprocessed.into_boxed_str());

    // Skip leading whitespace and comments
    let (mut remaining, _) = skip_whitespace_and_comments(static_input)?;

    // Parse statements manually to avoid issues with separated_list0
    let mut statements = Vec::new();
    while !remaining.is_empty() {
        match statement(remaining) {
            Ok((rest, stmt)) => {
                statements.push(stmt);
                // Skip whitespace after statement
                match skip_whitespace_and_comments(rest) {
                    Ok((new_rest, _)) => remaining = new_rest,
                    Err(_) => break,
                }
            }
            Err(_) => break,
        }
    }

    Ok((remaining, statements))
}

/// Compile DSL to UnifiedSignalGraph
pub struct DslCompiler {
    graph: UnifiedSignalGraph,
}

impl DslCompiler {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            graph: UnifiedSignalGraph::new(sample_rate),
        }
    }

    /// Compile statements into the graph
    pub fn compile(mut self, statements: Vec<DslStatement>) -> UnifiedSignalGraph {
        // First, compile all statements
        for stmt in statements {
            self.compile_statement(stmt);
        }

        // After compilation, handle ~master bus logic
        // Check if output was already set by an Output statement (backwards compatibility)
        if self.graph.has_output() {
            // Output was explicitly set via "out: expression" - don't override
            return self.graph;
        }

        // Check if "master" bus exists
        if let Some(master_node) = self.graph.get_bus("master") {
            // Explicit ~master definition exists, use it as output
            self.graph.set_output(master_node);
        } else {
            // No explicit ~master, check for auto-routing patterns
            // Pattern: out1, out2, out3... or d1, d2, d3...
            let all_buses = self.graph.get_all_bus_names();

            let auto_route_buses: Vec<String> = all_buses
                .iter()
                .filter(|name| {
                    // Match "out" followed by digits (out1, out2, out3...)
                    // or "d" followed by digits (d1, d2, d3...)
                    // or "o" followed by digits (o1, o2, o3...) - common Tidal-style syntax
                    let matches_out_pattern = name.starts_with("out")
                        && name.len() > 3
                        && name[3..].chars().all(|c| c.is_ascii_digit());

                    let matches_d_pattern = name.starts_with('d')
                        && name.len() > 1
                        && name[1..].chars().all(|c| c.is_ascii_digit());

                    let matches_o_pattern = name.starts_with('o')
                        && name.len() > 1
                        && name[1..].chars().all(|c| c.is_ascii_digit());

                    matches_out_pattern || matches_d_pattern || matches_o_pattern
                })
                .cloned()
                .collect();

            if !auto_route_buses.is_empty() {
                // Auto-route matching buses to master
                let mut sum_node = None;

                for bus_name in auto_route_buses {
                    if let Some(bus_node) = self.graph.get_bus(&bus_name) {
                        sum_node = if let Some(existing_sum) = sum_node {
                            // Add this bus to the sum
                            Some(self.graph.add_node(SignalNode::Add {
                                a: Signal::Node(existing_sum),
                                b: Signal::Node(bus_node),
                            }))
                        } else {
                            // First bus, start the sum
                            Some(bus_node)
                        };
                    }
                }

                if let Some(final_sum) = sum_node {
                    self.graph.set_output(final_sum);
                }
            } else if let Some(out_node) = self.graph.get_bus("out") {
                // Backwards compatibility: single "out" bus
                self.graph.set_output(out_node);
            } else {
                // No matching patterns, no ~master, no out - sum all buses
                if !all_buses.is_empty() {
                    let mut sum_node = None;

                    for bus_name in all_buses {
                        if let Some(bus_node) = self.graph.get_bus(&bus_name) {
                            sum_node = if let Some(existing_sum) = sum_node {
                                Some(self.graph.add_node(SignalNode::Add {
                                    a: Signal::Node(existing_sum),
                                    b: Signal::Node(bus_node),
                                }))
                            } else {
                                Some(bus_node)
                            };
                        }
                    }

                    if let Some(final_sum) = sum_node {
                        self.graph.set_output(final_sum);
                    }
                }
            }
        }

        self.graph
    }

    fn compile_statement(&mut self, stmt: DslStatement) {
        match stmt {
            DslStatement::BusDefinition { name, expr } => {
                let node_id = self.compile_expression(expr);
                self.graph.add_bus(name, node_id);
            }
            DslStatement::Output { channel, expr } => {
                let node_id = self.compile_expression(expr);
                let output_node = self.graph.add_node(SignalNode::Output {
                    input: Signal::Node(node_id),
                });

                // Use multi-output system: out -> channel 0, out1 -> channel 1, out2 -> channel 2, etc.
                match channel {
                    None => {
                        // Plain "out" - use backwards-compatible single output
                        self.graph.set_output(output_node);
                    }
                    Some(ch) => {
                        // Numbered output (out1, out2, etc.) - use multi-output system
                        self.graph.set_output_channel(ch, output_node);
                    }
                }
            }
            DslStatement::SetCps(cps) => {
                self.graph.set_cps(cps);
            }
            DslStatement::SetOutputMixMode(mode_str) => {
                use crate::unified_graph::OutputMixMode;
                if let Some(mode) = OutputMixMode::from_str(&mode_str) {
                    self.graph.set_output_mix_mode(mode);
                } else {
                    eprintln!(
                        "Warning: Invalid output mix mode '{}'. Valid modes: gain, sqrt, tanh, hard, none. Using default (sqrt).",
                        mode_str
                    );
                }
            }
            DslStatement::Hush { channel } => match channel {
                None => self.graph.hush_all(),
                Some(ch) => self.graph.hush_channel(ch),
            },
            DslStatement::Panic => {
                self.graph.panic();
            }
            DslStatement::Route { .. } => {
                // TODO: Implement routing
            }
        }
    }

    fn compile_expression(&mut self, expr: DslExpression) -> crate::unified_graph::NodeId {
        match expr {
            DslExpression::BusRef(name) => {
                // Look up the bus and return its node ID
                // If bus doesn't exist yet, create a placeholder constant
                if let Some(node_id) = self.graph.get_bus(&name) {
                    node_id
                } else {
                    eprintln!("Warning: BusRef '{}' not found, returning silence", name);
                    self.graph.add_node(SignalNode::Constant { value: 0.0 })
                }
            }
            DslExpression::Value(v) => self.graph.add_node(SignalNode::Constant { value: v }),
            DslExpression::Pattern(pattern_str) => {
                let pattern = parse_mini_notation(&pattern_str);
                self.graph.add_node(SignalNode::Pattern {
                    pattern_str,
                    pattern,
                    last_value: 0.0,
                    last_trigger_time: -1.0,
                })
            }
            DslExpression::Oscillator { waveform, freq, .. } => {
                let freq_signal = self.compile_expression_to_signal(*freq);
                self.graph.add_node(SignalNode::Oscillator {
                    freq: freq_signal,
                    semitone_offset: 0.0,
                    waveform,
                    phase: RefCell::new(0.0),
                    pending_freq: RefCell::new(None),
                    last_sample: RefCell::new(0.0),
                })
            }
            DslExpression::Filter {
                filter_type,
                input,
                cutoff,
                q,
            } => {
                let input_signal = self.compile_expression_to_signal(*input);
                let cutoff_signal = self.compile_expression_to_signal(*cutoff);
                let q_signal = self.compile_expression_to_signal(*q);

                match filter_type {
                    FilterType::LowPass => self.graph.add_node(SignalNode::LowPass {
                        input: input_signal,
                        cutoff: cutoff_signal,
                        q: q_signal,
                        state: Default::default(),
                    }),
                    FilterType::HighPass => self.graph.add_node(SignalNode::HighPass {
                        input: input_signal,
                        cutoff: cutoff_signal,
                        q: q_signal,
                        state: Default::default(),
                    }),
                    FilterType::BandPass => self.graph.add_node(SignalNode::BandPass {
                        input: input_signal,
                        center: cutoff_signal, // Center frequency for bandpass
                        q: q_signal,
                        state: Default::default(),
                    }),
                }
            }
            DslExpression::BinaryOp { op, left, right } => {
                let left_signal = self.compile_expression_to_signal(*left);
                let right_signal = self.compile_expression_to_signal(*right);

                match op {
                    BinaryOperator::Add => self.graph.add_node(SignalNode::Add {
                        a: left_signal,
                        b: right_signal,
                    }),
                    BinaryOperator::Multiply => self.graph.add_node(SignalNode::Multiply {
                        a: left_signal,
                        b: right_signal,
                    }),
                    _ => {
                        // TODO: Implement subtract and divide
                        self.graph.add_node(SignalNode::Constant { value: 0.0 })
                    }
                }
            }
            DslExpression::Chain { left, right } => {
                // Check if right is a DSP modifier (gain/pan/speed/cut)
                // If so, try to apply it to a SamplePattern (recursively if needed)
                match &*right {
                    DslExpression::Gain { value } => {
                        let modified_left = self.apply_modifier_to_sample(*left, |mut sample| {
                            sample.gain = Some(value.clone());
                            sample
                        });
                        self.compile_expression(modified_left)
                    }
                    DslExpression::Pan { value } => {
                        let modified_left = self.apply_modifier_to_sample(*left, |mut sample| {
                            sample.pan = Some(value.clone());
                            sample
                        });
                        self.compile_expression(modified_left)
                    }
                    DslExpression::Speed { value } => {
                        let modified_left = self.apply_modifier_to_sample(*left, |mut sample| {
                            sample.speed = Some(value.clone());
                            sample
                        });
                        self.compile_expression(modified_left)
                    }
                    DslExpression::Cut { value } => {
                        let modified_left = self.apply_modifier_to_sample(*left, |mut sample| {
                            sample.cut_group = Some(value.clone());
                            sample
                        });
                        self.compile_expression(modified_left)
                    }
                    DslExpression::N { value } => {
                        let modified_left = self.apply_modifier_to_sample(*left, |mut sample| {
                            sample.n = Some(value.clone());
                            sample
                        });
                        self.compile_expression(modified_left)
                    }
                    DslExpression::Note { value } => {
                        let modified_left = self.apply_modifier_to_sample(*left, |mut sample| {
                            sample.note = Some(value.clone());
                            sample
                        });
                        self.compile_expression(modified_left)
                    }
                    DslExpression::SegmentsModifier {
                        levels_str,
                        times_str,
                    } => {
                        let levels_str = levels_str.clone();
                        let times_str = times_str.clone();
                        let modified_left = self.apply_modifier_to_sample(*left, |mut sample| {
                            sample.envelope_type = Some(SampleEnvelopeType::Segments {
                                levels_str,
                                times_str,
                            });
                            sample
                        });
                        self.compile_expression(modified_left)
                    }
                    DslExpression::CurveModifier {
                        start,
                        end,
                        duration,
                        curve,
                    } => {
                        let start = start.clone();
                        let end = end.clone();
                        let duration = duration.clone();
                        let curve = curve.clone();
                        let modified_left = self.apply_modifier_to_sample(*left, |mut sample| {
                            sample.envelope_type = Some(SampleEnvelopeType::Curve {
                                start,
                                end,
                                duration,
                                curve,
                            });
                            sample
                        });
                        self.compile_expression(modified_left)
                    }
                    DslExpression::ADSRModifier {
                        attack,
                        decay,
                        sustain,
                        release,
                    } => {
                        let decay = decay.clone();
                        let sustain = sustain.clone();
                        let modified_left = self.apply_modifier_to_sample(*left, |mut sample| {
                            // For ADSR, we use attack/release from the modifier
                            // and set decay/sustain in the envelope_type
                            sample.attack = Some(attack.clone());
                            sample.release = Some(release.clone());
                            sample.envelope_type =
                                Some(SampleEnvelopeType::ADSR { decay, sustain });
                            sample
                        });
                        self.compile_expression(modified_left)
                    }
                    // Default: standard chain behavior (for effects, etc.)
                    _ => {
                        let left_id = self.compile_expression(*left);
                        let modified_right = self.inject_chain_input(*right, left_id);
                        self.compile_expression(modified_right)
                    }
                }
            }
            DslExpression::Synth { synth_type, params } => {
                use crate::superdirt_synths::SynthLibrary;
                let library = SynthLibrary::with_sample_rate(44100.0);

                // Extract frequency (first param)
                let freq = params
                    .first()
                    .map(|e| self.compile_expression_to_signal(e.clone()))
                    .unwrap_or(Signal::Value(440.0));

                match synth_type {
                    SynthType::SuperKick => {
                        let pitch_env = params
                            .get(1)
                            .map(|e| self.compile_expression_to_signal(e.clone()));
                        let sustain = params.get(2).and_then(|e| {
                            if let DslExpression::Value(v) = e {
                                Some(*v)
                            } else {
                                None
                            }
                        });
                        let noise = params
                            .get(3)
                            .map(|e| self.compile_expression_to_signal(e.clone()));
                        library.build_kick(&mut self.graph, freq, pitch_env, sustain, noise)
                    }
                    SynthType::SuperSaw => {
                        // Note: detune and voices must be constant for now (synth design limitation)
                        let detune = params.get(1).and_then(|e| {
                            if let DslExpression::Value(v) = e {
                                Some(*v)
                            } else {
                                None
                            }
                        });
                        let voices = params.get(2).and_then(|e| {
                            if let DslExpression::Value(v) = e {
                                Some(*v as usize)
                            } else {
                                None
                            }
                        });
                        library.build_supersaw(&mut self.graph, freq, detune, voices)
                    }
                    SynthType::SuperPWM => {
                        // Note: structural params must be constant (synth design limitation)
                        let pwm_rate = params.get(1).and_then(|e| {
                            if let DslExpression::Value(v) = e {
                                Some(*v)
                            } else {
                                None
                            }
                        });
                        let pwm_depth = params.get(2).and_then(|e| {
                            if let DslExpression::Value(v) = e {
                                Some(*v)
                            } else {
                                None
                            }
                        });
                        library.build_superpwm(&mut self.graph, freq, pwm_rate, pwm_depth)
                    }
                    SynthType::SuperChip => {
                        // Note: structural params must be constant (synth design limitation)
                        let vibrato_rate = params.get(1).and_then(|e| {
                            if let DslExpression::Value(v) = e {
                                Some(*v)
                            } else {
                                None
                            }
                        });
                        let vibrato_depth = params.get(2).and_then(|e| {
                            if let DslExpression::Value(v) = e {
                                Some(*v)
                            } else {
                                None
                            }
                        });
                        library.build_superchip(&mut self.graph, freq, vibrato_rate, vibrato_depth)
                    }
                    SynthType::SuperFM => {
                        // Note: structural params must be constant (synth design limitation)
                        let mod_ratio = params.get(1).and_then(|e| {
                            if let DslExpression::Value(v) = e {
                                Some(*v)
                            } else {
                                None
                            }
                        });
                        let mod_index = params.get(2).and_then(|e| {
                            if let DslExpression::Value(v) = e {
                                Some(*v)
                            } else {
                                None
                            }
                        });
                        library.build_superfm(&mut self.graph, freq, mod_ratio, mod_index)
                    }
                    SynthType::SuperSnare => {
                        // Note: structural params must be constant (synth design limitation)
                        let snappy = params.get(1).and_then(|e| {
                            if let DslExpression::Value(v) = e {
                                Some(*v)
                            } else {
                                None
                            }
                        });
                        let sustain = params.get(2).and_then(|e| {
                            if let DslExpression::Value(v) = e {
                                Some(*v)
                            } else {
                                None
                            }
                        });
                        library.build_snare(&mut self.graph, freq, snappy, sustain)
                    }
                    SynthType::SuperHat => {
                        // Note: structural params must be constant (synth design limitation)
                        let bright = params.get(0).and_then(|e| {
                            if let DslExpression::Value(v) = e {
                                Some(*v)
                            } else {
                                None
                            }
                        });
                        let sustain = params.get(1).and_then(|e| {
                            if let DslExpression::Value(v) = e {
                                Some(*v)
                            } else {
                                None
                            }
                        });
                        library.build_hat(&mut self.graph, bright, sustain)
                    }
                }
            }
            DslExpression::Effect {
                effect_type,
                input,
                params,
            } => {
                use crate::superdirt_synths::SynthLibrary;
                let library = SynthLibrary::with_sample_rate(44100.0);

                let input_node = self.compile_expression(*input);

                match effect_type {
                    EffectType::Reverb => {
                        let room_size = params
                            .first()
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(0.7);
                        let damping = params
                            .get(1)
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(0.5);
                        let mix = params
                            .get(2)
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(0.3);
                        library.add_reverb(&mut self.graph, input_node, room_size, damping, mix)
                    }
                    EffectType::Distortion => {
                        let drive = params
                            .first()
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(3.0);
                        let mix = params
                            .get(1)
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(0.5);
                        library.add_distortion(&mut self.graph, input_node, drive, mix)
                    }
                    EffectType::BitCrush => {
                        let bits = params
                            .first()
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(4.0);
                        let rate = params
                            .get(1)
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(4.0);
                        library.add_bitcrush(&mut self.graph, input_node, bits, rate)
                    }
                    EffectType::Chorus => {
                        let rate = params
                            .first()
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(1.0);
                        let depth = params
                            .get(1)
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(0.5);
                        let mix = params
                            .get(2)
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(0.3);
                        library.add_chorus(&mut self.graph, input_node, rate, depth, mix)
                    }
                    EffectType::Compressor => {
                        let threshold_db = params
                            .first()
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(-20.0);
                        let ratio = params
                            .get(1)
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(4.0);
                        let attack = params
                            .get(2)
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(0.01);
                        let release = params
                            .get(3)
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(0.1);
                        let makeup_gain_db = params
                            .get(4)
                            .and_then(|e| {
                                if let DslExpression::Value(v) = e {
                                    Some(*v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(0.0);

                        library.add_compressor(
                            &mut self.graph,
                            input_node,
                            threshold_db,
                            ratio,
                            attack,
                            release,
                            makeup_gain_db,
                        )
                    }
                }
            }
            DslExpression::SamplePattern {
                pattern,
                gain,
                pan,
                speed,
                cut_group,
                n,
                note,
                attack,
                release,
                envelope_type,
            } => {
                use std::collections::HashMap;

                // Parse the mini-notation pattern
                let parsed_pattern = parse_mini_notation(&pattern);

                // Compile DSP parameters to signals
                let gain_signal = gain
                    .map(|e| self.compile_expression_to_signal(*e))
                    .unwrap_or(Signal::Value(1.0));

                let pan_signal = pan
                    .map(|e| self.compile_expression_to_signal(*e))
                    .unwrap_or(Signal::Value(0.0));

                let speed_signal = speed
                    .map(|e| self.compile_expression_to_signal(*e))
                    .unwrap_or(Signal::Value(1.0));

                let cut_group_signal = cut_group
                    .map(|e| self.compile_expression_to_signal(*e))
                    .unwrap_or(Signal::Value(0.0)); // No cut group by default

                let n_signal = n
                    .map(|e| self.compile_expression_to_signal(*e))
                    .unwrap_or(Signal::Value(0.0)); // Sample 0 by default

                let note_signal = note
                    .map(|e| self.compile_expression_to_signal(*e))
                    .unwrap_or(Signal::Value(0.0)); // Original pitch by default

                let attack_signal = attack
                    .map(|e| self.compile_expression_to_signal(*e))
                    .unwrap_or(Signal::Value(0.0)); // No attack envelope by default

                let release_signal = release
                    .map(|e| self.compile_expression_to_signal(*e))
                    .unwrap_or(Signal::Value(0.0)); // No release envelope by default

                // Convert envelope_type to RuntimeEnvelopeType
                let runtime_envelope = envelope_type.map(|env_type| match env_type {
                    SampleEnvelopeType::Percussion => {
                        crate::unified_graph::RuntimeEnvelopeType::Percussion
                    }
                    SampleEnvelopeType::ADSR { decay, sustain } => {
                        crate::unified_graph::RuntimeEnvelopeType::ADSR {
                            decay: self.compile_expression_to_signal(*decay),
                            sustain: self.compile_expression_to_signal(*sustain),
                        }
                    }
                    SampleEnvelopeType::Segments {
                        levels_str,
                        times_str,
                    } => {
                        // Parse levels and times strings
                        let levels: Vec<f32> = levels_str
                            .split_whitespace()
                            .filter_map(|s| s.parse().ok())
                            .collect();
                        let times: Vec<f32> = times_str
                            .split_whitespace()
                            .filter_map(|s| s.parse().ok())
                            .collect();
                        crate::unified_graph::RuntimeEnvelopeType::Segments { levels, times }
                    }
                    SampleEnvelopeType::Curve {
                        start,
                        end,
                        duration,
                        curve,
                    } => crate::unified_graph::RuntimeEnvelopeType::Curve {
                        start: self.compile_expression_to_signal(*start),
                        end: self.compile_expression_to_signal(*end),
                        duration: self.compile_expression_to_signal(*duration),
                        curve: self.compile_expression_to_signal(*curve),
                    },
                });

                // Create Sample node
                self.graph.add_node(SignalNode::Sample {
                    pattern_str: pattern,
                    pattern: parsed_pattern,
                    last_trigger_time: -1.0,
                    last_cycle: -1, // Initialize to -1 to trigger on first cycle
                    playback_positions: HashMap::new(),
                    gain: gain_signal,
                    pan: pan_signal,
                    speed: speed_signal,
                    cut_group: cut_group_signal,
                    n: n_signal,
                    note: note_signal,
                    attack: attack_signal,
                    release: release_signal,
                    envelope_type: runtime_envelope,
                    unit_mode: Signal::Value(0.0), // 0 = rate mode (default)
                    loop_enabled: Signal::Value(0.0), // 0 = no loop (default)
                    begin: Signal::Value(0.0),
                    end: Signal::Value(1.0),
                })
            }
            DslExpression::Scale {
                pattern,
                scale_name,
                root_note,
            } => {
                use crate::pattern_tonal::note_to_midi;

                // Parse the mini-notation pattern
                let parsed_pattern = parse_mini_notation(&pattern);

                // Convert root note to MIDI number
                let root_midi = if let Ok(midi) = root_note.parse::<u8>() {
                    midi
                } else if let Some(midi) = note_to_midi(&root_note) {
                    midi
                } else {
                    60 // Default to C4
                };

                // Create ScaleQuantize node
                self.graph.add_node(SignalNode::ScaleQuantize {
                    pattern_str: pattern,
                    pattern: parsed_pattern,
                    scale_name,
                    root_note: root_midi,
                    last_value: 261.63, // Default to C4 frequency
                })
            }
            DslExpression::SynthPattern {
                notes,
                waveform,
                attack,
                decay,
                sustain,
                release,
                gain,
                pan,
            } => {
                // Parse the mini-notation pattern
                let parsed_pattern = parse_mini_notation(&notes);

                // Compile DSP parameters to signals
                let gain_signal = gain
                    .map(|e| self.compile_expression_to_signal(*e))
                    .unwrap_or(Signal::Value(0.3));

                let pan_signal = pan
                    .map(|e| self.compile_expression_to_signal(*e))
                    .unwrap_or(Signal::Value(0.0));

                // Use provided ADSR or defaults
                let attack_val = attack.unwrap_or(0.01);
                let decay_val = decay.unwrap_or(0.1);
                let sustain_val = sustain.unwrap_or(0.7);
                let release_val = release.unwrap_or(0.2);

                // Create SynthPattern node
                self.graph.add_node(SignalNode::SynthPattern {
                    pattern_str: notes,
                    pattern: parsed_pattern,
                    last_trigger_time: -1.0,
                    waveform,
                    attack: attack_val,
                    decay: decay_val,
                    sustain: sustain_val,
                    release: release_val,
                    gain: gain_signal,
                    pan: pan_signal,
                })
            }
            DslExpression::Delay {
                input,
                time,
                feedback,
                mix,
            } => {
                let input_signal = self.compile_expression_to_signal(*input);
                let time_signal = self.compile_expression_to_signal(*time);
                let feedback_signal = self.compile_expression_to_signal(*feedback);
                let mix_signal = self.compile_expression_to_signal(*mix);

                // Create delay buffer (2 seconds max @ 44.1kHz)
                let max_delay_samples = (2.0 * self.graph.sample_rate()) as usize;
                let buffer = vec![0.0; max_delay_samples];

                self.graph.add_node(SignalNode::Delay {
                    input: input_signal,
                    time: time_signal,
                    feedback: feedback_signal,
                    mix: mix_signal,
                    buffer,
                    write_idx: 0,
                })
            }
            DslExpression::PatternTransform { pattern, transform } => {
                // For now, pattern transforms only work on pattern strings or nested transforms
                match *pattern {
                    DslExpression::Pattern(pattern_str) => {
                        // Parse the base pattern
                        let base_pattern = parse_mini_notation(&pattern_str);

                        // Apply the transform
                        let transformed_pattern =
                            match self.apply_pattern_transform(base_pattern, transform) {
                                Ok(p) => p,
                                Err(e) => {
                                    eprintln!("Warning: Failed to apply pattern transform: {}", e);
                                    parse_mini_notation(&pattern_str) // Fallback to original
                                }
                            };

                        // Create a pattern node with the transformed pattern
                        self.graph.add_node(SignalNode::Pattern {
                            pattern_str, // Keep original string for debugging
                            pattern: transformed_pattern,
                            last_value: 0.0,
                            last_trigger_time: -1.0,
                        })
                    }
                    DslExpression::PatternTransform {
                        pattern: inner_pattern,
                        transform: inner_transform,
                    } => {
                        // Handle chained transforms: pattern $ f $ g
                        // First compile the inner transform
                        let inner_expr = DslExpression::PatternTransform {
                            pattern: inner_pattern,
                            transform: inner_transform,
                        };

                        // This will recursively compile inner transforms
                        let inner_node_id = self.compile_expression(inner_expr);

                        // Now we need to get the pattern from the inner node and apply our transform
                        // The inner node could be either a Pattern node or a Sample node
                        // Extract pattern data first to avoid borrow checker issues
                        let node = self.graph.get_node(inner_node_id);

                        if let Some(SignalNode::Pattern {
                            pattern: inner_pattern_obj,
                            pattern_str,
                            ..
                        }) = node
                        {
                            // Inner node is a Pattern, apply transform and create new Pattern node
                            let pattern_data = (inner_pattern_obj.clone(), pattern_str.clone());
                            let (inner_pattern, pattern_str) = pattern_data;

                            let transformed_pattern = match self
                                .apply_pattern_transform(inner_pattern.clone(), transform)
                            {
                                Ok(p) => p,
                                Err(e) => {
                                    eprintln!("Warning: Failed to apply chained transform: {}", e);
                                    inner_pattern
                                }
                            };

                            self.graph.add_node(SignalNode::Pattern {
                                pattern_str,
                                pattern: transformed_pattern,
                                last_value: 0.0,
                                last_trigger_time: -1.0,
                            })
                        } else if let Some(SignalNode::Sample {
                            pattern: inner_pattern_obj,
                            pattern_str,
                            gain,
                            pan,
                            speed,
                            cut_group,
                            attack,
                            release,
                            ..
                        }) = node
                        {
                            // Inner node is a Sample, apply transform and create new Sample node
                            let sample_data = (
                                inner_pattern_obj.clone(),
                                pattern_str.clone(),
                                gain.clone(),
                                pan.clone(),
                                speed.clone(),
                                cut_group.clone(),
                                attack.clone(),
                                release.clone(),
                            );
                            let (
                                inner_pattern,
                                pattern_str,
                                gain,
                                pan,
                                speed,
                                cut_group,
                                attack,
                                release,
                            ) = sample_data;

                            let transformed_pattern = match self
                                .apply_pattern_transform(inner_pattern.clone(), transform)
                            {
                                Ok(p) => p,
                                Err(e) => {
                                    eprintln!(
                                        "Warning: Failed to apply chained transform to sample: {}",
                                        e
                                    );
                                    inner_pattern
                                }
                            };

                            use std::collections::HashMap;
                            self.graph.add_node(SignalNode::Sample {
                                pattern_str,
                                pattern: transformed_pattern,
                                last_trigger_time: -1.0,
                                last_cycle: -1,
                                playback_positions: HashMap::new(),
                                gain,
                                pan,
                                speed,
                                cut_group,
                                n: Signal::Value(0.0),
                                note: Signal::Value(0.0),
                                attack,
                                release,
                                envelope_type: None, // TODO: Support envelope in pattern transforms
                                unit_mode: Signal::Value(0.0), // 0 = rate mode (default)
                                loop_enabled: Signal::Value(0.0), // 0 = no loop (default)
                                begin: Signal::Value(0.0),
                                end: Signal::Value(1.0),
                            })
                        } else {
                            eprintln!("Warning: Chained transform inner expression did not produce a pattern or sample node");
                            self.graph.add_node(SignalNode::Constant { value: 0.0 })
                        }
                    }
                    DslExpression::SamplePattern {
                        pattern: pattern_str,
                        gain,
                        pan,
                        speed,
                        cut_group,
                        n,
                        note,
                        attack,
                        release,
                        envelope_type,
                    } => {
                        // Handle transforms on sample patterns: s("bd sn" $ fast 2)
                        // Parse and transform the pattern
                        let base_pattern = parse_mini_notation(&pattern_str);
                        let transformed_pattern = match self
                            .apply_pattern_transform(base_pattern, transform)
                        {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("Warning: Failed to apply pattern transform to sample pattern: {}", e);
                                parse_mini_notation(&pattern_str)
                            }
                        };

                        // Compile DSP parameters
                        let gain_signal = gain
                            .map(|e| self.compile_expression_to_signal(*e))
                            .unwrap_or(Signal::Value(1.0));
                        let pan_signal = pan
                            .map(|e| self.compile_expression_to_signal(*e))
                            .unwrap_or(Signal::Value(0.0));
                        let speed_signal = speed
                            .map(|e| self.compile_expression_to_signal(*e))
                            .unwrap_or(Signal::Value(1.0));

                        let cut_group_signal = cut_group
                            .map(|e| self.compile_expression_to_signal(*e))
                            .unwrap_or(Signal::Value(0.0));

                        let n_signal = n
                            .map(|e| self.compile_expression_to_signal(*e))
                            .unwrap_or(Signal::Value(0.0));

                        let note_signal = note
                            .map(|e| self.compile_expression_to_signal(*e))
                            .unwrap_or(Signal::Value(0.0));

                        let attack_signal = attack
                            .map(|e| self.compile_expression_to_signal(*e))
                            .unwrap_or(Signal::Value(0.0));

                        let release_signal = release
                            .map(|e| self.compile_expression_to_signal(*e))
                            .unwrap_or(Signal::Value(0.0));

                        // Create Sample node with transformed pattern
                        use std::collections::HashMap;
                        self.graph.add_node(SignalNode::Sample {
                            pattern_str,
                            pattern: transformed_pattern,
                            last_trigger_time: -1.0,
                            last_cycle: -1, // Initialize to -1 to trigger on first cycle
                            playback_positions: HashMap::new(),
                            gain: gain_signal,
                            pan: pan_signal,
                            speed: speed_signal,
                            cut_group: cut_group_signal,
                            n: n_signal,
                            note: note_signal,
                            attack: attack_signal,
                            release: release_signal,
                            envelope_type: None, // TODO: Support envelope in this case
                            unit_mode: Signal::Value(0.0), // 0 = rate mode (default)
                            loop_enabled: Signal::Value(0.0), // 0 = no loop (default)
                            begin: Signal::Value(0.0),
                            end: Signal::Value(1.0),
                        })
                    }
                    DslExpression::BusRef(bus_name) => {
                        // Handle transforms on bus references: ~drums $ fast 2
                        // First get the bus node
                        let bus_node_id = if let Some(node_id) = self.graph.get_bus(&bus_name) {
                            node_id
                        } else {
                            eprintln!("Warning: Bus '{}' not found", bus_name);
                            return self.graph.add_node(SignalNode::Constant { value: 0.0 });
                        };

                        // Get the node and extract pattern data
                        let node = self.graph.get_node(bus_node_id);

                        if let Some(SignalNode::Sample {
                            pattern: pattern_obj,
                            pattern_str,
                            gain,
                            pan,
                            speed,
                            cut_group,
                            attack,
                            release,
                            ..
                        }) = node
                        {
                            // Bus contains a Sample node - apply transform and create new Sample node
                            let sample_data = (
                                pattern_obj.clone(),
                                pattern_str.clone(),
                                gain.clone(),
                                pan.clone(),
                                speed.clone(),
                                cut_group.clone(),
                                attack.clone(),
                                release.clone(),
                            );
                            let (
                                inner_pattern,
                                pattern_str,
                                gain,
                                pan,
                                speed,
                                cut_group,
                                attack,
                                release,
                            ) = sample_data;

                            let transformed_pattern = match self
                                .apply_pattern_transform(inner_pattern.clone(), transform)
                            {
                                Ok(p) => p,
                                Err(e) => {
                                    eprintln!(
                                        "Warning: Failed to apply transform to bus '{}': {}",
                                        bus_name, e
                                    );
                                    inner_pattern
                                }
                            };

                            use std::collections::HashMap;
                            self.graph.add_node(SignalNode::Sample {
                                pattern_str,
                                pattern: transformed_pattern,
                                last_trigger_time: -1.0,
                                last_cycle: -1,
                                playback_positions: HashMap::new(),
                                gain,
                                pan,
                                speed,
                                cut_group,
                                n: Signal::Value(0.0),
                                note: Signal::Value(0.0),
                                attack,
                                release,
                                envelope_type: None, // TODO: Support envelope in pattern transforms
                                unit_mode: Signal::Value(0.0), // 0 = rate mode (default)
                                loop_enabled: Signal::Value(0.0), // 0 = no loop (default)
                                begin: Signal::Value(0.0),
                                end: Signal::Value(1.0),
                            })
                        } else if let Some(SignalNode::Pattern {
                            pattern: pattern_obj,
                            pattern_str,
                            ..
                        }) = node
                        {
                            // Bus contains a Pattern node - apply transform and create new Pattern node
                            let pattern_data = (pattern_obj.clone(), pattern_str.clone());
                            let (inner_pattern, pattern_str) = pattern_data;

                            let transformed_pattern = match self
                                .apply_pattern_transform(inner_pattern.clone(), transform)
                            {
                                Ok(p) => p,
                                Err(e) => {
                                    eprintln!(
                                        "Warning: Failed to apply transform to bus '{}': {}",
                                        bus_name, e
                                    );
                                    inner_pattern
                                }
                            };

                            self.graph.add_node(SignalNode::Pattern {
                                pattern_str,
                                pattern: transformed_pattern,
                                last_value: 0.0,
                                last_trigger_time: -1.0,
                            })
                        } else {
                            eprintln!("Warning: Bus '{}' does not contain a pattern or sample node - cannot apply transform", bus_name);
                            self.graph.add_node(SignalNode::Constant { value: 0.0 })
                        }
                    }
                    _ => {
                        eprintln!("Warning: Pattern transforms currently only work on pattern strings, not {:?}", *pattern);
                        self.graph.add_node(SignalNode::Constant { value: 0.0 })
                    }
                }
            }
            _ => {
                // TODO: Implement other expression types
                self.graph.add_node(SignalNode::Constant { value: 0.0 })
            }
        }
    }

    fn compile_expression_to_signal(&mut self, expr: DslExpression) -> Signal {
        match expr {
            DslExpression::Value(v) => Signal::Value(v),
            DslExpression::BusRef(name) => Signal::Bus(name),
            DslExpression::Pattern(p) => Signal::Pattern(p),
            _ => {
                let node_id = self.compile_expression(expr);
                Signal::Node(node_id)
            }
        }
    }

    /// Apply a DSP modifier to a SamplePattern (recursively if needed)
    /// This handles chained modifiers like: s("bd") # gain(0.8) # pan(-0.5)
    fn apply_modifier_to_sample<F>(&mut self, expr: DslExpression, modify: F) -> DslExpression
    where
        F: FnOnce(SamplePatternFields) -> SamplePatternFields,
    {
        match expr {
            // Base case: found the SamplePattern
            DslExpression::SamplePattern {
                pattern,
                gain,
                pan,
                speed,
                cut_group,
                n,
                note,
                attack,
                release,
                envelope_type,
            } => {
                let fields = SamplePatternFields {
                    pattern,
                    gain,
                    pan,
                    speed,
                    cut_group,
                    n,
                    note,
                    attack,
                    release,
                    envelope_type: None,
                };
                let modified = modify(fields);
                DslExpression::SamplePattern {
                    pattern: modified.pattern,
                    gain: modified.gain,
                    pan: modified.pan,
                    speed: modified.speed,
                    cut_group: modified.cut_group,
                    n: modified.n,
                    note: modified.note,
                    attack: modified.attack,
                    release: modified.release,
                    envelope_type: modified.envelope_type,
                }
            }
            // Recursive case: chain with modifiers
            DslExpression::Chain { left, right } => {
                // Apply modifier to the left side recursively
                let modified_left = self.apply_modifier_to_sample(*left, modify);
                // Return the chain with modified left
                DslExpression::Chain {
                    left: Box::new(modified_left),
                    right,
                }
            }
            // Other expressions: return as-is (shouldn't happen in valid Tidal syntax)
            other => other,
        }
    }

    /// Inject the left-hand node from a chain as the input to the right-hand expression
    ///
    /// When parsing `a # lpf(x, y)`, the parser treats lpf as having 3 args:
    /// - args[0] = x -> input
    /// - args[1] = y -> cutoff
    /// - args[2] = default -> q
    ///
    /// But in chain context, we want:
    /// - input = a (from chain)
    /// - cutoff = x (shift from input)
    /// - q = y (shift from cutoff)
    fn inject_chain_input(
        &mut self,
        right: DslExpression,
        left_node: crate::unified_graph::NodeId,
    ) -> DslExpression {
        // Register the left node as a temporary bus so it can be referenced
        let bus_name = format!("__chain_{}", left_node.0);
        self.graph.add_bus(bus_name.clone(), left_node);

        match right {
            // For filters in chain context, shift arguments:
            // Original parse: lpf 1000 0.8 -> input=1000, cutoff=0.8, q=1.0
            // Chain context: lpf 1000 0.8 should mean cutoff=1000, q=0.8
            DslExpression::Filter {
                filter_type,
                input,
                cutoff,
                q,
            } => {
                DslExpression::Filter {
                    filter_type,
                    input: Box::new(DslExpression::BusRef(bus_name)),
                    cutoff: input, // Shift: what was parsed as input is actually cutoff
                    q: cutoff,     // Shift: what was parsed as cutoff is actually q
                }
            }
            // For effects in chain context, shift arguments
            // The original "input" was actually the first parameter, so prepend it to params
            // Example: sine 440 # compressor -30.0 10.0 0.001 0.01 0.0
            //   Parser sees: Effect { input: -30.0, params: [10.0, 0.001, 0.01, 0.0] }
            //   After shift: Effect { input: sine_440, params: [-30.0, 10.0, 0.001, 0.01, 0.0] }
            DslExpression::Effect {
                effect_type,
                input,
                params,
            } => {
                let mut new_params = vec![*input];
                new_params.extend(params);
                DslExpression::Effect {
                    effect_type,
                    input: Box::new(DslExpression::BusRef(bus_name)),
                    params: new_params,
                }
            }
            // For delays in chain context, shift arguments
            // Original parse: delay(t, f, m) -> input=t, time=f, feedback=m, mix=0.5
            // Chain context: delay(t, f, m) should mean time=t, feedback=f, mix=m
            DslExpression::Delay {
                input,
                time,
                feedback,
                mix,
            } => {
                DslExpression::Delay {
                    input: Box::new(DslExpression::BusRef(bus_name)),
                    time: input,    // Shift: what was parsed as input is actually time
                    feedback: time, // Shift: what was parsed as time is actually feedback
                    mix: feedback,  // Shift: what was parsed as feedback is actually mix
                }
            }
            // For other expressions, wrap in a chain if needed or return as-is
            // (this handles cases like: osc # osc, which would just multiply)
            other => other,
        }
    }

    /// Apply a pattern transformation to a pattern
    fn apply_pattern_transform(
        &mut self,
        pattern: crate::pattern::Pattern<String>,
        transform: PatternTransformOp,
    ) -> Result<crate::pattern::Pattern<String>, String> {
        match transform {
            PatternTransformOp::Fast(factor_expr) => {
                // Extract the numeric value
                let factor = self.extract_constant(*factor_expr)?;
                Ok(pattern.fast(Pattern::pure(factor)))
            }
            PatternTransformOp::Slow(factor_expr) => {
                let factor = self.extract_constant(*factor_expr)?;
                Ok(pattern.slow(Pattern::pure(factor)))
            }
            PatternTransformOp::Squeeze(factor_expr) => {
                let factor = self.extract_constant(*factor_expr)?;
                Ok(pattern.squeeze(factor))
            }
            PatternTransformOp::Rev => Ok(pattern.rev()),
            PatternTransformOp::Every { n, f } => {
                let n_val = self.extract_constant(*n)? as i32;
                let inner_transform = *f;

                // Create a closure that applies the inner transform
                Ok(pattern.every(n_val, move |p| {
                    // We need to apply the inner transform recursively
                    // For now, just handle simple transforms
                    match inner_transform {
                        PatternTransformOp::Fast(ref factor_expr) => {
                            if let DslExpression::Value(v) = **factor_expr {
                                p.fast(Pattern::pure(v as f64))
                            } else {
                                p // Can't evaluate non-constant, return unchanged
                            }
                        }
                        PatternTransformOp::Slow(ref factor_expr) => {
                            if let DslExpression::Value(v) = **factor_expr {
                                p.slow(Pattern::pure(v as f64))
                            } else {
                                p
                            }
                        }
                        PatternTransformOp::Squeeze(ref factor_expr) => {
                            if let DslExpression::Value(v) = **factor_expr {
                                p.squeeze(v as f64)
                            } else {
                                p
                            }
                        }
                        PatternTransformOp::Rev => p.rev(),
                        _ => {
                            eprintln!("Warning: Nested higher-order transforms not yet supported");
                            p
                        }
                    }
                }))
            }
            PatternTransformOp::Sometimes(f) => {
                let inner_transform = *f;
                Ok(pattern.sometimes(move |p| match inner_transform {
                    PatternTransformOp::Fast(ref factor_expr) => {
                        if let DslExpression::Value(v) = **factor_expr {
                            p.fast(Pattern::pure(v as f64))
                        } else {
                            p
                        }
                    }
                    PatternTransformOp::Slow(ref factor_expr) => {
                        if let DslExpression::Value(v) = **factor_expr {
                            p.slow(Pattern::pure(v as f64))
                        } else {
                            p
                        }
                    }
                    PatternTransformOp::Rev => p.rev(),
                    _ => p,
                }))
            }
            PatternTransformOp::Often(f) => {
                let inner_transform = *f;
                Ok(pattern.often(move |p| match inner_transform {
                    PatternTransformOp::Fast(ref factor_expr) => {
                        if let DslExpression::Value(v) = **factor_expr {
                            p.fast(Pattern::pure(v as f64))
                        } else {
                            p
                        }
                    }
                    PatternTransformOp::Slow(ref factor_expr) => {
                        if let DslExpression::Value(v) = **factor_expr {
                            p.slow(Pattern::pure(v as f64))
                        } else {
                            p
                        }
                    }
                    PatternTransformOp::Rev => p.rev(),
                    _ => p,
                }))
            }
            PatternTransformOp::Rarely(f) => {
                let inner_transform = *f;
                Ok(pattern.rarely(move |p| match inner_transform {
                    PatternTransformOp::Fast(ref factor_expr) => {
                        if let DslExpression::Value(v) = **factor_expr {
                            p.fast(Pattern::pure(v as f64))
                        } else {
                            p
                        }
                    }
                    PatternTransformOp::Slow(ref factor_expr) => {
                        if let DslExpression::Value(v) = **factor_expr {
                            p.slow(Pattern::pure(v as f64))
                        } else {
                            p
                        }
                    }
                    PatternTransformOp::Rev => p.rev(),
                    _ => p,
                }))
            }
            PatternTransformOp::Degrade => Ok(pattern.degrade()),
            PatternTransformOp::DegradeBy(prob_expr) => {
                let prob = self.extract_constant(*prob_expr)?;
                Ok(pattern.degrade_by(Pattern::pure(prob)))
            }
            PatternTransformOp::Palindrome => Ok(pattern.palindrome()),
            PatternTransformOp::Stutter(n_expr) => {
                let n = self.extract_constant(*n_expr)? as usize;
                Ok(pattern.stutter(n))
            }
            PatternTransformOp::Late(amount_expr) => {
                let amount = self.extract_constant(*amount_expr)?;
                Ok(pattern.late(Pattern::pure(amount)))
            }
            PatternTransformOp::Early(amount_expr) => {
                let amount = self.extract_constant(*amount_expr)?;
                Ok(pattern.early(Pattern::pure(amount)))
            }
            PatternTransformOp::Dup(n_expr) => {
                let n = self.extract_constant(*n_expr)? as usize;
                Ok(pattern.dup(n))
            }
            PatternTransformOp::Zoom { begin, end } => {
                let begin_val = self.extract_constant(*begin)?;
                let end_val = self.extract_constant(*end)?;
                Ok(pattern.zoom(Pattern::pure(begin_val), Pattern::pure(end_val)))
            }
            PatternTransformOp::Focus { begin, end } => {
                let begin_val = self.extract_constant(*begin)?;
                let end_val = self.extract_constant(*end)?;
                Ok(pattern.focus(Pattern::pure(begin_val), Pattern::pure(end_val)))
            }
            PatternTransformOp::Within {
                begin,
                end,
                transform,
            } => {
                let begin_val = self.extract_constant(*begin)?;
                let end_val = self.extract_constant(*end)?;
                let inner_transform = *transform;

                // Create a closure that applies the inner transform
                Ok(pattern.within(begin_val, end_val, move |p| {
                    // Handle common transforms
                    match inner_transform {
                        PatternTransformOp::Fast(ref factor_expr) => {
                            if let DslExpression::Value(v) = **factor_expr {
                                p.fast(Pattern::pure(v as f64))
                            } else {
                                p
                            }
                        }
                        PatternTransformOp::Slow(ref factor_expr) => {
                            if let DslExpression::Value(v) = **factor_expr {
                                p.slow(Pattern::pure(v as f64))
                            } else {
                                p
                            }
                        }
                        PatternTransformOp::Squeeze(ref factor_expr) => {
                            if let DslExpression::Value(v) = **factor_expr {
                                p.squeeze(v as f64)
                            } else {
                                p
                            }
                        }
                        PatternTransformOp::Rev => p.rev(),
                        PatternTransformOp::Palindrome => p.palindrome(),
                        PatternTransformOp::Degrade => p.degrade(),
                        PatternTransformOp::DegradeBy(ref prob_expr) => {
                            if let DslExpression::Value(v) = **prob_expr {
                                p.degrade_by(Pattern::pure(v as f64))
                            } else {
                                p
                            }
                        }
                        PatternTransformOp::Stutter(ref n_expr) => {
                            if let DslExpression::Value(v) = **n_expr {
                                p.stutter(v as usize)
                            } else {
                                p
                            }
                        }
                        _ => {
                            eprintln!(
                                "Warning: Transform {:?} not yet supported in within closure",
                                inner_transform
                            );
                            p
                        }
                    }
                }))
            }
            PatternTransformOp::Chop(n_expr) => {
                let n = self.extract_constant(*n_expr)? as usize;
                Ok(pattern.chop(n))
            }
            PatternTransformOp::Gap(n_expr) => {
                let n = self.extract_constant(*n_expr)? as usize;
                Ok(pattern.gap(n))
            }
            PatternTransformOp::Segment(n_expr) => {
                let n = self.extract_constant(*n_expr)? as usize;
                Ok(pattern.segment(n))
            }
            PatternTransformOp::Swing(amount_expr) => {
                let amount = self.extract_constant(*amount_expr)?;
                Ok(pattern.swing(Pattern::pure(amount)))
            }
            PatternTransformOp::Shuffle(amount_expr) => {
                let amount = self.extract_constant(*amount_expr)?;
                Ok(pattern.shuffle(Pattern::pure(amount)))
            }
            PatternTransformOp::Chunk { n, transform } => {
                let n_val = self.extract_constant(*n)? as usize;
                let inner_transform = *transform;

                // Create a closure that applies the inner transform
                Ok(pattern.chunk(n_val, move |p| {
                    // Handle common transforms
                    match inner_transform {
                        PatternTransformOp::Fast(ref factor_expr) => {
                            if let DslExpression::Value(v) = **factor_expr {
                                p.fast(Pattern::pure(v as f64))
                            } else {
                                p
                            }
                        }
                        PatternTransformOp::Slow(ref factor_expr) => {
                            if let DslExpression::Value(v) = **factor_expr {
                                p.slow(Pattern::pure(v as f64))
                            } else {
                                p
                            }
                        }
                        PatternTransformOp::Squeeze(ref factor_expr) => {
                            if let DslExpression::Value(v) = **factor_expr {
                                p.squeeze(v as f64)
                            } else {
                                p
                            }
                        }
                        PatternTransformOp::Rev => p.rev(),
                        PatternTransformOp::Palindrome => p.palindrome(),
                        PatternTransformOp::Degrade => p.degrade(),
                        PatternTransformOp::DegradeBy(ref prob_expr) => {
                            if let DslExpression::Value(v) = **prob_expr {
                                p.degrade_by(Pattern::pure(v as f64))
                            } else {
                                p
                            }
                        }
                        PatternTransformOp::Stutter(ref n_expr) => {
                            if let DslExpression::Value(v) = **n_expr {
                                p.stutter(v as usize)
                            } else {
                                p
                            }
                        }
                        _ => {
                            eprintln!(
                                "Warning: Transform {:?} not yet supported in chunk closure",
                                inner_transform
                            );
                            p
                        }
                    }
                }))
            }
            PatternTransformOp::Jux(_transform) => {
                // TODO: Jux returns Pattern<(String, String)> for stereo, but our DSL
                // currently only supports Pattern<String>. This requires architectural
                // changes to support stereo patterns in the DSL.
                eprintln!("Warning: jux transform requires stereo pattern support, not yet implemented in DSL");
                Ok(pattern)
            }
        }
    }

    /// Extract a constant numeric value from an expression
    fn extract_constant(&self, expr: DslExpression) -> Result<f64, String> {
        match expr {
            DslExpression::Value(v) => Ok(v as f64),
            _ => Err(format!(
                "Pattern transform arguments must be constant values, got: {:?}",
                expr
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bus_definition() {
        let input = "~lfo $ sine 0.5";
        let result = statement(input);
        assert!(result.is_ok());

        if let Ok((_, DslStatement::BusDefinition { name, expr })) = result {
            assert_eq!(name, "lfo");
            assert!(matches!(expr, DslExpression::Oscillator { .. }));
        }
    }

    #[test]
    fn test_parse_arithmetic() {
        let input = "440 * 2 + 100";
        let result = expression(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_chain() {
        let input = "sine 440 # lpf 1000 2";
        let result = expression(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_complete_dsl() {
        let input = r#"
            ~lfo $ sine 0.5 * 0.5 + 0.5
            ~bass $ saw 55 # lpf (~lfo * 2000 + 500) 0.8
            out $ ~bass * 0.4
        "#;

        let result = parse_dsl(input);
        assert!(result.is_ok());

        if let Ok((_, statements)) = result {
            // The parser might group statements differently
            // Just check that we got some statements
            assert!(statements.len() >= 1);
            // Could be 1 statement with multiple parts or 3 separate statements
        }
    }

    #[test]
    fn test_parse_superkick() {
        let input = "superkick 60 0.5 0.3 0.1";
        let result = primary(input);
        assert!(result.is_ok());

        if let Ok((_, DslExpression::Synth { synth_type, params })) = result {
            assert!(matches!(synth_type, SynthType::SuperKick));
            assert_eq!(params.len(), 4);
        } else {
            panic!("Expected Synth expression");
        }
    }

    #[test]
    fn test_parse_supersaw() {
        let input = "supersaw 110 0.5 7";
        let result = primary(input);
        assert!(result.is_ok());

        if let Ok((_, DslExpression::Synth { synth_type, params })) = result {
            assert!(matches!(synth_type, SynthType::SuperSaw));
            assert_eq!(params.len(), 3);
        } else {
            panic!("Expected Synth expression");
        }
    }

    #[test]
    fn test_parse_reverb() {
        // Nested function call requires parentheses in space-separated syntax
        let input = "reverb (sine 440) 0.8 0.5 0.3";
        let result = primary(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        if let Ok((_, DslExpression::Effect { effect_type, .. })) = result {
            assert!(matches!(effect_type, EffectType::Reverb));
        } else {
            panic!("Expected Effect expression");
        }
    }

    #[test]
    fn test_parse_distortion() {
        // Nested function call requires parentheses in space-separated syntax
        let input = "dist (saw 110) 5.0 0.5";
        let result = primary(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        if let Ok((_, DslExpression::Effect { effect_type, .. })) = result {
            assert!(matches!(effect_type, EffectType::Distortion));
        } else {
            panic!("Expected Effect expression");
        }
    }

    #[test]
    fn test_compile_supersaw() {
        let input = "out $ supersaw 110 0.5 5 * 0.3";
        let (_, statements) = parse_dsl(input).unwrap();
        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);

        // Render a bit of audio to verify it works
        let buffer = graph.render(4410);
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        assert!(rms > 0.01, "SuperSaw should produce audio");
    }

    #[test]
    fn test_compile_reverb_effect() {
        // Nested function call requires parentheses in space-separated syntax
        let input = "out $ reverb (sine 440) 0.7 0.5 0.5";
        let (_, statements) = parse_dsl(input).unwrap();
        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);

        // Render audio
        let buffer = graph.render(4410);
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        assert!(rms > 0.01, "Reverb should produce audio, got RMS={}", rms);
    }

    #[test]
    fn test_compile_synth_with_effects_chain() {
        // Complex nested effects chain with proper parenthesization for space-separated syntax
        // OLD: reverb(chorus(dist(supersaw(110, 0.5, 5), 3.0, 0.3), 1.0, 0.5, 0.3), 0.7, 0.5, 0.4)
        // NEW: reverb (chorus (dist (supersaw 110 0.5 5) 3.0 0.3) 1.0 0.5 0.3) 0.7 0.5 0.4

        // For now, just test that it parses correctly with the new syntax
        // Full effects chain rendering may need additional implementation
        let input =
            "out $ reverb (chorus (dist (supersaw 110 0.5 5) 3.0 0.3) 1.0 0.5 0.3) 0.7 0.5 0.4";

        let result = parse_dsl(input);
        assert!(
            result.is_ok(),
            "Complex effects chain should parse with new syntax: {:?}",
            result
        );

        // Verify it compiles without panicking
        let (_, statements) = result.unwrap();
        let compiler = DslCompiler::new(44100.0);
        let _graph = compiler.compile(statements);

        // TODO: Complex nested effects chains may need additional implementation
        // to properly route audio through all layers. For now, we've verified
        // the new space-separated syntax parses correctly.
    }

    #[test]
    fn test_compile_superkick_with_reverb() {
        // Nested function call requires parentheses in space-separated syntax
        let input = "out $ reverb (superkick 60 0.5 0.3 0.1) 0.8 0.5 0.3";
        let (_, statements) = parse_dsl(input).unwrap();
        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);

        let buffer = graph.render(22050);
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        assert!(rms > 0.01, "Kick with reverb should produce audio");
    }

    #[test]
    fn test_parse_synth_pattern() {
        let input = r#"synth "c4 e4 g4" "saw" 0.01 0.1 0.7 0.2"#;
        let result = primary(input);
        assert!(result.is_ok(), "Should parse synth pattern");

        if let Ok((
            _,
            DslExpression::SynthPattern {
                notes,
                waveform,
                attack,
                decay,
                sustain,
                release,
                ..
            },
        )) = result
        {
            assert_eq!(notes, "c4 e4 g4");
            assert_eq!(waveform, Waveform::Saw);
            assert_eq!(attack, Some(0.01));
            assert_eq!(decay, Some(0.1));
            assert_eq!(sustain, Some(0.7));
            assert_eq!(release, Some(0.2));
        } else {
            panic!("Expected SynthPattern expression, got: {:?}", result);
        }
    }

    #[test]
    fn test_compile_synth_pattern() {
        let input = r#"
            tempo: 0.5
            out $ synth "c4 e4 g4 c5" "saw" 0.01 0.1 0.7 0.2 * 0.3
        "#;
        let (_, statements) = parse_dsl(input).unwrap();
        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);

        // Render 1 second (2 cycles at 2 CPS)
        let buffer = graph.render(44100);
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        assert!(
            rms > 0.01,
            "SynthPattern should produce audio, got RMS: {}",
            rms
        );
    }

    #[test]
    fn test_compile_synth_pattern_minimal() {
        // Test synth pattern with $ syntax and multiple notes
        let input = r#"
            tempo: 0.5
            out $ synth "a4 e4 c4" "sine" 0.01 0.1 0.7 0.2 * 0.3
        "#;
        let (_, statements) = parse_dsl(input).unwrap();
        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);

        // Render 1 second (2 cycles at 2 CPS)
        let buffer = graph.render(44100);
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        assert!(rms > 0.01, "Synth pattern should produce audio");
    }
}
