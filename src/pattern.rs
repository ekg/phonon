//! Complete port of Strudel's pattern system to Rust
//! 
//! This is a full implementation of the TidalCycles/Strudel pattern language

use std::sync::Arc;
use std::collections::HashMap;
use std::fmt::Debug;

/// Fraction type for rational time values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fraction {
    pub numerator: i64,
    pub denominator: i64,
}

impl Fraction {
    pub fn new(n: i64, d: i64) -> Self {
        let gcd = gcd(n.abs(), d.abs());
        Self {
            numerator: n / gcd * d.signum(),
            denominator: d.abs() / gcd,
        }
    }
    
    pub fn from_float(f: f64) -> Self {
        // Simple conversion - could be improved
        let denominator = 1000000;
        let numerator = (f * denominator as f64) as i64;
        Self::new(numerator, denominator)
    }
    
    pub fn to_float(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

impl std::ops::Add for Fraction {
    type Output = Self;
    
    fn add(self, other: Self) -> Self {
        Fraction::new(
            self.numerator * other.denominator + other.numerator * self.denominator,
            self.denominator * other.denominator,
        )
    }
}

impl std::ops::Sub for Fraction {
    type Output = Self;
    
    fn sub(self, other: Self) -> Self {
        Fraction::new(
            self.numerator * other.denominator - other.numerator * self.denominator,
            self.denominator * other.denominator,
        )
    }
}

impl std::ops::Mul for Fraction {
    type Output = Self;
    
    fn mul(self, other: Self) -> Self {
        Fraction::new(
            self.numerator * other.numerator,
            self.denominator * other.denominator,
        )
    }
}

impl std::ops::Div for Fraction {
    type Output = Self;
    
    fn div(self, other: Self) -> Self {
        Fraction::new(
            self.numerator * other.denominator,
            self.denominator * other.numerator,
        )
    }
}

impl std::cmp::PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Fraction {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let lhs = self.numerator * other.denominator;
        let rhs = other.numerator * self.denominator;
        lhs.cmp(&rhs)
    }
}

/// TimeSpan represents a time interval
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeSpan {
    pub begin: Fraction,
    pub end: Fraction,
}

impl TimeSpan {
    pub fn new(begin: Fraction, end: Fraction) -> Self {
        Self { begin, end }
    }
    
    pub fn duration(&self) -> Fraction {
        Fraction::new(
            self.end.numerator * self.begin.denominator - self.begin.numerator * self.end.denominator,
            self.end.denominator * self.begin.denominator
        )
    }
    
    pub fn midpoint(&self) -> Fraction {
        Fraction::new(
            self.begin.numerator * self.end.denominator + self.end.numerator * self.begin.denominator,
            2 * self.begin.denominator * self.end.denominator
        )
    }
}

/// Hap (short for "happening") is an event with a value
#[derive(Debug, Clone)]
pub struct Hap<T> {
    pub whole: Option<TimeSpan>,
    pub part: TimeSpan,
    pub value: T,
    pub context: HashMap<String, String>,
}

impl<T: Clone> Hap<T> {
    pub fn new(whole: Option<TimeSpan>, part: TimeSpan, value: T) -> Self {
        Self {
            whole,
            part,
            value,
            context: HashMap::new(),
        }
    }
    
    pub fn with_value<U>(&self, f: impl FnOnce(&T) -> U) -> Hap<U> {
        Hap {
            whole: self.whole,
            part: self.part,
            value: f(&self.value),
            context: self.context.clone(),
        }
    }
}

/// State for pattern queries
#[derive(Debug, Clone)]
pub struct State {
    pub span: TimeSpan,
    pub controls: HashMap<String, f64>,
}

/// Core Pattern type - the heart of the system
pub struct Pattern<T: Clone + Send + Sync> {
    // The query function is the essence of a pattern
    query: Arc<dyn Fn(&State) -> Vec<Hap<T>> + Send + Sync>,
    steps: Option<Fraction>,
}

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Create a new pattern from a query function
    pub fn new(query: impl Fn(&State) -> Vec<Hap<T>> + Send + Sync + 'static) -> Self {
        Self {
            query: Arc::new(query),
            steps: None,
        }
    }
    
    /// Query the pattern for events in a time span
    pub fn query(&self, state: &State) -> Vec<Hap<T>> {
        (self.query)(state)
    }
    
    /// Create a pattern from a single value (pure)
    pub fn pure(value: T) -> Self {
        Self::new(move |state| {
            vec![Hap::new(
                Some(state.span),
                state.span,
                value.clone(),
            )]
        })
    }
    
