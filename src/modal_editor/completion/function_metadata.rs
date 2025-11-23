//! Function metadata for help and keyword argument completion
//!
//! Provides parameter information, descriptions, and examples for all Phonon functions

use std::collections::HashMap;

/// Parameter metadata
#[derive(Debug, Clone)]
pub struct ParamMetadata {
    /// Parameter name (for keyword arguments)
    pub name: &'static str,
    /// Parameter type description
    pub param_type: &'static str,
    /// Whether this parameter is optional
    pub optional: bool,
    /// Default value (if optional)
    pub default: Option<&'static str>,
    /// Parameter description
    pub description: &'static str,
}

/// Function metadata
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// Function name
    pub name: &'static str,
    /// Short description
    pub description: &'static str,
    /// Parameters in order
    pub params: Vec<ParamMetadata>,
    /// Example usage
    pub example: &'static str,
    /// Category
    pub category: &'static str,
}

impl FunctionMetadata {
    /// Get parameter by name
    pub fn get_param(&self, name: &str) -> Option<&ParamMetadata> {
        self.params.iter().find(|p| p.name == name)
    }

    /// Get parameter by position
    pub fn get_param_at(&self, index: usize) -> Option<&ParamMetadata> {
        self.params.get(index)
    }

    /// Format parameter signature for display
    pub fn param_signature(&self) -> String {
        let params: Vec<String> = self
            .params
            .iter()
            .map(|p| {
                if p.optional {
                    if let Some(default) = p.default {
                        format!("[:{} {}={}]", p.name, p.param_type, default)
                    } else {
                        format!("[:{} {}]", p.name, p.param_type)
                    }
                } else {
                    format!(":{} {}", p.name, p.param_type)
                }
            })
            .collect();

        params.join(" ")
    }
}

