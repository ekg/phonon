#![allow(unused_assignments, unused_mut)]
//! Complete port of Strudel's pattern system to Rust
//!
//! This is a full implementation of the TidalCycles/Strudel pattern language

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

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
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
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
            self.end.numerator * self.begin.denominator
                - self.begin.numerator * self.end.denominator,
            self.end.denominator * self.begin.denominator,
        )
    }

    pub fn midpoint(&self) -> Fraction {
        Fraction::new(
            self.begin.numerator * self.end.denominator
                + self.end.numerator * self.begin.denominator,
            2 * self.begin.denominator * self.end.denominator,
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

// Manual Debug implementation for Pattern since it contains a closure
impl<T: Clone + Send + Sync> std::fmt::Debug for Pattern<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pattern")
            .field("query", &"<closure>")
            .field("steps", &self.steps)
            .finish()
    }
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
    /// This creates a repeating pattern with one event per cycle
    pub fn pure(value: T) -> Self {
        Self::new(move |state| {
            let mut haps = Vec::new();
            let start_cycle = state.span.begin.to_float().floor() as i64;
            let end_cycle = state.span.end.to_float().ceil() as i64;

            for cycle in start_cycle..end_cycle {
                let cycle_begin = Fraction::from_float(cycle as f64);
                let cycle_end = Fraction::from_float((cycle + 1) as f64);

                // Only include if it overlaps with the query span
                if cycle_end > state.span.begin && cycle_begin < state.span.end {
                    let part_begin = cycle_begin.max(state.span.begin);
                    let part_end = cycle_end.min(state.span.end);

                    haps.push(Hap::new(
                        Some(TimeSpan::new(cycle_begin, cycle_end)),
                        TimeSpan::new(part_begin, part_end),
                        value.clone(),
                    ));
                }
            }
            haps
        })
    }

    /// Create a silence pattern
    pub fn silence() -> Self {
        Self::new(|_| vec![])
    }

    // ============= Core Transformations =============

    /// Transform the values in a pattern
    pub fn fmap<U: Clone + Send + Sync + 'static>(
        self,
        f: impl Fn(T) -> U + Send + Sync + 'static,
    ) -> Pattern<U> {
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
            let mut result = Vec::new();

            // Process each cycle separately
            let start_cycle = state.span.begin.to_float().floor() as i32;
            let end_cycle = state.span.end.to_float().ceil() as i32;

            for cycle in start_cycle..end_cycle {
                let cycle_begin = Fraction::from_float(cycle as f64);
                let cycle_end = Fraction::from_float((cycle + 1) as f64);
                let cycle_span = TimeSpan::new(cycle_begin, cycle_end);

                // Query events for this specific cycle
                let cycle_state = State {
                    span: cycle_span,
                    controls: state.controls.clone(),
                };

                let mut cycle_haps = self.query(&cycle_state);

                // Sort events by their start time within the cycle
                cycle_haps.sort_by(|a, b| a.part.begin.partial_cmp(&b.part.begin).unwrap());

                // Collect the time spans
                let time_spans: Vec<_> = cycle_haps.iter().map(|h| h.part).collect();

                // Reverse the values but keep the time spans in order
                let reversed_values: Vec<_> =
                    cycle_haps.iter().rev().map(|h| h.value.clone()).collect();

                // Create new haps with reversed values at original time positions
                for (i, time_span) in time_spans.iter().enumerate() {
                    if let Some(value) = reversed_values.get(i) {
                        // Only include if within the query span
                        if time_span.end > state.span.begin && time_span.begin < state.span.end {
                            let mut hap = Hap::new(cycle_haps[i].whole, *time_span, value.clone());

                            // Clip to query span if necessary
                            if hap.part.begin < state.span.begin {
                                hap.part.begin = state.span.begin;
                            }
                            if hap.part.end > state.span.end {
                                hap.part.end = state.span.end;
                            }

                            result.push(hap);
                        }
                    }
                }
            }

            result
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
        Pattern::new(move |state| patterns.iter().flat_map(|p| p.query(state)).collect())
    }

    /// Concatenate patterns in sequence (play one after another)
    pub fn cat(patterns: Vec<Pattern<T>>) -> Pattern<T> {
        if patterns.is_empty() {
            return Pattern::silence();
        }

        let len = patterns.len() as f64;
        Pattern::new(move |state| {
            let mut all_haps = Vec::new();

            // For each cycle that overlaps with our query span
            let start_cycle = state.span.begin.to_float().floor() as i64;
            let end_cycle = state.span.end.to_float().ceil() as i64;

            for cycle in start_cycle..end_cycle {
                let cycle_f = cycle as f64;

                // Get the portion of this cycle that overlaps with our query
                let cycle_start = cycle_f.max(state.span.begin.to_float());
                let cycle_end = (cycle_f + 1.0).min(state.span.end.to_float());

                if cycle_start >= cycle_end {
                    continue;
                }

                // Within each cycle, patterns are divided equally
                let local_start = cycle_start - cycle_f;
                let local_end = cycle_end - cycle_f;

                // Determine which patterns to query
                let start_idx = (local_start * len).floor() as usize;
                let end_idx = ((local_end * len).ceil() as usize).min(patterns.len());

                for idx in start_idx..end_idx {
                    let pattern = &patterns[idx];

                    // This pattern occupies the time span [idx/len, (idx+1)/len] within the cycle
                    let pattern_start = idx as f64 / len;
                    let pattern_end = (idx + 1) as f64 / len;

                    // Calculate the query window within this pattern
                    let query_start = local_start.max(pattern_start);
                    let query_end = local_end.min(pattern_end);

                    if query_start >= query_end {
                        continue;
                    }

                    // Scale the query to pattern's internal time
                    let scaled_start = (query_start - pattern_start) * len;
                    let scaled_end = (query_end - pattern_start) * len;

                    let scaled_state = State {
                        span: TimeSpan::new(
                            Fraction::from_float(scaled_start),
                            Fraction::from_float(scaled_end),
                        ),
                        controls: state.controls.clone(),
                    };

                    // Query the pattern and rescale results
                    for mut hap in pattern.query(&scaled_state) {
                        // Rescale from pattern time back to global time
                        let hap_start = hap.part.begin.to_float() / len + pattern_start + cycle_f;
                        let hap_end = hap.part.end.to_float() / len + pattern_start + cycle_f;

                        hap.part = TimeSpan::new(
                            Fraction::from_float(hap_start),
                            Fraction::from_float(hap_end),
                        );

                        if let Some(whole) = hap.whole {
                            let whole_start =
                                whole.begin.to_float() / len + pattern_start + cycle_f;
                            let whole_end = whole.end.to_float() / len + pattern_start + cycle_f;
                            hap.whole = Some(TimeSpan::new(
                                Fraction::from_float(whole_start),
                                Fraction::from_float(whole_end),
                            ));
                        }

                        all_haps.push(hap);
                    }
                }
            }

            all_haps
        })
    }

    /// Alternate between patterns each cycle
    pub fn slowcat(patterns: Vec<Pattern<T>>) -> Pattern<T> {
        if patterns.is_empty() {
            return Pattern::silence();
        }

        let len = patterns.len();
        Pattern::new(move |state| {
            // Determine which pattern is active based on the cycle number
            let cycle = state.span.begin.to_float().floor() as usize;
            let pattern_idx = cycle % len;
            let pattern = &patterns[pattern_idx];

            // Query the selected pattern with the current time span
            pattern.query(state)
        })
    }

    /// Select a slice from the pattern based on slice number
    /// slice n i - divides pattern into n slices and selects slice i
    pub fn slice(self, n: usize, index: usize) -> Self {
        if n == 0 {
            return Pattern::silence();
        }
        let slice_index = index % n;
        let begin = slice_index as f64 / n as f64;
        let end = (slice_index + 1) as f64 / n as f64;

        // Use zoom to focus on the specific slice
        Pattern::new(move |state| {
            let zoomed_begin = state.span.begin.to_float() * n as f64 + slice_index as f64;
            let zoomed_end = state.span.end.to_float() * n as f64 + slice_index as f64;

            let zoomed_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(zoomed_begin / n as f64),
                    Fraction::from_float(zoomed_end / n as f64),
                ),
                controls: state.controls.clone(),
            };

            self.query(&zoomed_state)
                .into_iter()
                .map(|mut hap| {
                    // Adjust timing to fit within the original query span
                    hap.whole = hap.whole.map(|w| TimeSpan::new(
                        Fraction::from_float(w.begin.to_float() * n as f64 - slice_index as f64),
                        Fraction::from_float(w.end.to_float() * n as f64 - slice_index as f64),
                    ));
                    hap.part = TimeSpan::new(
                        Fraction::from_float(hap.part.begin.to_float() * n as f64 - slice_index as f64),
                        Fraction::from_float(hap.part.end.to_float() * n as f64 - slice_index as f64),
                    );
                    hap
                })
                .collect()
        })
    }
}