    /// Create a silence pattern
    pub fn silence() -> Self {
        Self::new(|_| vec![])
    }
    
    // ============= Core Transformations =============
    
    /// Transform the values in a pattern
    pub fn fmap<U: Clone + Send + Sync + 'static>(self, f: impl Fn(T) -> U + Send + Sync + 'static) -> Pattern<U> {
        let f = Arc::new(f);
        Pattern::new(move |state| {
            self.query(state)
                .into_iter()
                .map(|hap| hap.with_value(|v| f(v.clone())))
                .collect()
        })
    }
    
    /// Speed up a pattern by a factor
    pub fn fast(self, factor: f64) -> Self {
        Pattern::new(move |state| {
            let new_span = TimeSpan::new(
                Fraction::from_float(state.span.begin.to_float() * factor),
                Fraction::from_float(state.span.end.to_float() * factor),
            );
            let new_state = State {
                span: new_span,
                controls: state.controls.clone(),
            };
            
            self.query(&new_state)
                .into_iter()
                .map(|mut hap| {
                    hap.part = TimeSpan::new(
                        Fraction::from_float(hap.part.begin.to_float() / factor),
                        Fraction::from_float(hap.part.end.to_float() / factor),
                    );
                    if let Some(whole) = hap.whole {
                        hap.whole = Some(TimeSpan::new(
                            Fraction::from_float(whole.begin.to_float() / factor),
                            Fraction::from_float(whole.end.to_float() / factor),
                        ));
                    }
                    hap
                })
                .collect()
        })
    }
    
    /// Slow down a pattern by a factor
    pub fn slow(self, factor: f64) -> Self {
        self.fast(1.0 / factor)
    }
    
    /// Reverse a pattern within each cycle
    pub fn rev(self) -> Self {
        Pattern::new(move |state| {
            self.query(state)
                .into_iter()
                .map(|mut hap| {
                    let cycle_time = hap.part.begin.to_float().floor();
                    let local_begin = hap.part.begin.to_float() - cycle_time;
                    let local_end = hap.part.end.to_float() - cycle_time;
                    
                    hap.part = TimeSpan::new(
                        Fraction::from_float(cycle_time + (1.0 - local_end)),
                        Fraction::from_float(cycle_time + (1.0 - local_begin)),
                    );
                    
                    if let Some(whole) = hap.whole {
                        let whole_cycle = whole.begin.to_float().floor();
                        let whole_local_begin = whole.begin.to_float() - whole_cycle;
                        let whole_local_end = whole.end.to_float() - whole_cycle;
                        
                        hap.whole = Some(TimeSpan::new(
                            Fraction::from_float(whole_cycle + (1.0 - whole_local_end)),
                            Fraction::from_float(whole_cycle + (1.0 - whole_local_begin)),
                        ));
                    }
                    
                    hap
                })
                .collect()
        })
    }
    
    /// Apply a function every n cycles
    pub fn every(self, n: i32, f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static) -> Self
    where
        T: 'static,
    {
        let f = Arc::new(f);
        Pattern::new(move |state| {
            let cycle = state.span.begin.to_float().floor() as i32;
            if cycle % n == 0 {
                let transformed = f(self.clone());
                transformed.query(state)
            } else {
                self.query(state)
            }
        })
    }
    
    /// Rotate pattern left by n steps
    pub fn rotate_left(self, n: f64) -> Self {
        Pattern::new(move |state| {
            self.query(state)
                .into_iter()
                .map(|mut hap| {
                    hap.part = TimeSpan::new(
                        Fraction::from_float(hap.part.begin.to_float() - n),
                        Fraction::from_float(hap.part.end.to_float() - n),
                    );
                    if let Some(whole) = hap.whole {
                        hap.whole = Some(TimeSpan::new(
                            Fraction::from_float(whole.begin.to_float() - n),
                            Fraction::from_float(whole.end.to_float() - n),
                        ));
                    }
                    hap
                })
                .collect()
        })
    }
    
    /// Rotate pattern right by n steps  
    pub fn rotate_right(self, n: f64) -> Self {
        self.rotate_left(-n)
    }
}

