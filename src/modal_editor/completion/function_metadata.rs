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
