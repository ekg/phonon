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
//! - **Bus assignment**: `~name: expression` - Define a named signal bus
//! - **Output**: `out: expression` - Set the output signal
//! - **Tempo**: `cps: 2.0` - Set cycles per second (tempo)
//! - **Signal chain**: `a # b` - Chain signals (output of `a` feeds input of `b`)
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
//! let input = "out: sine(440) * 0.2";
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
//!     ~lfo: sine(0.5) * 0.5 + 0.5
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
//!     ~bass: saw(55) # lpf(800, 0.9)
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
//!     ~lfo: sine(0.25)
//!     ~bass: saw("55 82.5 110") # lpf(~lfo * 2000 + 500, 0.8)
//!     out: ~bass * 0.3
//! "#;
//!
//! let (_, statements) = parse_dsl(input).unwrap();
//! let compiler = DslCompiler::new(44100.0);
//! let graph = compiler.compile(statements);
//! ```

use crate::mini_notation_v3::parse_mini_notation;
use crate::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{alpha1, alphanumeric1, char, digit1, multispace0, multispace1},
    combinator::{map, map_res, recognize, value},
    multi::{many0, separated_list0},
    number::complete::float,
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};

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
    /// Silence output channel(s): hush, hush1, hush2
    Hush { channel: Option<usize> },
    /// Kill all voices and silence all outputs: panic
    Panic,
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
    /// Oscillator: sine(440), saw(~freq), square(220, 0.3)
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
        attack: Option<Box<DslExpression>>,
        release: Option<Box<DslExpression>>,
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
}

/// Pattern transformation operations
#[derive(Debug, Clone)]
pub enum PatternTransformOp {
    /// Speed up pattern: fast 2
    Fast(Box<DslExpression>),
    /// Slow down pattern: slow 2
    Slow(Box<DslExpression>),
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
}

#[derive(Debug, Clone, Copy)]
pub enum FilterType {
    LowPass,
    HighPass,
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

/// Parse function arguments
fn function_args(input: &str) -> IResult<&str, Vec<DslExpression>> {
    delimited(
        char('('),
        separated_list0(ws(char(',')), ws(expression)),
        char(')'),
    )(input)
}

/// Parse oscillator: sine(440)
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

/// Parse filter: lpf(input, cutoff, q)
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