lazy_static::lazy_static! {
    /// Global function metadata registry
    pub static ref FUNCTION_METADATA: HashMap<&'static str, FunctionMetadata> = {
        let mut m = HashMap::new();

        // Filters
        m.insert("lpf", FunctionMetadata {
            name: "lpf",
            description: "Low-pass filter - removes frequencies above cutoff",
            params: vec![
                ParamMetadata {
                    name: "cutoff",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Filter cutoff frequency in Hz",
                },
                ParamMetadata {
                    name: "q",
                    param_type: "float",
                    optional: true,
                    default: Some("1.0"),
                    description: "Filter resonance/Q factor (0.1-10)",
                },
            ],
            example: "~bass: saw 55 # lpf 800 :q 1.5",
            category: "Filters",
        });

        m.insert("hpf", FunctionMetadata {
            name: "hpf",
            description: "High-pass filter - removes frequencies below cutoff",
            params: vec![
                ParamMetadata {
                    name: "cutoff",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Filter cutoff frequency in Hz",
                },
                ParamMetadata {
                    name: "q",
                    param_type: "float",
                    optional: true,
                    default: Some("1.0"),
                    description: "Filter resonance/Q factor (0.1-10)",
                },
            ],
            example: "~noise: noise # hpf 2000 :q 0.8",
            category: "Filters",
        });

        m.insert("bpf", FunctionMetadata {
            name: "bpf",
            description: "Band-pass filter - only allows frequencies near cutoff",
            params: vec![
                ParamMetadata {
                    name: "cutoff",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Filter center frequency in Hz",
                },
                ParamMetadata {
                    name: "q",
                    param_type: "float",
                    optional: true,
                    default: Some("1.0"),
                    description: "Filter resonance/Q factor (0.1-10)",
                },
            ],
            example: "~vocal: noise # bpf 1000 :q 5.0",
            category: "Filters",
        });

        m.insert("bq_hp", FunctionMetadata {
            name: "bq_hp",
            description: "Biquad highpass filter - efficient CPU-friendly filter",
            params: vec![
                ParamMetadata {
                    name: "frequency",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Filter cutoff frequency in Hz",
                },
                ParamMetadata {
                    name: "q",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Filter Q/resonance factor",
                },
            ],
            example: "~filtered: ~signal # bq_hp 200 0.7",
            category: "Filters",
        });

        m.insert("bq_lp", FunctionMetadata {
            name: "bq_lp",
            description: "Biquad lowpass filter - efficient CPU-friendly filter",
            params: vec![
                ParamMetadata {
                    name: "frequency",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Filter cutoff frequency in Hz",
                },
                ParamMetadata {
                    name: "q",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Filter Q/resonance factor",
                },
            ],
            example: "~filtered: ~signal # bq_lp 1000 0.7",
            category: "Filters",
        });

        m.insert("brown_noise", FunctionMetadata {
            name: "brown_noise",
            description: "Brown noise generator - 6dB/octave rolloff, warm rumble",
            params: vec![],
            example: "~rumble: brown_noise * 0.3",
            category: "Generators",
        });

        m.insert("notch", FunctionMetadata {
            name: "notch",
            description: "Notch filter - removes frequencies near cutoff",
            params: vec![
                ParamMetadata {
                    name: "cutoff",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Filter center frequency in Hz",
                },
                ParamMetadata {
                    name: "q",
                    param_type: "float",
                    optional: true,
                    default: Some("1.0"),
                    description: "Filter resonance/Q factor (0.1-10)",
                },
            ],
            example: "~clean: ~signal # notch 1000 :q 3.0",
            category: "Filters",
        });

        // Envelopes
        m.insert("adsr", FunctionMetadata {
            name: "adsr",
            description: "ADSR envelope - attack, decay, sustain, release",
            params: vec![
                ParamMetadata {
                    name: "attack",
                    param_type: "seconds",
                    optional: false,
                    default: None,
                    description: "Attack time in seconds",
                },
                ParamMetadata {
                    name: "decay",
                    param_type: "seconds",
                    optional: false,
                    default: None,
                    description: "Decay time in seconds",
                },
                ParamMetadata {
                    name: "sustain",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.7"),
                    description: "Sustain level (0-1)",
                },
                ParamMetadata {
                    name: "release",
                    param_type: "seconds",
                    optional: true,
                    default: Some("0.2"),
                    description: "Release time in seconds",
                },
            ],
            example: "~env: adsr 0.01 0.1 :sustain 0.8 :release 0.3",
            category: "Envelopes",
        });

        m.insert("ad", FunctionMetadata {
            name: "ad",
            description: "AD envelope - attack, decay",
            params: vec![
                ParamMetadata {
                    name: "attack",
                    param_type: "seconds",
                    optional: false,
                    default: None,
                    description: "Attack time in seconds",
                },
                ParamMetadata {
                    name: "decay",
                    param_type: "seconds",
                    optional: false,
                    default: None,
                    description: "Decay time in seconds",
                },
            ],
            example: "~env: ad :attack 0.01 :decay 0.3",
            category: "Envelopes",
        });

        m.insert("asr", FunctionMetadata {
            name: "asr",
            description: "ASR envelope - attack, sustain, release (gate-triggered)",
            params: vec![
                ParamMetadata {
                    name: "gate",
                    param_type: "signal",
                    optional: false,
                    default: None,
                    description: "Gate signal (trigger)",
                },
                ParamMetadata {
                    name: "attack",
                    param_type: "seconds",
                    optional: false,
                    default: None,
                    description: "Attack time in seconds",
                },
                ParamMetadata {
                    name: "release",
                    param_type: "seconds",
                    optional: false,
                    default: None,
                    description: "Release time in seconds",
                },
            ],
            example: "~env: asr :gate ~trigger :attack 0.02 :release 0.15",
            category: "Envelopes",
        });

        // Effects
        m.insert("reverb", FunctionMetadata {
            name: "reverb",
            description: "Reverb effect - adds space and ambience",
            params: vec![
                ParamMetadata {
                    name: "room_size",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "Room size (0-1)",
                },
                ParamMetadata {
                    name: "damping",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "High frequency damping (0-1)",
                },
                ParamMetadata {
                    name: "mix",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.3"),
                    description: "Wet/dry mix (0-1)",
                },
            ],
            example: "~wet: ~dry # reverb 0.8 0.5 :mix 0.4",
            category: "Effects",
        });

        m.insert("chorus", FunctionMetadata {
            name: "chorus",
            description: "Chorus effect - adds richness and width",
            params: vec![
                ParamMetadata {
                    name: "rate",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "LFO rate in Hz",
                },
                ParamMetadata {
                    name: "depth",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "Modulation depth (0-1)",
                },
                ParamMetadata {
                    name: "mix",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.3"),
                    description: "Wet/dry mix (0-1)",
                },
            ],
            example: "~wide: ~synth # chorus 2.0 0.3 :mix 0.5",
            category: "Effects",
        });

        m.insert("delay", FunctionMetadata {
            name: "delay",
            description: "Delay effect - echo/repeat",
            params: vec![
                ParamMetadata {
                    name: "time",
                    param_type: "seconds",
                    optional: false,
                    default: None,
                    description: "Delay time in seconds",
                },
                ParamMetadata {
                    name: "feedback",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.5"),
                    description: "Feedback amount (0-1)",
                },
                ParamMetadata {
                    name: "mix",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.5"),
                    description: "Wet/dry mix (0-1)",
                },
            ],
            example: "~echo: ~dry # delay 0.25 :feedback 0.6 :mix 0.4",
            category: "Effects",
        });

        m.insert("distort", FunctionMetadata {
            name: "distort",
            description: "Distortion effect - adds harmonic saturation",
            params: vec![
                ParamMetadata {
                    name: "drive",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Distortion amount/gain",
                },
                ParamMetadata {
                    name: "mix",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.5"),
                    description: "Wet/dry mix (0-1)",
                },
            ],
            example: "~heavy: ~clean # distort 5.0 :mix 0.8",
            category: "Effects",
        });

        // Pattern Functions
        m.insert("s", FunctionMetadata {
            name: "s",
            description: "Sample trigger - plays samples from ~/dirt-samples/",
            params: vec![
                ParamMetadata {
                    name: "pattern",
                    param_type: "string",
                    optional: false,
                    default: None,
                    description: "Sample pattern in mini-notation",
                },
            ],
            example: "~drums: s \"bd sn hh*4 cp\"",
            category: "Patterns",
        });

        // Sample Modifiers
        m.insert("gain", FunctionMetadata {
            name: "gain",
            description: "Adjust sample amplitude/volume",
            params: vec![
                ParamMetadata {
                    name: "amount",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Gain multiplier (1.0 = normal, 0.5 = half volume, 2.0 = double)",
                },
            ],
            example: "~drums: s \"bd sn\" # gain 0.8",
            category: "Modifiers",
        });

        m.insert("pan", FunctionMetadata {
            name: "pan",
            description: "Control stereo panning position",
            params: vec![
                ParamMetadata {
                    name: "position",
                    param_type: "-1 to 1",
                    optional: false,
                    default: None,
                    description: "Pan position (-1 = left, 0 = center, 1 = right)",
                },
            ],
            example: "~drums: s \"bd sn\" # pan \"-1 1\"",
            category: "Modifiers",
        });

        m.insert("speed", FunctionMetadata {
            name: "speed",
            description: "Change sample playback speed and pitch",
            params: vec![
                ParamMetadata {
                    name: "rate",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Playback speed (1.0 = normal, 2.0 = double speed/octave up, -1.0 = reverse)",
                },
            ],
            example: "~fast: s \"bd\" # speed 2.0\n~reverse: s \"sn\" # speed -1.0",
            category: "Modifiers",
        });

        m.insert("begin", FunctionMetadata {
            name: "begin",
            description: "Set sample start point for slicing",
            params: vec![
                ParamMetadata {
                    name: "position",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "Start position (0.0 = beginning, 0.5 = middle, 1.0 = end)",
                },
            ],
            example: "~slice: s \"bd\" # begin 0.5",
            category: "Modifiers",
        });

        m.insert("end", FunctionMetadata {
            name: "end",
            description: "Set sample end point for slicing",
            params: vec![
                ParamMetadata {
                    name: "position",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "End position (0.0 = beginning, 1.0 = end)",
                },
            ],
            example: "~slice: s \"bd\" # begin 0.25 # end 0.75",
            category: "Modifiers",
        });

        // Oscillators
        m.insert("sine", FunctionMetadata {
            name: "sine",
            description: "Sine wave oscillator",
            params: vec![
                ParamMetadata {
                    name: "freq",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Oscillator frequency in Hz",
                },
            ],
            example: "~tone: sine 440\n~keyword: sine :freq 440",
            category: "Oscillators",
        });

        m.insert("saw", FunctionMetadata {
            name: "saw",
            description: "Sawtooth wave oscillator",
            params: vec![
                ParamMetadata {
                    name: "freq",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Oscillator frequency in Hz",
                },
            ],
            example: "~bass: saw 55\n~keyword: saw :freq 110",
            category: "Oscillators",
        });

        m.insert("square", FunctionMetadata {
            name: "square",
            description: "Square wave oscillator",
            params: vec![
                ParamMetadata {
                    name: "freq",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Oscillator frequency in Hz",
                },
            ],
            example: "~lead: square 220\n~keyword: square :freq 440",
            category: "Oscillators",
        });

        m.insert("tri", FunctionMetadata {
            name: "tri",
            description: "Triangle wave oscillator",
            params: vec![
                ParamMetadata {
                    name: "freq",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Oscillator frequency in Hz",
                },
            ],
            example: "~pad: tri 330\n~keyword: tri :freq 660",
            category: "Oscillators",
        });

        m.insert("fast", FunctionMetadata {
            name: "fast",
            description: "Speed up pattern - plays N times faster",
            params: vec![
                ParamMetadata {
                    name: "factor",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Speed multiplier (2 = twice as fast)",
                },
            ],
            example: "~drums: s \"bd sn\" $ fast 2",
            category: "Transforms",
        });

        m.insert("slow", FunctionMetadata {
            name: "slow",
            description: "Slow down pattern - plays N times slower",
            params: vec![
                ParamMetadata {
                    name: "factor",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Slowdown multiplier (2 = half speed)",
                },
            ],
            example: "~slow_drums: s \"bd sn\" $ slow 2",
            category: "Transforms",
        });

        m.insert("every", FunctionMetadata {
            name: "every",
            description: "Apply transformation every Nth cycle",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Apply every N cycles",
                },
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transformation to apply",
                },
            ],
            example: "~drums: s \"bd sn\" $ every 4 (fast 2)",
            category: "Transforms",
        });

        // Time Manipulation Transforms
        m.insert("shuffle", FunctionMetadata {
            name: "shuffle",
            description: "Randomly shift events in time by amount",
            params: vec![
                ParamMetadata {
                    name: "amount",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Maximum time shift (0.0-1.0)",
                },
            ],
            example: "~drums: s \"bd sn hh cp\" $ shuffle 0.5",
            category: "Transforms",
        });

        m.insert("chop", FunctionMetadata {
            name: "chop",
            description: "Slice pattern into N parts and stack (play simultaneously)",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of slices",
                },
            ],
            example: "~chopped: s \"bd sn\" $ chop 8",
            category: "Transforms",
        });

        m.insert("striate", FunctionMetadata {
            name: "striate",
            description: "Alias for chop - slice and stack pattern",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of slices",
                },
            ],
            example: "~striate: s \"bd sn\" $ striate 4",
            category: "Transforms",
        });

        m.insert("slice", FunctionMetadata {
            name: "slice",
            description: "Reorder N slices by index pattern (deterministic control)",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of slices",
                },
                ParamMetadata {
                    name: "indices",
                    param_type: "pattern",
                    optional: false,
                    default: None,
                    description: "Pattern of indices to select slices",
                },
            ],
            example: "~sliced: s \"bd sn hh cp\" $ slice 4 \"3 2 1 0\"",
            category: "Transforms",
        });

        m.insert("scramble", FunctionMetadata {
            name: "scramble",
            description: "Fisher-Yates shuffle - randomize event order",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of subdivisions to scramble",
                },
            ],
            example: "~scrambled: s \"bd sn hh cp\" $ scramble 4",
            category: "Transforms",
        });

        // Pattern Structure Transforms
        m.insert("rev", FunctionMetadata {
            name: "rev",
            description: "Reverse pattern - plays backwards",
            params: vec![],
            example: "~reversed: s \"bd sn hh cp\" $ rev",
            category: "Transforms",
        });

        m.insert("palindrome", FunctionMetadata {
            name: "palindrome",
            description: "Pattern followed by its reverse",
            params: vec![],
            example: "~palindrome: s \"bd sn hh\" $ palindrome",
            category: "Transforms",
        });

        m.insert("mirror", FunctionMetadata {
            name: "mirror",
            description: "Alias for palindrome - pattern then reverse",
            params: vec![],
            example: "~mirrored: s \"bd sn\" $ mirror",
            category: "Transforms",
        });

        m.insert("rotL", FunctionMetadata {
            name: "rotL",
            description: "Rotate pattern left by N steps",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of steps to rotate left",
                },
            ],
            example: "~rotated: s \"bd sn hh cp\" $ rotL 1",
            category: "Transforms",
        });

        m.insert("rotR", FunctionMetadata {
            name: "rotR",
            description: "Rotate pattern right by N steps",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of steps to rotate right",
                },
            ],
            example: "~rotated: s \"bd sn hh cp\" $ rotR 1",
            category: "Transforms",
        });

        // Timing/Feel Transforms
        m.insert("swing", FunctionMetadata {
            name: "swing",
            description: "Add swing feel - delays every other event",
            params: vec![
                ParamMetadata {
                    name: "amount",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Swing amount (0.0-1.0)",
                },
            ],
            example: "~swung: s \"bd*8\" $ swing 0.5",
            category: "Transforms",
        });

        m.insert("late", FunctionMetadata {
            name: "late",
            description: "Delay pattern in time",
            params: vec![
                ParamMetadata {
                    name: "amount",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Time delay in cycles",
                },
            ],
            example: "~delayed: s \"bd sn\" $ late 0.25",
            category: "Transforms",
        });

        m.insert("early", FunctionMetadata {
            name: "early",
            description: "Shift pattern earlier in time",
            params: vec![
                ParamMetadata {
                    name: "amount",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Time advance in cycles",
                },
            ],
            example: "~advanced: s \"bd sn\" $ early 0.1",
            category: "Transforms",
        });

        m.insert("offset", FunctionMetadata {
            name: "offset",
            description: "Alias for late - shift pattern in time",
            params: vec![
                ParamMetadata {
                    name: "amount",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Time offset in cycles",
                },
            ],
            example: "~offset: s \"bd sn\" $ offset 0.5",
            category: "Transforms",
        });

        // Duration Transforms
        m.insert("legato", FunctionMetadata {
            name: "legato",
            description: "Adjust event duration - makes events longer/shorter",
            params: vec![
                ParamMetadata {
                    name: "factor",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Duration multiplier (>1 = longer, <1 = shorter)",
                },
            ],
            example: "~legato: s \"bd sn\" $ legato 2.0",
            category: "Transforms",
        });

        m.insert("staccato", FunctionMetadata {
            name: "staccato",
            description: "Make events shorter (opposite of legato)",
            params: vec![
                ParamMetadata {
                    name: "factor",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Shortening factor (0.5 = half duration)",
                },
            ],
            example: "~short: s \"bd sn\" $ staccato 0.5",
            category: "Transforms",
        });

        m.insert("stretch", FunctionMetadata {
            name: "stretch",
            description: "Sustain notes to fill gaps (legato 1.0)",
            params: vec![],
            example: "~stretched: s \"bd ~ sn ~\" $ stretch",
            category: "Transforms",
        });

        m.insert("linger", FunctionMetadata {
            name: "linger",
            description: "Linger on values for longer",
            params: vec![
                ParamMetadata {
                    name: "factor",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Linger duration multiplier",
                },
            ],
            example: "~lingering: s \"bd sn\" $ linger 2.0",
            category: "Transforms",
        });

        // Repetition Transforms
        m.insert("stutter", FunctionMetadata {
            name: "stutter",
            description: "Repeat each event N times",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of repetitions",
                },
            ],
            example: "~stutter: s \"bd sn\" $ stutter 4",
            category: "Transforms",
        });

        m.insert("ply", FunctionMetadata {
            name: "ply",
            description: "Repeat each event N times (similar to stutter)",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of repetitions per event",
                },
            ],
            example: "~plied: s \"bd sn\" $ ply 3",
            category: "Transforms",
        });

        m.insert("dup", FunctionMetadata {
            name: "dup",
            description: "Duplicate pattern N times (like bd*n)",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of duplications",
                },
            ],
            example: "~duped: s \"bd sn\" $ dup 4",
            category: "Transforms",
        });

        m.insert("iter", FunctionMetadata {
            name: "iter",
            description: "Iterate pattern shifting by 1/N each cycle",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of iterations",
                },
            ],
            example: "~iter: s \"bd sn hh cp\" $ iter 4",
            category: "Transforms",
        });

        m.insert("iterBack", FunctionMetadata {
            name: "iterBack",
            description: "Iterate pattern backwards",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of iterations",
                },
            ],
            example: "~iterBack: s \"bd sn hh cp\" $ iterBack 4",
            category: "Transforms",
        });

        m.insert("echo", FunctionMetadata {
            name: "echo",
            description: "Echo/delay effect on pattern level",
            params: vec![
                ParamMetadata {
                    name: "times",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of echoes",
                },
                ParamMetadata {
                    name: "time",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Time between echoes (cycles)",
                },
                ParamMetadata {
                    name: "feedback",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Feedback amount (0.0-1.0)",
                },
            ],
            example: "~echoed: s \"bd sn\" $ echo 3 0.25 0.5",
            category: "Transforms",
        });

        // Control/Filter Transforms
        m.insert("degrade", FunctionMetadata {
            name: "degrade",
            description: "Randomly remove events (50% probability)",
            params: vec![],
            example: "~degraded: s \"bd sn hh cp\" $ degrade",
            category: "Transforms",
        });

        m.insert("degradeBy", FunctionMetadata {
            name: "degradeBy",
            description: "Remove events with probability P",
            params: vec![
                ParamMetadata {
                    name: "probability",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Removal probability (0.0-1.0)",
                },
            ],
            example: "~sparse: s \"bd sn hh cp\" $ degradeBy 0.7",
            category: "Transforms",
        });

        m.insert("gap", FunctionMetadata {
            name: "gap",
            description: "Insert silence every N cycles",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Gap interval in cycles",
                },
            ],
            example: "~gapped: s \"bd sn\" $ gap 2",
            category: "Transforms",
        });

        m.insert("segment", FunctionMetadata {
            name: "segment",
            description: "Divide pattern into N segments",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of segments",
                },
            ],
            example: "~segmented: s \"bd sn\" $ segment 8",
            category: "Transforms",
        });

        // Time Range Transforms
        m.insert("zoom", FunctionMetadata {
            name: "zoom",
            description: "Focus on specific time range (begin to end)",
            params: vec![
                ParamMetadata {
                    name: "begin",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Start position (0.0-1.0)",
                },
                ParamMetadata {
                    name: "end",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "End position (0.0-1.0)",
                },
            ],
            example: "~zoomed: s \"bd sn hh cp\" $ zoom 0.25 0.75",
            category: "Transforms",
        });

        m.insert("compress", FunctionMetadata {
            name: "compress",
            description: "Compress pattern to time range",
            params: vec![
                ParamMetadata {
                    name: "begin",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Start position (0.0-1.0)",
                },
                ParamMetadata {
                    name: "end",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "End position (0.0-1.0)",
                },
            ],
            example: "~compressed: s \"bd sn\" $ compress 0.0 0.5",
            category: "Transforms",
        });

        m.insert("compressGap", FunctionMetadata {
            name: "compressGap",
            description: "Compress to range with gaps",
            params: vec![
                ParamMetadata {
                    name: "begin",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Start position (0.0-1.0)",
                },
                ParamMetadata {
                    name: "end",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "End position (0.0-1.0)",
                },
            ],
            example: "~gapcompress: s \"bd sn\" $ compressGap 0.0 0.25",
            category: "Transforms",
        });

        m.insert("fit", FunctionMetadata {
            name: "fit",
            description: "Fit pattern to N cycles",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Number of cycles to fit",
                },
            ],
            example: "~fitted: s \"bd sn hh cp\" $ fit 2",
            category: "Transforms",
        });

        // Advanced Transforms
        m.insert("spin", FunctionMetadata {
            name: "spin",
            description: "Rotate through N different versions",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of rotations",
                },
            ],
            example: "~spinning: s \"bd sn hh cp\" $ spin 4",
            category: "Transforms",
        });

        m.insert("loop", FunctionMetadata {
            name: "loop",
            description: "Loop pattern N times within cycle",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of loops per cycle",
                },
            ],
            example: "~looped: s \"bd sn\" $ loop 4",
            category: "Transforms",
        });

        m.insert("loopAt", FunctionMetadata {
            name: "loopAt",
            description: "Stretch pattern over N cycles AND slow sample playback (pitched down)",
            params: vec![
                ParamMetadata {
                    name: "cycles",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Number of cycles - affects timing AND playback speed",
                },
            ],
            example: "~slow: s \"bd sn hh cp\" $ loopAt 2",
            category: "Transforms",
        });

        m.insert("chew", FunctionMetadata {
            name: "chew",
            description: "Chew through pattern (granular slicing)",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Chew factor",
                },
            ],
            example: "~chewed: s \"bd sn\" $ chew 8",
            category: "Transforms",
        });

        m.insert("fastGap", FunctionMetadata {
            name: "fastGap",
            description: "Fast with gaps between repetitions",
            params: vec![
                ParamMetadata {
                    name: "factor",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Speed factor",
                },
            ],
            example: "~fastgap: s \"bd sn\" $ fastGap 2",
            category: "Transforms",
        });

        m.insert("discretise", FunctionMetadata {
            name: "discretise",
            description: "Quantize time to N divisions",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of time divisions",
                },
            ],
            example: "~quantized: s \"bd sn hh\" $ discretise 16",
            category: "Transforms",
        });

        m.insert("binary", FunctionMetadata {
            name: "binary",
            description: "Bit mask pattern for binary rhythms",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Binary number as bitmask",
                },
            ],
            example: "~binary: s \"bd\" $ binary 13",
            category: "Transforms",
        });

        m.insert("range", FunctionMetadata {
            name: "range",
            description: "Scale numeric values to min-max range",
            params: vec![
                ParamMetadata {
                    name: "min",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Minimum value",
                },
                ParamMetadata {
                    name: "max",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Maximum value",
                },
            ],
            example: "~scaled: sine \"0.5 1.0\" $ range 200 800",
            category: "Transforms",
        });

        m.insert("reset", FunctionMetadata {
            name: "reset",
            description: "Restart pattern every N cycles",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Reset interval in cycles",
                },
            ],
            example: "~reset: s \"bd sn hh cp\" $ reset 4",
            category: "Transforms",
        });

        m.insert("restart", FunctionMetadata {
            name: "restart",
            description: "Alias for reset - restart pattern every N cycles",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Restart interval in cycles",
                },
            ],
            example: "~restart: s \"bd sn\" $ restart 2",
            category: "Transforms",
        });

        m.insert("loopback", FunctionMetadata {
            name: "loopback",
            description: "Play backwards then forwards (bidirectional)",
            params: vec![],
            example: "~loopback: s \"bd sn hh cp\" $ loopback",
            category: "Transforms",
        });

        m.insert("squeeze", FunctionMetadata {
            name: "squeeze",
            description: "Compress to first 1/N of cycle and speed up",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Squeeze factor",
                },
            ],
            example: "~squeezed: s \"bd sn hh cp\" $ squeeze 4",
            category: "Transforms",
        });

        // Numeric Pattern Transforms
        m.insert("quantize", FunctionMetadata {
            name: "quantize",
            description: "Quantize numeric values to steps",
            params: vec![
                ParamMetadata {
                    name: "steps",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of quantization steps",
                },
            ],
            example: "~quant: sine \"0 1\" $ quantize 8",
            category: "Transforms",
        });

        m.insert("smooth", FunctionMetadata {
            name: "smooth",
            description: "Smooth numeric values (low-pass filter on pattern)",
            params: vec![
                ParamMetadata {
                    name: "amount",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Smoothing amount (0.0-1.0)",
                },
            ],
            example: "~smooth: sine \"0 1\" $ smooth 0.5",
            category: "Transforms",
        });

        m.insert("exp", FunctionMetadata {
            name: "exp",
            description: "Exponential transformation on numeric values",
            params: vec![
                ParamMetadata {
                    name: "base",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Exponent base",
                },
            ],
            example: "~exp: sine \"0 1\" $ exp 2",
            category: "Transforms",
        });

        m.insert("log", FunctionMetadata {
            name: "log",
            description: "Logarithmic transformation on numeric values",
            params: vec![
                ParamMetadata {
                    name: "base",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Logarithm base",
                },
            ],
            example: "~log: sine \"1 100\" $ log 10",
            category: "Transforms",
        });

        m.insert("walk", FunctionMetadata {
            name: "walk",
            description: "Random walk on numeric values",
            params: vec![
                ParamMetadata {
                    name: "step_size",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Maximum step size per change",
                },
            ],
            example: "~walk: sine 440 $ walk 50",
            category: "Transforms",
        });

        // Time/Cycle Transforms
        m.insert("focus", FunctionMetadata {
            name: "focus",
            description: "Focus on specific cycles (cycle range)",
            params: vec![
                ParamMetadata {
                    name: "cycle_begin",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Start cycle",
                },
                ParamMetadata {
                    name: "cycle_end",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "End cycle",
                },
            ],
            example: "~focused: s \"bd sn hh cp\" $ focus 0 2",
            category: "Transforms",
        });

        m.insert("trim", FunctionMetadata {
            name: "trim",
            description: "Trim pattern to time range (0.0-1.0 within cycle)",
            params: vec![
                ParamMetadata {
                    name: "begin",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Start position (0.0-1.0)",
                },
                ParamMetadata {
                    name: "end",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "End position (0.0-1.0)",
                },
            ],
            example: "~trimmed: s \"bd sn hh cp\" $ trim 0.25 0.75",
            category: "Transforms",
        });

        m.insert("wait", FunctionMetadata {
            name: "wait",
            description: "Delay pattern by N cycles",
            params: vec![
                ParamMetadata {
                    name: "cycles",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Number of cycles to wait",
                },
            ],
            example: "~waited: s \"bd sn\" $ wait 2",
            category: "Transforms",
        });

        m.insert("accelerate", FunctionMetadata {
            name: "accelerate",
            description: "Speed up pattern over time",
            params: vec![
                ParamMetadata {
                    name: "rate",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Acceleration rate",
                },
            ],
            example: "~accel: s \"bd sn\" $ accelerate 1.5",
            category: "Transforms",
        });

        // Conditional/Layering Transforms
        m.insert("inside", FunctionMetadata {
            name: "inside",
            description: "Apply transform only inside time range",
            params: vec![
                ParamMetadata {
                    name: "begin",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Start position (0.0-1.0)",
                },
                ParamMetadata {
                    name: "end",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "End position (0.0-1.0)",
                },
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transform to apply inside range",
                },
            ],
            example: "~inside: s \"bd sn hh cp\" $ inside 0.25 0.75 (fast 2)",
            category: "Transforms",
        });

        m.insert("outside", FunctionMetadata {
            name: "outside",
            description: "Apply transform only outside time range",
            params: vec![
                ParamMetadata {
                    name: "begin",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Start position (0.0-1.0)",
                },
                ParamMetadata {
                    name: "end",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "End position (0.0-1.0)",
                },
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transform to apply outside range",
                },
            ],
            example: "~outside: s \"bd sn hh cp\" $ outside 0.25 0.75 (fast 2)",
            category: "Transforms",
        });

        m.insert("within", FunctionMetadata {
            name: "within",
            description: "Apply transform within time window",
            params: vec![
                ParamMetadata {
                    name: "begin",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Start position (0.0-1.0)",
                },
                ParamMetadata {
                    name: "end",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "End position (0.0-1.0)",
                },
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transform to apply within window",
                },
            ],
            example: "~within: s \"bd sn\" $ within 0.0 0.5 (fast 4)",
            category: "Transforms",
        });

        m.insert("superimpose", FunctionMetadata {
            name: "superimpose",
            description: "Layer pattern with transformed version",
            params: vec![
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transform to apply to layered copy",
                },
            ],
            example: "~layered: s \"bd sn\" $ superimpose (fast 2)",
            category: "Transforms",
        });

        m.insert("chunk", FunctionMetadata {
            name: "chunk",
            description: "Divide into N chunks and apply transform to each",
            params: vec![
                ParamMetadata {
                    name: "n",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of chunks",
                },
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transform to apply per chunk",
                },
            ],
            example: "~chunked: s \"bd sn hh cp\" $ chunk 2 (fast 2)",
            category: "Transforms",
        });

        // Probabilistic Transforms
        m.insert("sometimes", FunctionMetadata {
            name: "sometimes",
            description: "Apply transform with 50% probability",
            params: vec![
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transform to sometimes apply",
                },
            ],
            example: "~maybe: s \"bd sn\" $ sometimes (fast 2)",
            category: "Transforms",
        });

        m.insert("often", FunctionMetadata {
            name: "often",
            description: "Apply transform with 75% probability",
            params: vec![
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transform to often apply",
                },
            ],
            example: "~often: s \"bd sn\" $ often (fast 2)",
            category: "Transforms",
        });

        m.insert("rarely", FunctionMetadata {
            name: "rarely",
            description: "Apply transform with 25% probability",
            params: vec![
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transform to rarely apply",
                },
            ],
            example: "~rare: s \"bd sn\" $ rarely (fast 2)",
            category: "Transforms",
        });

        m.insert("sometimesBy", FunctionMetadata {
            name: "sometimesBy",
            description: "Apply transform with specific probability",
            params: vec![
                ParamMetadata {
                    name: "probability",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Probability (0.0-1.0)",
                },
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transform to apply",
                },
            ],
            example: "~prob: s \"bd sn\" $ sometimesBy 0.3 (fast 2)",
            category: "Transforms",
        });

        m.insert("almostAlways", FunctionMetadata {
            name: "almostAlways",
            description: "Apply transform with 90% probability",
            params: vec![
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transform to almost always apply",
                },
            ],
            example: "~mostly: s \"bd sn\" $ almostAlways (fast 2)",
            category: "Transforms",
        });

        m.insert("almostNever", FunctionMetadata {
            name: "almostNever",
            description: "Apply transform with 10% probability",
            params: vec![
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transform to almost never apply",
                },
            ],
            example: "~seldom: s \"bd sn\" $ almostNever (fast 2)",
            category: "Transforms",
        });

        m.insert("always", FunctionMetadata {
            name: "always",
            description: "Always apply transform (100% probability)",
            params: vec![
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transform to always apply",
                },
            ],
            example: "~always: s \"bd sn\" $ always (fast 2)",
            category: "Transforms",
        });

        m.insert("whenmod", FunctionMetadata {
            name: "whenmod",
            description: "Apply when (cycle - offset) % modulo == 0",
            params: vec![
                ParamMetadata {
                    name: "modulo",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Modulo value",
                },
                ParamMetadata {
                    name: "offset",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Offset value",
                },
                ParamMetadata {
                    name: "transform",
                    param_type: "function",
                    optional: false,
                    default: None,
                    description: "Transform to apply",
                },
            ],
            example: "~when: s \"bd sn\" $ whenmod 4 0 (fast 2)",
            category: "Transforms",
        });

        // Pattern Manipulation Transforms
        m.insert("mask", FunctionMetadata {
            name: "mask",
            description: "Apply boolean mask pattern to filter events",
            params: vec![
                ParamMetadata {
                    name: "pattern",
                    param_type: "pattern",
                    optional: false,
                    default: None,
                    description: "Mask pattern (true/false values)",
                },
            ],
            example: "~masked: s \"bd sn hh cp\" $ mask \"t f t f\"",
            category: "Transforms",
        });

        m.insert("weave", FunctionMetadata {
            name: "weave",
            description: "Weave pattern with interleaving",
            params: vec![
                ParamMetadata {
                    name: "count",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Weave count",
                },
            ],
            example: "~woven: s \"bd sn\" $ weave 4",
            category: "Transforms",
        });

        m.insert("degradeSeed", FunctionMetadata {
            name: "degradeSeed",
            description: "Degrade with specific random seed (reproducible)",
            params: vec![
                ParamMetadata {
                    name: "seed",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Random seed value",
                },
            ],
            example: "~seeded: s \"bd sn hh cp\" $ degradeSeed 42",
            category: "Transforms",
        });

        m.insert("undegrade", FunctionMetadata {
            name: "undegrade",
            description: "Return pattern unchanged (opposite of degrade)",
            params: vec![],
            example: "~normal: s \"bd sn\" $ undegrade",
            category: "Transforms",
        });

        m.insert("humanize", FunctionMetadata {
            name: "humanize",
            description: "Add human timing and velocity variation",
            params: vec![
                ParamMetadata {
                    name: "time_var",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Time variation amount",
                },
                ParamMetadata {
                    name: "velocity_var",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Velocity variation amount",
                },
            ],
            example: "~human: s \"bd sn\" $ humanize 0.02 0.1",
            category: "Transforms",
        });

        m.insert("euclid", FunctionMetadata {
            name: "euclid",
            description: "Euclidean rhythm pattern (distribute pulses evenly)",
            params: vec![
                ParamMetadata {
                    name: "pulses",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of pulses",
                },
                ParamMetadata {
                    name: "steps",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Total steps",
                },
            ],
            example: "~euclid: s \"bd\" $ euclid 3 8",
            category: "Transforms",
        });

        m.insert("flanger", FunctionMetadata {
            name: "flanger",
            description: "Flanger effect - sweeping comb filter modulation",
            params: vec![
                ParamMetadata {
                    name: "depth",
                    param_type: "milliseconds",
                    optional: false,
                    default: None,
                    description: "Modulation depth in milliseconds (0-10ms typical)",
                },
                ParamMetadata {
                    name: "rate",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "LFO modulation rate in Hz (0.1-5 Hz typical)",
                },
                ParamMetadata {
                    name: "feedback",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "Feedback amount (0-0.9, higher = more metallic)",
                },
            ],
            example: "~flanged: ~synth # flanger 2.0 0.5 0.7",
            category: "Effects",
        });

        m.insert("granular", FunctionMetadata {
            name: "granular",
            description: "Granular synthesis - slice audio into grains for texture",
            params: vec![
                ParamMetadata {
                    name: "source",
                    param_type: "signal",
                    optional: false,
                    default: None,
                    description: "Source signal to granulate",
                },
                ParamMetadata {
                    name: "grain_size_ms",
                    param_type: "milliseconds",
                    optional: false,
                    default: None,
                    description: "Grain size in milliseconds (10-200ms typical)",
                },
                ParamMetadata {
                    name: "density",
                    param_type: "grains/sec",
                    optional: false,
                    default: None,
                    description: "Grain density (grains per second, 10-100 typical)",
                },
                ParamMetadata {
                    name: "pitch",
                    param_type: "ratio",
                    optional: false,
                    default: None,
                    description: "Pitch ratio (1.0 = normal, 0.5 = octave down, 2.0 = octave up)",
                },
            ],
            example: "~grains: granular ~input 50 20 1.0",
            category: "Effects",
        });

        m.insert("limiter", FunctionMetadata {
            name: "limiter",
            description: "Limiter - prevents clipping by hard limiting peaks",
            params: vec![
                ParamMetadata {
                    name: "input",
                    param_type: "signal",
                    optional: false,
                    default: None,
                    description: "Input signal to limit",
                },
                ParamMetadata {
                    name: "threshold",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "Threshold level (0-1, typically 0.9-0.99)",
                },
            ],
            example: "~safe: limiter ~master 0.95",
            category: "Effects",
        });

        m.insert("moog", FunctionMetadata {
            name: "moog",
            description: "Moog ladder filter - classic analog lowpass with resonance",
            params: vec![
                ParamMetadata {
                    name: "input",
                    param_type: "signal",
                    optional: false,
                    default: None,
                    description: "Input signal to filter",
                },
                ParamMetadata {
                    name: "cutoff",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Filter cutoff frequency in Hz",
                },
                ParamMetadata {
                    name: "resonance",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "Filter resonance (0-1, can self-oscillate at high values)",
                },
            ],
            example: "~bass: saw 55 # moog ~bass 800 0.7",
            category: "Filters",
        });

        m.insert("moog_hz", FunctionMetadata {
            name: "moog_hz",
            description: "Moog filter with Hz cutoff - fundsp-based implementation",
            params: vec![
                ParamMetadata {
                    name: "cutoff",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Filter cutoff frequency in Hz",
                },
                ParamMetadata {
                    name: "resonance",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "Filter resonance (0-1)",
                },
            ],
            example: "~filtered: ~synth # moog_hz 1000 0.5",
            category: "Filters",
        });

        m.insert("pink_noise", FunctionMetadata {
            name: "pink_noise",
            description: "Pink noise generator - 3dB/octave rolloff, natural spectrum",
            params: vec![],
            example: "~wind: pink_noise * 0.2",
            category: "Generators",
        });

        m.insert("pitch_shift", FunctionMetadata {
            name: "pitch_shift",
            description: "Pitch shifter - change pitch without changing duration",
            params: vec![
                ParamMetadata {
                    name: "input",
                    param_type: "signal",
                    optional: false,
                    default: None,
                    description: "Input signal to pitch shift",
                },
                ParamMetadata {
                    name: "semitones",
                    param_type: "semitones",
                    optional: false,
                    default: None,
                    description: "Pitch shift in semitones (12 = octave up, -12 = octave down)",
                },
            ],
            example: "~shifted: pitch_shift ~vocal 7",
            category: "Effects",
        });

        m.insert("plate", FunctionMetadata {
            name: "plate",
            description: "Dattorro plate reverb - lush, dense reverb with natural decay",
            params: vec![
                ParamMetadata {
                    name: "pre_delay",
                    param_type: "seconds",
                    optional: false,
                    default: None,
                    description: "Pre-delay time before reverb (0-0.1 seconds typical)",
                },
                ParamMetadata {
                    name: "decay",
                    param_type: "seconds",
                    optional: false,
                    default: None,
                    description: "Reverb decay time (0.1-20 seconds)",
                },
                ParamMetadata {
                    name: "diffusion",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.7"),
                    description: "Diffusion density (0-1, higher = denser)",
                },
                ParamMetadata {
                    name: "damping",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.3"),
                    description: "High frequency damping (0-1, higher = darker)",
                },
                ParamMetadata {
                    name: "mod_depth",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.3"),
                    description: "Modulation depth for shimmer (0-1)",
                },
                ParamMetadata {
                    name: "mix",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.5"),
                    description: "Wet/dry mix (0 = dry, 1 = wet)",
                },
            ],
            example: "~verb: ~dry # plate 0.02 2.5 :diffusion 0.8",
            category: "Effects",
        });

        m.insert("pulse", FunctionMetadata {
            name: "pulse",
            description: "Pulse wave oscillator - bandlimited PWM oscillator",
            params: vec![
                ParamMetadata {
                    name: "frequency",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Oscillator frequency in Hz",
                },
                ParamMetadata {
                    name: "pulse_width",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "Pulse width (0.5 = square, 0.1 = thin, 0.9 = wide)",
                },
            ],
            example: "~pwm: pulse 110 0.2",
            category: "Oscillators",
        });

        m.insert("reverb_stereo", FunctionMetadata {
            name: "reverb_stereo",
            description: "Stereo reverb - fundsp-based stereo reverb effect",
            params: vec![
                ParamMetadata {
                    name: "wet",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "Wet/dry mix (0 = dry, 1 = wet)",
                },
                ParamMetadata {
                    name: "time",
                    param_type: "seconds",
                    optional: false,
                    default: None,
                    description: "Reverb time/size (0.1-10 seconds)",
                },
            ],
            example: "~verb: ~synth # reverb_stereo 0.3 2.0",
            category: "Effects",
        });

        m.insert("ring_mod", FunctionMetadata {
            name: "ring_mod",
            description: "Ring modulation - multiply two signals for metallic tones",
            params: vec![
                ParamMetadata {
                    name: "signal1",
                    param_type: "signal",
                    optional: false,
                    default: None,
                    description: "First signal (carrier)",
                },
                ParamMetadata {
                    name: "signal2",
                    param_type: "signal",
                    optional: false,
                    default: None,
                    description: "Second signal (modulator)",
                },
            ],
            example: "~metal: ring_mod (saw 110) (sine 440)",
            category: "Effects",
        });

        m.insert("sample_hold", FunctionMetadata {
            name: "sample_hold",
            description: "Sample and hold - captures input on trigger, holds until next trigger",
            params: vec![
                ParamMetadata {
                    name: "input",
                    param_type: "signal",
                    optional: false,
                    default: None,
                    description: "Input signal to sample",
                },
                ParamMetadata {
                    name: "trigger",
                    param_type: "signal",
                    optional: false,
                    default: None,
                    description: "Trigger signal (samples on rising edge)",
                },
            ],
            example: "~stepped: sample_hold (sine 2) (sine 8)",
            category: "Utilities",
        });

        m.insert("superchip", FunctionMetadata {
            name: "superchip",
            description: "Chiptune synth - retro 8-bit game sounds with vibrato",
            params: vec![
                ParamMetadata {
                    name: "freq",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Base frequency in Hz",
                },
                ParamMetadata {
                    name: "vibrato_rate",
                    param_type: "Hz",
                    optional: true,
                    default: Some("5.0"),
                    description: "Vibrato rate in Hz",
                },
                ParamMetadata {
                    name: "vibrato_depth",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.1"),
                    description: "Vibrato depth (0-1)",
                },
            ],
            example: "~chip: superchip 440 :vibrato_rate 6 :vibrato_depth 0.2",
            category: "Synths",
        });

        m.insert("superfm", FunctionMetadata {
            name: "superfm",
            description: "FM synth - frequency modulation for complex timbres",
            params: vec![
                ParamMetadata {
                    name: "freq",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Carrier frequency in Hz",
                },
                ParamMetadata {
                    name: "mod_ratio",
                    param_type: "ratio",
                    optional: true,
                    default: Some("2.0"),
                    description: "Modulator frequency ratio (1.0-10.0)",
                },
                ParamMetadata {
                    name: "mod_index",
                    param_type: "float",
                    optional: true,
                    default: Some("1.0"),
                    description: "Modulation index/depth (0-10)",
                },
            ],
            example: "~fm: superfm 220 :mod_ratio 3.5 :mod_index 2.0",
            category: "Synths",
        });

        m.insert("superhat", FunctionMetadata {
            name: "superhat",
            description: "Hi-hat synth - metallic percussion sounds",
            params: vec![
                ParamMetadata {
                    name: "bright",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.5"),
                    description: "Brightness/tone (0 = dark, 1 = bright)",
                },
                ParamMetadata {
                    name: "sustain",
                    param_type: "seconds",
                    optional: true,
                    default: Some("0.1"),
                    description: "Decay time in seconds",
                },
            ],
            example: "~hat: superhat :bright 0.8 :sustain 0.05",
            category: "Synths",
        });

        m.insert("superkick", FunctionMetadata {
            name: "superkick",
            description: "Kick drum synth - punchy 808-style kick drums",
            params: vec![
                ParamMetadata {
                    name: "freq",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Base frequency in Hz (40-80 Hz typical)",
                },
                ParamMetadata {
                    name: "pitch_env",
                    param_type: "signal",
                    optional: true,
                    default: Some("auto"),
                    description: "Pitch envelope depth",
                },
                ParamMetadata {
                    name: "sustain",
                    param_type: "seconds",
                    optional: true,
                    default: Some("0.3"),
                    description: "Decay time in seconds",
                },
                ParamMetadata {
                    name: "noise_amt",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.1"),
                    description: "Click/noise amount (0-1)",
                },
            ],
            example: "~kick: superkick 55 :sustain 0.4",
            category: "Synths",
        });

        m.insert("superpwm", FunctionMetadata {
            name: "superpwm",
            description: "PWM synth - pulse width modulation for analog synth sounds",
            params: vec![
                ParamMetadata {
                    name: "freq",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Base frequency in Hz",
                },
                ParamMetadata {
                    name: "pwm_rate",
                    param_type: "Hz",
                    optional: true,
                    default: Some("0.5"),
                    description: "PWM LFO rate in Hz",
                },
                ParamMetadata {
                    name: "pwm_depth",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.3"),
                    description: "PWM depth (0-1)",
                },
            ],
            example: "~pwm: superpwm 110 :pwm_rate 0.3 :pwm_depth 0.5",
            category: "Synths",
        });

        m.insert("supersaw", FunctionMetadata {
            name: "supersaw",
            description: "Supersaw synth - detuned saw oscillators for thick sound",
            params: vec![
                ParamMetadata {
                    name: "freq",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Base frequency in Hz",
                },
                ParamMetadata {
                    name: "detune",
                    param_type: "cents",
                    optional: true,
                    default: Some("10.0"),
                    description: "Detune amount in cents",
                },
                ParamMetadata {
                    name: "voices",
                    param_type: "int",
                    optional: true,
                    default: Some("7"),
                    description: "Number of voices (3-9)",
                },
            ],
            example: "~saw: supersaw 110 :detune 15 :voices 7",
            category: "Synths",
        });

        m.insert("supersnare", FunctionMetadata {
            name: "supersnare",
            description: "Snare drum synth - snappy snare with noise and tone",
            params: vec![
                ParamMetadata {
                    name: "freq",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Tone frequency in Hz (150-250 Hz typical)",
                },
                ParamMetadata {
                    name: "snappy",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.5"),
                    description: "Snappiness/noise amount (0-1)",
                },
                ParamMetadata {
                    name: "sustain",
                    param_type: "seconds",
                    optional: true,
                    default: Some("0.15"),
                    description: "Decay time in seconds",
                },
            ],
            example: "~snare: supersnare 200 :snappy 0.7 :sustain 0.12",
            category: "Synths",
        });

        m.insert("svf_hp", FunctionMetadata {
            name: "svf_hp",
            description: "State variable highpass filter - smooth analog-style filter",
            params: vec![
                ParamMetadata {
                    name: "frequency",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Filter cutoff frequency in Hz",
                },
                ParamMetadata {
                    name: "resonance",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "Filter resonance (0-1)",
                },
            ],
            example: "~filtered: ~signal # svf_hp 500 0.5",
            category: "Filters",
        });

        m.insert("svf_lp", FunctionMetadata {
            name: "svf_lp",
            description: "State variable lowpass filter - smooth analog-style filter",
            params: vec![
                ParamMetadata {
                    name: "frequency",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Filter cutoff frequency in Hz",
                },
                ParamMetadata {
                    name: "resonance",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "Filter resonance (0-1)",
                },
            ],
            example: "~filtered: ~signal # svf_lp 1000 0.7",
            category: "Filters",
        });

        m.insert("vco", FunctionMetadata {
            name: "vco",
            description: "Voltage-controlled oscillator - multi-waveform oscillator",
            params: vec![
                ParamMetadata {
                    name: "frequency",
                    param_type: "Hz",
                    optional: false,
                    default: None,
                    description: "Oscillator frequency in Hz",
                },
                ParamMetadata {
                    name: "waveform",
                    param_type: "0-3",
                    optional: false,
                    default: None,
                    description: "Waveform (0=sine, 1=saw, 2=square, 3=triangle)",
                },
                ParamMetadata {
                    name: "pulse_width",
                    param_type: "0-1",
                    optional: true,
                    default: Some("0.5"),
                    description: "Pulse width for square wave (0-1)",
                },
            ],
            example: "~vco: vco 220 2 :pulse_width 0.3",
            category: "Oscillators",
        });

        m.insert("vocoder", FunctionMetadata {
            name: "vocoder",
            description: "Vocoder - voice/carrier modulation for robotic vocals",
            params: vec![
                ParamMetadata {
                    name: "modulator",
                    param_type: "signal",
                    optional: false,
                    default: None,
                    description: "Modulator signal (usually voice or rhythmic source)",
                },
                ParamMetadata {
                    name: "carrier",
                    param_type: "signal",
                    optional: false,
                    default: None,
                    description: "Carrier signal (usually synth with rich harmonics)",
                },
                ParamMetadata {
                    name: "num_bands",
                    param_type: "int",
                    optional: false,
                    default: None,
                    description: "Number of frequency bands (2-32, 8-16 typical)",
                },
            ],
            example: "~robot: vocoder ~voice (saw 110) 16",
            category: "Effects",
        });

        m.insert("white_noise", FunctionMetadata {
            name: "white_noise",
            description: "White noise generator - full spectrum noise",
            params: vec![],
            example: "~noise: white_noise * 0.1",
            category: "Generators",
        });

        m.insert("xfade", FunctionMetadata {
            name: "xfade",
            description: "Crossfade between two signals - smooth transition",
            params: vec![
                ParamMetadata {
                    name: "signal_a",
                    param_type: "signal",
                    optional: false,
                    default: None,
                    description: "First signal (position = 0)",
                },
                ParamMetadata {
                    name: "signal_b",
                    param_type: "signal",
                    optional: false,
                    default: None,
                    description: "Second signal (position = 1)",
                },
                ParamMetadata {
                    name: "position",
                    param_type: "0-1",
                    optional: false,
                    default: None,
                    description: "Crossfade position (0 = all A, 1 = all B)",
                },
            ],
            example: "~mix: xfade ~dry ~wet 0.5",
            category: "Utilities",
        });

        m.insert("xline", FunctionMetadata {
            name: "xline",
            description: "Exponential line generator - smooth exponential ramp",
            params: vec![
                ParamMetadata {
                    name: "start",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "Start value",
                },
                ParamMetadata {
                    name: "end",
                    param_type: "float",
                    optional: false,
                    default: None,
                    description: "End value",
                },
                ParamMetadata {
                    name: "duration",
                    param_type: "seconds",
                    optional: false,
                    default: None,
                    description: "Ramp duration in seconds",
                },
            ],
            example: "~sweep: xline 100 10000 2.0",
            category: "Utilities",
        });

        m
    };
}

