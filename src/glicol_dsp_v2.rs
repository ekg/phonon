//! Enhanced Glicol DSP with pattern parameters
//!
//! This version supports patterns as parameters: `lpf "1000 2000 500" 0.8`

use crate::dsp_parameter::{DspParameter, IntoParameter};
use std::collections::HashMap;

/// Enhanced DSP chain with pattern parameter support
#[derive(Clone, Debug)]
pub struct DspChain {
    pub nodes: Vec<DspNode>,
}

/// DSP node with pattern-aware parameters
#[derive(Clone, Debug)]
pub enum DspNode {
    // Oscillators with pattern parameters
    Sin {
        freq: DspParameter,
    },
    Saw {
        freq: DspParameter,
    },
    Square {
        freq: DspParameter,
        duty: DspParameter,
    },
    Triangle {
        freq: DspParameter,
    },
    Noise {
        seed: u64,
    },
    Impulse {
        freq: DspParameter,
    },
    Pink {
        seed: u64,
    },
    Brown {
        seed: u64,
    },

    // Math operations
    Mul {
        factor: DspParameter,
    },
    Add {
        value: DspParameter,
    },
    Div {
        divisor: DspParameter,
    },
    Sub {
        value: DspParameter,
    },

    // Filters with pattern parameters
    Lpf {
        cutoff: DspParameter,
        q: DspParameter,
    },
    Hpf {
        cutoff: DspParameter,
        q: DspParameter,
    },
    Bpf {
        center: DspParameter,
        q: DspParameter,
    },
    Notch {
        center: DspParameter,
        q: DspParameter,
    },

    // Effects with pattern parameters
    Delay {
        time: DspParameter,
        feedback: DspParameter,
        mix: DspParameter,
    },
    Reverb {
        room_size: DspParameter,
        damping: DspParameter,
        mix: DspParameter,
    },
    Chorus {
        rate: DspParameter,
        depth: DspParameter,
        mix: DspParameter,
    },
    Phaser {
        rate: DspParameter,
        depth: DspParameter,
        mix: DspParameter,
    },
    Distortion {
        gain: DspParameter,
    },
    Compressor {
        threshold: DspParameter,
        ratio: DspParameter,
    },
    Clip {
        min: DspParameter,
        max: DspParameter,
    },

    // Envelopes with pattern parameters
    Adsr {
        attack: DspParameter,
        decay: DspParameter,
        sustain: DspParameter,
        release: DspParameter,
    },
    Env {
        stages: Vec<(DspParameter, DspParameter)>,
    },

    // Modulators
    Lfo {
        freq: DspParameter,
        shape: LfoShape,
    },

    // Pattern integration
    Pattern {
        pattern: String,
        speed: DspParameter,
    },
    Seq {
        pattern: String,
    },
    Speed {
        factor: DspParameter,
    },
    S {
        pattern: String,
    },

    // Reference to another chain
    Ref {
        name: String,
    },

    // Value node
    Value(DspParameter),

    // Signal operations (for combining signals)
    SignalAdd {
        left: Box<DspChain>,
        right: Box<DspChain>,
    },
    SignalMul {
        left: Box<DspChain>,
        right: Box<DspChain>,
    },
    SignalSub {
        left: Box<DspChain>,
        right: Box<DspChain>,
    },
    SignalDiv {
        left: Box<DspChain>,
        right: Box<DspChain>,
    },

    // Sample playback
    Sp {
        sample: String,
    },

    // Utilities
    Mix {
        sources: Vec<DspChain>,
    },
    Pan {
        position: DspParameter,
    },
    Gain {
        amount: DspParameter,
    },
}

#[derive(Clone, Debug)]
pub enum LfoShape {
    Sine,
    Triangle,
    Square,
    Saw,
}

/// DSP environment holds all chains
#[derive(Clone, Debug)]
pub struct DspEnvironment {
    pub chains: HashMap<String, DspChain>,
    pub output: Option<DspChain>,
}

impl Default for DspChain {
    fn default() -> Self {
        Self::new()
    }
}

impl DspChain {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn from_node(node: DspNode) -> Self {
        let mut chain = Self::new();
        chain.nodes.push(node);
        chain
    }

    pub fn chain(mut self, node: DspNode) -> Self {
        self.nodes.push(node);
        self
    }
}

// Builder functions for the DSP chain
impl DspChain {
    /// Overload the >> operator for chaining
    pub fn then(self, other: DspNode) -> Self {
        self.chain(other)
    }
}

// Implement >> operator for chaining
impl std::ops::Shr<DspNode> for DspChain {
    type Output = DspChain;

    fn shr(self, rhs: DspNode) -> Self::Output {
        self.chain(rhs)
    }
}