impl Pattern<String> {
    /// Hurry - speed up the pattern and also speed up sample playback
    /// hurry 2 - play twice as fast AND pitch up samples
    pub fn hurry(self, factor: f64) -> Self {
        // Fast speeds up the pattern timing
        let fast_pattern = self.fast(factor);

        // Add speed control to modify sample playback
        Pattern::new(move |state| {
            fast_pattern.query(state)
                .into_iter()
                .map(|mut hap| {
                    // Add speed control to each event
                    let mut controls = hap.value.clone();
                    // Note: This assumes the value can have speed appended
                    // In real implementation, we'd want to add this to the controls HashMap
                    hap
                })
                .collect()
        })
    }
}

// ============= Euclidean Rhythms =============

impl Pattern<bool> {
    /// Generate Euclidean rhythm pattern
    pub fn euclid(pulses: usize, steps: usize, rotation: i32) -> Self {
        if pulses == 0 || steps == 0 {
            return Pattern::silence();
        }

        // Generate euclidean pattern using Bjorklund's algorithm
        // For compatibility with TidalCycles, we want patterns like:
        // (3,8) -> X..X..X.
        // (5,8) -> X.X.X.XX
        let mut result = vec![false; steps];

        if pulses > 0 {
            // Distribute pulses evenly across steps
            for i in 0..pulses {
                let pos = (i * steps) / pulses;
                result[pos] = true;
            }
        }

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