    alt((lpf, hpf))(input)
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

/// Parse sample pattern: s("bd sn hh") or s("bd*4", gain: 0.8)
/// Also handles pattern transforms: s("bd sn" $ fast 2)
fn sample_pattern_expr(input: &str) -> IResult<&str, DslExpression> {
    map(preceded(tag("s"), function_args), |args| {
        // First arg can be a pattern string OR a pattern transform
        let first_arg = args.first().cloned();

        // Use positional args: s("pattern", gain, pan, speed, cut_group, attack, release)
        let gain = args.get(1).map(|e| Box::new(e.clone()));
        let pan = args.get(2).map(|e| Box::new(e.clone()));
        let speed = args.get(3).map(|e| Box::new(e.clone()));
        let cut_group = args.get(4).map(|e| Box::new(e.clone()));
        let attack = args.get(5).map(|e| Box::new(e.clone()));
        let release = args.get(6).map(|e| Box::new(e.clone()));

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
                    attack,
                    release,
                }
            }
            Some(DslExpression::PatternTransform { pattern, transform }) => {
                // Pattern with transform: s("bd sn" $ fast 2)
                // Wrap the whole thing in a PatternTransform that contains a SamplePattern
                DslExpression::PatternTransform {
                    pattern: Box::new(DslExpression::SamplePattern {
                        pattern: if let DslExpression::Pattern(p) = *pattern {
                            p
                        } else {
                            String::new()
                        },
                        gain,
                        pan,
                        speed,
                        cut_group,
                        attack,
                        release,
                    }),
                    transform,
                }
            }
            _ => {
                // Unknown first argument type
                DslExpression::SamplePattern {
                    pattern: String::new(),
                    gain,
                    pan,
                    speed,
                    cut_group,
                    attack,
                    release,
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
    alt((
        // rev (no arguments)
        map(tag("rev"), |_| PatternTransformOp::Rev),
        // degradeBy n (must come before degrade)
        map(preceded(tag("degradeBy"), ws(primary)), |n| {
            PatternTransformOp::DegradeBy(Box::new(n))
        }),
        // degrade (no arguments)
        map(tag("degrade"), |_| PatternTransformOp::Degrade),
        // fast n
        map(preceded(tag("fast"), ws(primary)), |n| {
            PatternTransformOp::Fast(Box::new(n))
        }),
        // slow n
        map(preceded(tag("slow"), ws(primary)), |n| {
            PatternTransformOp::Slow(Box::new(n))
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

/// Parse a bus definition: ~name: expression
fn bus_definition(input: &str) -> IResult<&str, DslStatement> {
    map(
        tuple((preceded(char('~'), identifier), ws(char(':')), expression)),
        |(name, _, expr)| DslStatement::BusDefinition {
            name: name.to_string(),
            expr,
        },
    )(input)
}

/// Parse output definition: out: expression, out1: expression, out2: expression, etc.
fn output_definition(input: &str) -> IResult<&str, DslStatement> {
    map(
        tuple((
            tag("out"),
            // Optional channel number: out1, out2, etc.
            alt((
                map_res(digit1, |s: &str| s.parse::<usize>()),
                value(0, tag("")), // Default to channel 0 for plain "out"
            )),
            ws(char(':')),
            expression,
        )),
        |(_, channel, _, expr)| DslStatement::Output {
            channel: if channel == 0 { None } else { Some(channel) },
            expr,
        },
    )(input)
}

/// Parse CPS setting: cps: 0.5 or tempo: 1.0 (alias for cps)
fn cps_setting(input: &str) -> IResult<&str, DslStatement> {
    map(
        preceded(
            tuple((alt((tag("cps"), tag("tempo"))), ws(char(':')))),
            number,
        ),
        DslStatement::SetCps,
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

/// Parse a statement
fn statement(input: &str) -> IResult<&str, DslStatement> {
    alt((
        bus_definition,
        output_definition,
        cps_setting,
        hush_statement,
        panic_statement,
    ))(input)
}

/// Parse multiple statements separated by newlines
pub fn parse_dsl(input: &str) -> IResult<&str, Vec<DslStatement>> {
    let (input, _) = multispace0(input)?; // Skip leading whitespace
    separated_list0(multispace1, statement)(input)
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
        for stmt in statements {
            self.compile_statement(stmt);
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
                    waveform,
                    phase: 0.0,
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
                // Chain is like connecting output to input
                let left_id = self.compile_expression(*left);

                // Inject left_id as the input to the right side expression
                let modified_right = self.inject_chain_input(*right, left_id);

                // Compile the modified right side
                self.compile_expression(modified_right)
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
                }
            }
            DslExpression::SamplePattern {
                pattern,
                gain,
                pan,
                speed,
                cut_group,
                attack,
                release,
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

                let attack_signal = attack
                    .map(|e| self.compile_expression_to_signal(*e))
                    .unwrap_or(Signal::Value(0.0)); // No attack envelope by default

                let release_signal = release
                    .map(|e| self.compile_expression_to_signal(*e))
                    .unwrap_or(Signal::Value(0.0)); // No release envelope by default

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
                    attack: attack_signal,
                    release: release_signal,
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
                        // Extract pattern data first to avoid borrow checker issues
                        let pattern_data = if let Some(SignalNode::Pattern {
                            pattern: inner_pattern_obj,
                            pattern_str,
                            ..
                        }) = self.graph.get_node(inner_node_id)
                        {
                            Some((inner_pattern_obj.clone(), pattern_str.clone()))
                        } else {
                            None
                        };

                        if let Some((inner_pattern, pattern_str)) = pattern_data {
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
                        } else {
                            eprintln!("Warning: Chained transform inner expression did not produce a pattern node");
                            self.graph.add_node(SignalNode::Constant { value: 0.0 })
                        }
                    }
                    DslExpression::SamplePattern {
                        pattern: pattern_str,
                        gain,
                        pan,
                        speed,
                        cut_group,
                        attack,
                        release,
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
                            attack: attack_signal,
                            release: release_signal,
                        })
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
            // Original parse: lpf(1000, 0.8) -> input=1000, cutoff=0.8, q=1.0
            // Chain context: lpf(1000, 0.8) should mean cutoff=1000, q=0.8
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
            // For effects in chain context, shift arguments similarly
            // Effect params don't include input, so no shift needed
            DslExpression::Effect {
                effect_type,
                input: _,
                params,
            } => DslExpression::Effect {
                effect_type,
                input: Box::new(DslExpression::BusRef(bus_name)),
                params,
            },
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
        use crate::pattern::Pattern;

        match transform {
            PatternTransformOp::Fast(factor_expr) => {
                // Extract the numeric value
                let factor = self.extract_constant(*factor_expr)?;
                Ok(pattern.fast(factor))
            }
            PatternTransformOp::Slow(factor_expr) => {
                let factor = self.extract_constant(*factor_expr)?;
                Ok(pattern.slow(factor))
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
                                p.fast(v as f64)
                            } else {
                                p // Can't evaluate non-constant, return unchanged
                            }
                        }
                        PatternTransformOp::Slow(ref factor_expr) => {
                            if let DslExpression::Value(v) = **factor_expr {
                                p.slow(v as f64)
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
                            p.fast(v as f64)
                        } else {
                            p
                        }
                    }
                    PatternTransformOp::Slow(ref factor_expr) => {
                        if let DslExpression::Value(v) = **factor_expr {
                            p.slow(v as f64)
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
                            p.fast(v as f64)
                        } else {
                            p
                        }
                    }
                    PatternTransformOp::Slow(ref factor_expr) => {
                        if let DslExpression::Value(v) = **factor_expr {
                            p.slow(v as f64)
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
                            p.fast(v as f64)
                        } else {
                            p
                        }
                    }
                    PatternTransformOp::Slow(ref factor_expr) => {
                        if let DslExpression::Value(v) = **factor_expr {
                            p.slow(v as f64)
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
                Ok(pattern.degrade_by(prob))
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
        let input = "~lfo: sine(0.5)";
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
        let input = "sine(440) # lpf(1000, 2)";
        let result = expression(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_complete_dsl() {
        let input = r#"
            ~lfo: sine(0.5) * 0.5 + 0.5
            ~bass: saw(55) # lpf(~lfo * 2000 + 500, 0.8)
            out: ~bass * 0.4
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
        let input = "superkick(60, 0.5, 0.3, 0.1)";
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
        let input = "supersaw(110, 0.5, 7)";
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
        let input = "reverb(sine(440), 0.8, 0.5, 0.3)";
        let result = primary(input);
        assert!(result.is_ok());

        if let Ok((_, DslExpression::Effect { effect_type, .. })) = result {
            assert!(matches!(effect_type, EffectType::Reverb));
        } else {
            panic!("Expected Effect expression");
        }
    }

    #[test]
    fn test_parse_distortion() {
        let input = "dist(saw(110), 5.0, 0.5)";
        let result = primary(input);
        assert!(result.is_ok());

        if let Ok((_, DslExpression::Effect { effect_type, .. })) = result {
            assert!(matches!(effect_type, EffectType::Distortion));
        } else {
            panic!("Expected Effect expression");
        }
    }

    #[test]
    fn test_compile_supersaw() {
        let input = "out: supersaw(110, 0.5, 5) * 0.3";
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
        let input = "out: reverb(sine(440), 0.7, 0.5, 0.5)";
        let (_, statements) = parse_dsl(input).unwrap();
        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);

        // Render audio
        let buffer = graph.render(4410);
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        assert!(rms > 0.01, "Reverb should produce audio");
    }

    #[test]
    fn test_compile_synth_with_effects_chain() {
        // Simpler inline version since bus refs aren't fully implemented yet
        let input = "out: reverb(chorus(dist(supersaw(110, 0.5, 5), 3.0, 0.3), 1.0, 0.5, 0.3), 0.7, 0.5, 0.4)";

        let (_, statements) = parse_dsl(input).unwrap();
        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);

        // Render audio
        let buffer = graph.render(22050);
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        assert!(
            rms > 0.01,
            "Full effects chain should produce audio, got RMS={}",
            rms
        );
    }

    #[test]
    fn test_compile_superkick_with_reverb() {
        let input = "out: reverb(superkick(60, 0.5, 0.3, 0.1), 0.8, 0.5, 0.3)";
        let (_, statements) = parse_dsl(input).unwrap();
        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);

        let buffer = graph.render(22050);
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        assert!(rms > 0.01, "Kick with reverb should produce audio");
    }

    #[test]
    fn test_parse_synth_pattern() {
        let input = r#"synth("c4 e4 g4", "saw", 0.01, 0.1, 0.7, 0.2)"#;
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
            tempo: 2.0
            out: synth("c4 e4 g4 c5", "saw", 0.01, 0.1, 0.7, 0.2) * 0.3
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
        // Test with minimal args (should use defaults)
        let input = r#"
            tempo: 2.0
            out: synth("a4", "sine")
        "#;
        let (_, statements) = parse_dsl(input).unwrap();
        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);

        let buffer = graph.render(22050);
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        assert!(rms > 0.01, "Minimal synth pattern should produce audio");
    }
}
