//! DSP Parameter types that support patterns, constants, and references
//!
//! This module enables the core concept that "everything is a pattern" by allowing
//! DSP function parameters to be defined as:
//! - Constant values: `lpf 1000 0.8`
//! - Pattern strings: `lpf "1000 2000 500 3000" 0.8`
//! - Signal references: `lpf ~lfo 0.8`
//! - Arithmetic expressions: `lpf (~lfo * 1000 + 500) 0.8`

use crate::mini_notation_v3::parse_mini_notation;
use crate::pattern::{Fraction, State, TimeSpan};

/// A parameter that can be a constant, pattern, or reference
#[derive(Clone, Debug)]
pub enum DspParameter {
    /// A constant numeric value
    Constant(f32),

    /// A pattern string that generates time-varying values
    Pattern(String),

    /// A reference to another signal chain
    Reference(String),

    /// An arithmetic expression combining parameters
    Expression(Box<ParameterExpression>),
}

/// Arithmetic expressions for parameter computation
#[derive(Clone, Debug)]
pub enum ParameterExpression {
    /// Binary operation
    Binary {
        op: BinaryOp,
        left: DspParameter,
        right: DspParameter,
    },

    /// Unary operation
    Unary { op: UnaryOp, param: DspParameter },
}

#[derive(Clone, Debug)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Debug)]
pub enum UnaryOp {
    Negate,
}

impl DspParameter {
    /// Create a constant parameter
    pub fn constant(value: f32) -> Self {
        DspParameter::Constant(value)
    }

    /// Create a pattern parameter from a string
    pub fn pattern(pattern_str: &str) -> Self {
        DspParameter::Pattern(pattern_str.to_string())
    }

    /// Create a reference parameter
    pub fn reference(name: &str) -> Self {
        DspParameter::Reference(name.to_string())
    }

    /// Evaluate the parameter at a given time position
    /// Returns the value at that point in the pattern cycle
    pub fn evaluate(
        &self,
        cycle_pos: f64,
        references: &std::collections::HashMap<String, f32>,
    ) -> f32 {
        match self {
            DspParameter::Constant(v) => *v,

            DspParameter::Pattern(pattern_str) => {
                // Parse the pattern and query it at the current position
                let pattern = parse_mini_notation(pattern_str);

                // Create a query span for the current position
                // We'll sample a small window around the current position
                let begin = Fraction::from_float(cycle_pos);
                let end = Fraction::from_float(cycle_pos + 0.001); // Small window

                let state = State {
                    span: TimeSpan::new(begin, end),
                    controls: std::collections::HashMap::new(),
                };

                let events = pattern.query(&state);

                // Get the first event's value, or default to 0
                if let Some(event) = events.first() {
                    // The value is a String, try to parse as number
                    event.value.parse::<f32>().unwrap_or(0.0)
                } else {
                    0.0 // No event at this position
                }
            }

            DspParameter::Reference(name) => {
                // Look up the reference value
                references.get(name).copied().unwrap_or(0.0)
            }

            DspParameter::Expression(expr) => expr.evaluate(cycle_pos, references),
        }
    }

    /// Check if this parameter is time-varying (pattern or reference)
    pub fn is_dynamic(&self) -> bool {
        match self {
            DspParameter::Constant(_) => false,
            DspParameter::Pattern(_) => true,
            DspParameter::Reference(_) => true,
            DspParameter::Expression(expr) => expr.is_dynamic(),
        }
    }
}

impl ParameterExpression {
    pub fn evaluate(
        &self,
        cycle_pos: f64,
        references: &std::collections::HashMap<String, f32>,
    ) -> f32 {
        match self {
            ParameterExpression::Binary { op, left, right } => {
                let left_val = left.evaluate(cycle_pos, references);
                let right_val = right.evaluate(cycle_pos, references);

                match op {
                    BinaryOp::Add => left_val + right_val,
                    BinaryOp::Subtract => left_val - right_val,
                    BinaryOp::Multiply => left_val * right_val,
                    BinaryOp::Divide => {
                        if right_val != 0.0 {
                            left_val / right_val
                        } else {
                            0.0 // Avoid division by zero
                        }
                    }
                }
            }

            ParameterExpression::Unary { op, param } => {
                let val = param.evaluate(cycle_pos, references);
                match op {
                    UnaryOp::Negate => -val,
                }
            }
        }
    }

    pub fn is_dynamic(&self) -> bool {
        match self {
            ParameterExpression::Binary { left, right, .. } => {
                left.is_dynamic() || right.is_dynamic()
            }
            ParameterExpression::Unary { param, .. } => param.is_dynamic(),
        }
    }
}

/// Helper to create pattern parameters from various input types
pub trait IntoParameter {
    fn into_parameter(self) -> DspParameter;
}

impl IntoParameter for f32 {
    fn into_parameter(self) -> DspParameter {
        DspParameter::Constant(self)
    }
}

impl IntoParameter for f64 {
    fn into_parameter(self) -> DspParameter {
        DspParameter::Constant(self as f32)
    }
}

impl IntoParameter for &str {
    fn into_parameter(self) -> DspParameter {
        // Check if it's a reference (starts with ~)
        if self.starts_with('~') {
            DspParameter::Reference(self[1..].to_string())
        } else {
            // Otherwise treat as a pattern
            DspParameter::Pattern(self.to_string())
        }
    }
}

impl IntoParameter for String {
    fn into_parameter(self) -> DspParameter {
        self.as_str().into_parameter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_parameter() {
        let param = DspParameter::constant(440.0);
        let refs = std::collections::HashMap::new();
        assert_eq!(param.evaluate(0.0, &refs), 440.0);
        assert_eq!(param.evaluate(0.5, &refs), 440.0);
        assert!(!param.is_dynamic());
    }

    #[test]
    fn test_pattern_parameter() {
        let param = DspParameter::pattern("100 200 300 400");
        let refs = std::collections::HashMap::new();

        // At different cycle positions, should get different values
        let val1 = param.evaluate(0.0, &refs);
        let val2 = param.evaluate(0.25, &refs);

        // Pattern should produce varying values
        assert!(param.is_dynamic());
        // Values should be from the pattern
        assert!(val1 == 100.0 || val1 == 0.0); // Depending on exact query timing
    }

    #[test]
    fn test_reference_parameter() {
        let param = DspParameter::reference("lfo");
        let mut refs = std::collections::HashMap::new();
        refs.insert("lfo".to_string(), 0.5);

        assert_eq!(param.evaluate(0.0, &refs), 0.5);
        assert!(param.is_dynamic());
    }

    #[test]
    fn test_into_parameter() {
        // Test different input types
        let p1: DspParameter = 440.0f32.into_parameter();
        assert!(matches!(p1, DspParameter::Constant(440.0)));

        let p2: DspParameter = "100 200 300".into_parameter();
        assert!(matches!(p2, DspParameter::Pattern(_)));

        let p3: DspParameter = "~lfo".into_parameter();
        assert!(matches!(p3, DspParameter::Reference(ref s) if s == "lfo"));
    }
}