impl std::ops::Shr<DspChain> for DspChain {
    type Output = DspChain;

    fn shr(mut self, mut rhs: DspChain) -> Self::Output {
        self.nodes.append(&mut rhs.nodes);
        self
    }
}

// Signal arithmetic operators
impl std::ops::Add for DspChain {
    type Output = DspChain;

    fn add(self, rhs: Self) -> Self::Output {
        DspChain::from_node(DspNode::SignalAdd {
            left: Box::new(self),
            right: Box::new(rhs),
        })
    }
}

impl std::ops::Mul for DspChain {
    type Output = DspChain;

    fn mul(self, rhs: Self) -> Self::Output {
        DspChain::from_node(DspNode::SignalMul {
            left: Box::new(self),
            right: Box::new(rhs),
        })
    }
}

impl std::ops::Sub for DspChain {
    type Output = DspChain;

    fn sub(self, rhs: Self) -> Self::Output {
        DspChain::from_node(DspNode::SignalSub {
            left: Box::new(self),
            right: Box::new(rhs),
        })
    }
}

impl std::ops::Div for DspChain {
    type Output = DspChain;

    fn div(self, rhs: Self) -> Self::Output {
        DspChain::from_node(DspNode::SignalDiv {
            left: Box::new(self),
            right: Box::new(rhs),
        })
    }
}

/// Builder functions for creating DSP nodes
pub mod dsp {
    use super::*;

    /// Create a sine oscillator with pattern frequency
    pub fn sin<P: IntoParameter>(freq: P) -> DspChain {
        DspChain::from_node(DspNode::Sin {
            freq: freq.into_parameter(),
        })
    }

    /// Create a saw oscillator with pattern frequency
    pub fn saw<P: IntoParameter>(freq: P) -> DspChain {
        DspChain::from_node(DspNode::Saw {
            freq: freq.into_parameter(),
        })
    }

    /// Create a square oscillator with pattern frequency and duty
    pub fn square<P1: IntoParameter, P2: IntoParameter>(freq: P1, duty: P2) -> DspChain {
        DspChain::from_node(DspNode::Square {
            freq: freq.into_parameter(),
            duty: duty.into_parameter(),
        })
    }

    /// Create a triangle oscillator with pattern frequency
    pub fn triangle<P: IntoParameter>(freq: P) -> DspChain {
        DspChain::from_node(DspNode::Triangle {
            freq: freq.into_parameter(),
        })
    }

    /// Create a noise generator
    pub fn noise() -> DspChain {
        DspChain::from_node(DspNode::Noise { seed: 42 })
    }

    /// Create an impulse generator with pattern frequency
    pub fn impulse<P: IntoParameter>(freq: P) -> DspChain {
        DspChain::from_node(DspNode::Impulse {
            freq: freq.into_parameter(),
        })
    }

    /// Multiply signal by pattern value
    pub fn mul<P: IntoParameter>(factor: P) -> DspChain {
        DspChain::from_node(DspNode::Mul {
            factor: factor.into_parameter(),
        })
    }

    /// Add pattern value to signal
    pub fn add<P: IntoParameter>(value: P) -> DspChain {
        DspChain::from_node(DspNode::Add {
            value: value.into_parameter(),
        })
    }

    /// Low-pass filter with pattern cutoff and Q
    pub fn lpf<P1: IntoParameter, P2: IntoParameter>(cutoff: P1, q: P2) -> DspChain {
        DspChain::from_node(DspNode::Lpf {
            cutoff: cutoff.into_parameter(),
            q: q.into_parameter(),
        })
    }

    /// High-pass filter with pattern cutoff and Q
    pub fn hpf<P1: IntoParameter, P2: IntoParameter>(cutoff: P1, q: P2) -> DspChain {
        DspChain::from_node(DspNode::Hpf {
            cutoff: cutoff.into_parameter(),
            q: q.into_parameter(),
        })
    }

    /// Band-pass filter with pattern center and Q
    pub fn bpf<P1: IntoParameter, P2: IntoParameter>(center: P1, q: P2) -> DspChain {
        DspChain::from_node(DspNode::Bpf {
            center: center.into_parameter(),
            q: q.into_parameter(),
        })
    }

    /// Notch filter with pattern center and Q
    pub fn notch<P1: IntoParameter, P2: IntoParameter>(center: P1, q: P2) -> DspChain {
        DspChain::from_node(DspNode::Notch {
            center: center.into_parameter(),
            q: q.into_parameter(),
        })
    }

    /// Delay effect with pattern time, feedback, and mix
    pub fn delay<P1: IntoParameter, P2: IntoParameter, P3: IntoParameter>(
        time: P1,
        feedback: P2,
        mix: P3,
    ) -> DspChain {
        DspChain::from_node(DspNode::Delay {
            time: time.into_parameter(),
            feedback: feedback.into_parameter(),
            mix: mix.into_parameter(),
        })
    }