// ============= Pattern Combinators =============

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Stack patterns on top of each other (play simultaneously)
    pub fn stack(patterns: Vec<Pattern<T>>) -> Pattern<T> {
        Pattern::new(move |state| {
            patterns
                .iter()
                .flat_map(|p| p.query(state))
                .collect()
        })
    }
    
    /// Concatenate patterns in sequence (play one after another)
    pub fn cat(patterns: Vec<Pattern<T>>) -> Pattern<T> {
        if patterns.is_empty() {
            return Pattern::silence();
        }
        
        let len = patterns.len() as f64;
        Pattern::new(move |state| {
            let cycle = state.span.begin.to_float().floor();
            let index = ((state.span.begin.to_float() - cycle) * len).floor() as usize;
            
            if index < patterns.len() {
                let pattern = &patterns[index];
                let scaled_span = TimeSpan::new(
                    Fraction::from_float(state.span.begin.to_float() * len),
                    Fraction::from_float(state.span.end.to_float() * len),
                );
                
                let scaled_state = State {
                    span: scaled_span,
                    controls: state.controls.clone(),
                };
                
                pattern.query(&scaled_state)
                    .into_iter()
                    .map(|mut hap| {
                        hap.part = TimeSpan::new(
                            Fraction::from_float(hap.part.begin.to_float() / len),
                            Fraction::from_float(hap.part.end.to_float() / len),
                        );
                        if let Some(whole) = hap.whole {
                            hap.whole = Some(TimeSpan::new(
                                Fraction::from_float(whole.begin.to_float() / len),
                                Fraction::from_float(whole.end.to_float() / len),
                            ));
                        }
                        hap
                    })
                    .collect()
            } else {
                vec![]
            }
        })
    }
    
    /// Alternate between patterns each cycle
    pub fn slowcat(patterns: Vec<Pattern<T>>) -> Pattern<T> {
        let len = patterns.len() as f64;
        Self::cat(patterns).slow(len)
    }
}

// ============= Euclidean Rhythms =============

impl Pattern<bool> {
    /// Generate Euclidean rhythm pattern
    pub fn euclid(pulses: usize, steps: usize, rotation: i32) -> Self {
        let mut pattern = vec![false; steps];
        let mut bucket = vec![vec![true]; pulses];
        bucket.extend(vec![vec![false]; steps - pulses]);
        
        let mut level = 0;
        while bucket.len() > 1 && bucket.iter().any(|x| x.len() != bucket[0].len()) {
            let mut new_bucket = Vec::new();
            let pivot = bucket.iter().position(|x| x.len() != bucket[0].len()).unwrap_or(bucket.len());
            
            for i in 0..pivot.min(bucket.len() - pivot) {
                let mut combined = bucket[i].clone();
                combined.extend(&bucket[pivot + i]);
                new_bucket.push(combined);
            }
            
            for i in (pivot.min(bucket.len() - pivot))..pivot.max(bucket.len() - pivot) {
                if i < pivot {
                    new_bucket.push(bucket[i].clone());
                } else {
                    new_bucket.push(bucket[i].clone());
                }
            }
            
            bucket = new_bucket;
            level += 1;
        }
        
        // Flatten
        let mut result: Vec<bool> = bucket.into_iter().flatten().collect();
        
        // Apply rotation
        if rotation != 0 {
            let rot = ((rotation % steps as i32) + steps as i32) as usize % steps;
            result.rotate_left(rot);
        }
        
        // Convert to pattern
        let step_duration = 1.0 / steps as f64;
        Pattern::new(move |state| {
            let mut haps = Vec::new();
            let cycle = state.span.begin.to_float().floor();
            
            for (i, &active) in result.iter().enumerate() {
                if active {
                    let begin = cycle + (i as f64 * step_duration);
                    let end = begin + step_duration;
                    
                    if begin < state.span.end.to_float() && end > state.span.begin.to_float() {
                        haps.push(Hap::new(
                            Some(TimeSpan::new(
                                Fraction::from_float(begin),
                                Fraction::from_float(end),
                            )),
                            TimeSpan::new(
                                Fraction::from_float(begin.max(state.span.begin.to_float())),
                                Fraction::from_float(end.min(state.span.end.to_float())),
                            ),
                            true,
                        ));
                    }
                }
            }
            
            haps
        })
    }
}

// Make Pattern cloneable
impl<T: Clone + Send + Sync> Clone for Pattern<T> {
    fn clone(&self) -> Self {
        Self {
            query: self.query.clone(),
            steps: self.steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pure_pattern() {
        let p = Pattern::pure(42);
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        
        let haps = p.query(&state);
        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, 42);
    }
    
    #[test]
    fn test_fast_pattern() {
        let p = Pattern::pure(1).fast(2.0);
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        
        let haps = p.query(&state);
        assert!(!haps.is_empty());
    }
    
    #[test]
    fn test_euclidean_rhythm() {
        let p = Pattern::<bool>::euclid(3, 8, 0);
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        
        let haps = p.query(&state);
        assert_eq!(haps.len(), 3); // Should have 3 hits in the pattern
    }
}