/// Search functions by name or description
pub fn search_functions(query: &str) -> Vec<&'static FunctionMetadata> {
    let query_lower = query.to_lowercase();
    FUNCTION_METADATA
        .values()
        .filter(|f| {
            f.name.to_lowercase().contains(&query_lower)
                || f.description.to_lowercase().contains(&query_lower)
                || f.category.to_lowercase().contains(&query_lower)
        })
        .collect()
}

/// Get functions by category
pub fn functions_by_category(category: &str) -> Vec<&'static FunctionMetadata> {
    FUNCTION_METADATA
        .values()
        .filter(|f| f.category.eq_ignore_ascii_case(category))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lpf_metadata() {
        let lpf = FUNCTION_METADATA.get("lpf").unwrap();
        assert_eq!(lpf.name, "lpf");
        assert_eq!(lpf.params.len(), 2);
        assert_eq!(lpf.params[0].name, "cutoff");
        assert!(!lpf.params[0].optional);
        assert_eq!(lpf.params[1].name, "q");
        assert!(lpf.params[1].optional);
    }

    #[test]
    fn test_adsr_metadata() {
        let adsr = FUNCTION_METADATA.get("adsr").unwrap();
        assert_eq!(adsr.params.len(), 4);
        assert_eq!(adsr.params[2].name, "sustain");
        assert_eq!(adsr.params[2].default, Some("0.7"));
    }

    #[test]
    fn test_search_functions() {
        let results = search_functions("filter");
        assert!(!results.is_empty());
        assert!(results.iter().any(|f| f.name == "lpf"));
    }

    #[test]
    fn test_functions_by_category() {
        let filters = functions_by_category("Filters");
        assert!(filters.len() >= 4); // lpf, hpf, bpf, notch
    }

    #[test]
    fn test_param_signature() {
        let lpf = FUNCTION_METADATA.get("lpf").unwrap();
        let sig = lpf.param_signature();
        assert!(sig.contains("cutoff"));
        assert!(sig.contains("q"));
    }
}