    /// Reverb effect with pattern room size, damping, and mix
    pub fn reverb<P1: IntoParameter, P2: IntoParameter, P3: IntoParameter>(
        room_size: P1,
        damping: P2,
        mix: P3,
    ) -> DspChain {
        DspChain::from_node(DspNode::Reverb {
            room_size: room_size.into_parameter(),
            damping: damping.into_parameter(),
            mix: mix.into_parameter(),
        })
    }

    /// ADSR envelope with pattern parameters
    pub fn adsr<P1: IntoParameter, P2: IntoParameter, P3: IntoParameter, P4: IntoParameter>(
        attack: P1,
        decay: P2,
        sustain: P3,
        release: P4,
    ) -> DspChain {
        DspChain::from_node(DspNode::Adsr {
            attack: attack.into_parameter(),
            decay: decay.into_parameter(),
            sustain: sustain.into_parameter(),
            release: release.into_parameter(),
        })
    }

    /// LFO with pattern frequency
    pub fn lfo<P: IntoParameter>(freq: P, shape: LfoShape) -> DspChain {
        DspChain::from_node(DspNode::Lfo {
            freq: freq.into_parameter(),
            shape,
        })
    }

    /// Pattern source using the 's' function
    pub fn s(pattern: &str) -> DspChain {
        DspChain::from_node(DspNode::S {
            pattern: pattern.to_string(),
        })
    }

    /// Reference to another chain
    pub fn reference(name: &str) -> DspChain {
        DspChain::from_node(DspNode::Ref {
            name: name.to_string(),
        })
    }

    /// Sample playback
    pub fn sp(sample: &str) -> DspChain {
        DspChain::from_node(DspNode::Sp {
            sample: sample.to_string(),
        })
    }

    /// Gain control with pattern amount
    pub fn gain<P: IntoParameter>(amount: P) -> DspChain {
        DspChain::from_node(DspNode::Gain {
            amount: amount.into_parameter(),
        })
    }

    /// Pan control with pattern position
    pub fn pan<P: IntoParameter>(position: P) -> DspChain {
        DspChain::from_node(DspNode::Pan {
            position: position.into_parameter(),
        })
    }

    /// Distortion with pattern gain
    pub fn distortion<P: IntoParameter>(gain: P) -> DspChain {
        DspChain::from_node(DspNode::Distortion {
            gain: gain.into_parameter(),
        })
    }

    /// Compressor with pattern threshold and ratio
    pub fn compressor<P1: IntoParameter, P2: IntoParameter>(threshold: P1, ratio: P2) -> DspChain {
        DspChain::from_node(DspNode::Compressor {
            threshold: threshold.into_parameter(),
            ratio: ratio.into_parameter(),
        })
    }

    /// Clip signal with pattern min and max
    pub fn clip<P1: IntoParameter, P2: IntoParameter>(min: P1, max: P2) -> DspChain {
        DspChain::from_node(DspNode::Clip {
            min: min.into_parameter(),
            max: max.into_parameter(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::dsp::*;
    use super::*;

    #[test]
    fn test_pattern_parameters() {
        // Test that we can create nodes with pattern parameters
        let chain1 = sin("440 880 660") >> lpf("1000 2000 500", 0.8);
        assert_eq!(chain1.nodes.len(), 2);

        // Test with references
        let chain2 = saw("~freq") >> hpf("~cutoff", "~resonance");
        assert_eq!(chain2.nodes.len(), 2);

        // Test with constant values
        let chain3 = triangle(220.0) >> gain(0.5);
        assert_eq!(chain3.nodes.len(), 2);

        // Test arithmetic on chains
        let chain4 = sin(440.0) * sin(5.0); // Ring modulation
        assert_eq!(chain4.nodes.len(), 1);
        assert!(matches!(chain4.nodes[0], DspNode::SignalMul { .. }));
    }

    #[test]
    fn test_complex_pattern_chain() {
        // This should work with patterns everywhere
        let chain = saw("55 110 220")
            >> lpf("1000 2000 500 3000", "0.5 0.8 0.3")
            >> delay("0.25 0.125", 0.5, "0.3 0.5 0.7")
            >> gain("0.8 0.5 1.0");

        assert_eq!(chain.nodes.len(), 4);
    }

    #[test]
    fn test_s_function_pattern() {
        // Test the 's' function for pattern sources
        let chain = s("bd sn hh cp") >> lpf("500 1000 2000", 0.7);
        assert_eq!(chain.nodes.len(), 2);
    }
}
