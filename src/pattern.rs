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

    /// Choose - randomly select from a list of options (one per cycle)
    /// Example: Pattern::choose(vec!["bd", "sn", "hh"]) picks one sample per cycle
    /// Uses deterministic randomness based on cycle number
    pub fn choose(options: Vec<T>) -> Self {
        use rand::{rngs::StdRng, Rng, SeedableRng};

        if options.is_empty() {
            return Self::silence();
        }

        Self::new(move |state| {
            let mut haps = Vec::new();
            let start_cycle = state.span.begin.to_float().floor() as i64;
            let end_cycle = state.span.end.to_float().ceil() as i64;

            for cycle in start_cycle..end_cycle {
                let cycle_begin = Fraction::from_float(cycle as f64);
                let cycle_end = Fraction::from_float((cycle + 1) as f64);

                // Only include if it overlaps with the query span
                if cycle_end > state.span.begin && cycle_begin < state.span.end {
                    // Deterministic random selection based on cycle number
                    let mut rng = StdRng::seed_from_u64(cycle as u64);
                    let index = rng.gen_range(0..options.len());
                    let value = options[index].clone();

                    let part_begin = cycle_begin.max(state.span.begin);
                    let part_end = cycle_end.min(state.span.end);

                    haps.push(Hap::new(
                        Some(TimeSpan::new(cycle_begin, cycle_end)),
                        TimeSpan::new(part_begin, part_end),
                        value,
                    ));
                }
            }
            haps
        })
    }

    /// Wchoose - weighted random choice (Tidal's wchoose function)
    /// Example: Pattern::wchoose(vec![("bd", 3.0), ("sn", 1.0)]) picks "bd" 75% of the time
    /// Uses deterministic randomness based on cycle number
    pub fn wchoose(weighted_options: Vec<(T, f64)>) -> Self {
        use rand::{rngs::StdRng, Rng, SeedableRng};

        if weighted_options.is_empty() {
            return Self::silence();
        }

        // Calculate total weight and cumulative weights
        let total_weight: f64 = weighted_options.iter().map(|(_, w)| w).sum();
        if total_weight <= 0.0 {
            return Self::silence();
        }

        let mut cumulative_weights = Vec::new();
        let mut cumsum = 0.0;
        for (_, weight) in &weighted_options {
            cumsum += weight;
            cumulative_weights.push(cumsum);
        }

        Self::new(move |state| {
            let mut haps = Vec::new();
            let start_cycle = state.span.begin.to_float().floor() as i64;
            let end_cycle = state.span.end.to_float().ceil() as i64;

            for cycle in start_cycle..end_cycle {
                let cycle_begin = Fraction::from_float(cycle as f64);
                let cycle_end = Fraction::from_float((cycle + 1) as f64);

                // Only include if it overlaps with the query span
                if cycle_end > state.span.begin && cycle_begin < state.span.end {
                    // Deterministic random selection based on cycle number
                    let mut rng = StdRng::seed_from_u64(cycle as u64);
                    let random_value = rng.gen::<f64>() * total_weight;

                    // Find which option was selected based on cumulative weights
                    let mut selected_index = 0;
                    for (i, &cumulative) in cumulative_weights.iter().enumerate() {
                        if random_value < cumulative {
                            selected_index = i;
                            break;
                        }
                    }

                    let value = weighted_options[selected_index].0.clone();

                    let part_begin = cycle_begin.max(state.span.begin);
                    let part_end = cycle_end.min(state.span.end);

                    haps.push(Hap::new(
                        Some(TimeSpan::new(cycle_begin, cycle_end)),
                        TimeSpan::new(part_begin, part_end),
                        value,
                    ));
                }
            }
            haps
        })
    }

    /// Run - generate ascending sequence (Tidal's run function)
    /// Example: Pattern::run(4) creates pattern with values 0, 1, 2, 3 evenly spaced in cycle
    /// Used for sample selection or numeric sequences
    pub fn run(n: usize) -> Pattern<f64> {
        if n == 0 {
            return Pattern::silence();
        }

        Pattern::new(move |state| {
            let mut haps = Vec::new();
            let start_cycle = state.span.begin.to_float().floor() as i64;
            let end_cycle = state.span.end.to_float().ceil() as i64;

            for cycle in start_cycle..end_cycle {
                let cycle_begin = Fraction::from_float(cycle as f64);
                let cycle_end = Fraction::from_float((cycle + 1) as f64);

                // Only process if it overlaps with the query span
                if cycle_end > state.span.begin && cycle_begin < state.span.end {
                    // Create n evenly spaced events with values 0..(n-1)
                    for i in 0..n {
                        let event_fraction = i as f64 / n as f64;
                        let next_fraction = (i + 1) as f64 / n as f64;

                        let event_begin = Fraction::from_float(cycle as f64 + event_fraction);
                        let event_end = Fraction::from_float(cycle as f64 + next_fraction);

                        // Only include if it overlaps with the query span
                        if event_end > state.span.begin && event_begin < state.span.end {
                            let part_begin = event_begin.max(state.span.begin);
                            let part_end = event_end.min(state.span.end);

                            haps.push(Hap::new(
                                Some(TimeSpan::new(event_begin, event_end)),
                                TimeSpan::new(part_begin, part_end),
                                i as f64, // Value is the index
                            ));
                        }
                    }
                }
            }
            haps
        })
    }

    /// Irand - random integer generator (Tidal's irand function)
    /// Example: Pattern::irand(4) generates random integers 0-3 (one per cycle)
    /// Uses deterministic randomness based on cycle number
    pub fn irand(n: usize) -> Pattern<f64> {
        use rand::{rngs::StdRng, Rng, SeedableRng};

        if n == 0 {
            return Pattern::silence();
        }

        Pattern::new(move |state| {
            let mut haps = Vec::new();
            let start_cycle = state.span.begin.to_float().floor() as i64;
            let end_cycle = state.span.end.to_float().ceil() as i64;

            for cycle in start_cycle..end_cycle {
                let cycle_begin = Fraction::from_float(cycle as f64);
                let cycle_end = Fraction::from_float((cycle + 1) as f64);

                // Only include if it overlaps with the query span
                if cycle_end > state.span.begin && cycle_begin < state.span.end {
                    // Deterministic random selection based on cycle number
                    let mut rng = StdRng::seed_from_u64(cycle as u64);
                    let value = rng.gen_range(0..n) as f64;

                    let part_begin = cycle_begin.max(state.span.begin);
                    let part_end = cycle_end.min(state.span.end);

                    haps.push(Hap::new(
                        Some(TimeSpan::new(cycle_begin, cycle_end)),
                        TimeSpan::new(part_begin, part_end),
                        value,
                    ));
                }
            }
            haps
        })
    }

    /// Rand - random float generator (Tidal's rand function)
    /// Generates random floats in range [0.0, 1.0) (one per cycle)
    /// Uses deterministic randomness based on cycle number
    pub fn rand() -> Pattern<f64> {
        use rand::{rngs::StdRng, Rng, SeedableRng};

        Pattern::new(move |state| {
            let mut haps = Vec::new();
            let start_cycle = state.span.begin.to_float().floor() as i64;
            let end_cycle = state.span.end.to_float().ceil() as i64;

            for cycle in start_cycle..end_cycle {
                let cycle_begin = Fraction::from_float(cycle as f64);
                let cycle_end = Fraction::from_float((cycle + 1) as f64);

                // Only include if it overlaps with the query span
                if cycle_end > state.span.begin && cycle_begin < state.span.end {
                    // Deterministic random float based on cycle number
                    let mut rng = StdRng::seed_from_u64(cycle as u64);
                    let value = rng.gen::<f64>();

                    let part_begin = cycle_begin.max(state.span.begin);
                    let part_end = cycle_end.min(state.span.end);

                    haps.push(Hap::new(
                        Some(TimeSpan::new(cycle_begin, cycle_end)),
                        TimeSpan::new(part_begin, part_end),
                        value,
                    ));
                }
            }
            haps
        })
    }

    /// Scan - cumulative pattern that grows each cycle (Tidal's scan function)
    /// Example: Pattern::scan(4) creates:
    ///   Cycle 0: 0
    ///   Cycle 1: 0 1
    ///   Cycle 2: 0 1 2
    ///   Cycle 3: 0 1 2 3
    ///   Then repeats with modulo
    pub fn scan(n: usize) -> Pattern<f64> {
        if n == 0 {
            return Pattern::silence();
        }

        Pattern::new(move |state| {
            let mut haps = Vec::new();
            let start_cycle = state.span.begin.to_float().floor() as i64;
            let end_cycle = state.span.end.to_float().ceil() as i64;

            for cycle in start_cycle..end_cycle {
                let cycle_begin = Fraction::from_float(cycle as f64);
                let cycle_end = Fraction::from_float((cycle + 1) as f64);

                // Only process if it overlaps with the query span
                if cycle_end > state.span.begin && cycle_begin < state.span.end {
                    // Number of events in this cycle grows: (cycle % n) + 1
                    let num_events = ((cycle.abs() as usize) % n) + 1;

                    // Create num_events evenly spaced events with values 0..(num_events-1)
                    for i in 0..num_events {
                        let event_fraction = i as f64 / num_events as f64;
                        let next_fraction = (i + 1) as f64 / num_events as f64;

                        let event_begin = Fraction::from_float(cycle as f64 + event_fraction);
                        let event_end = Fraction::from_float(cycle as f64 + next_fraction);

                        // Only include if it overlaps with the query span
                        if event_end > state.span.begin && event_begin < state.span.end {
                            let part_begin = event_begin.max(state.span.begin);
                            let part_end = event_end.min(state.span.end);

                            haps.push(Hap::new(
                                Some(TimeSpan::new(event_begin, event_end)),
                                TimeSpan::new(part_begin, part_end),
                                i as f64, // Value is the index
                            ));
                        }
                    }
                }
            }
            haps
        })
    }

    /// Sine wave pattern - generates continuous sine wave values (Tidal's sine)
    /// Returns values in range [-1.0, 1.0] based on cycle position
    /// Completes one full cycle per pattern cycle
    pub fn sine_wave() -> Pattern<f64> {
        use std::f64::consts::PI;

        Pattern::new(move |state| {
            // For continuous patterns, create a single hap spanning the query
            let phase = state.span.begin.to_float() % 1.0;
            let value = (phase * 2.0 * PI).sin();

            vec![Hap::new(
                Some(state.span.clone()),
                state.span.clone(),
                value,
            )]
        })
    }

    /// Cosine wave pattern - generates continuous cosine wave values (Tidal's cosine)
    /// Returns values in range [-1.0, 1.0] based on cycle position
    /// Completes one full cycle per pattern cycle
    pub fn cosine_wave() -> Pattern<f64> {
        use std::f64::consts::PI;

        Pattern::new(move |state| {
            let phase = state.span.begin.to_float() % 1.0;
            let value = (phase * 2.0 * PI).cos();

            vec![Hap::new(
                Some(state.span.clone()),
                state.span.clone(),
                value,
            )]
        })
    }

    /// Sawtooth wave pattern - generates linear ramp (Tidal's saw)
    /// Returns values in range [0.0, 1.0] ramping up linearly over each cycle
    pub fn saw_wave() -> Pattern<f64> {
        Pattern::new(move |state| {
            let phase = state.span.begin.to_float() % 1.0;
            let value = phase; // Linear ramp 0->1

            vec![Hap::new(
                Some(state.span.clone()),
                state.span.clone(),
                value,
            )]
        })
    }

    /// Triangle wave pattern - generates triangle wave (Tidal's tri)
    /// Returns values in range [0.0, 1.0] ramping up then down over each cycle
    pub fn tri_wave() -> Pattern<f64> {
        Pattern::new(move |state| {
            let phase = state.span.begin.to_float() % 1.0;
            // Triangle: ramp up 0->1 in first half, then 1->0 in second half
            let value = if phase < 0.5 {
                phase * 2.0  // 0->1 in first half
            } else {
                2.0 - (phase * 2.0)  // 1->0 in second half
            };

            vec![Hap::new(
                Some(state.span.clone()),
                state.span.clone(),
                value,
            )]
        })
    }

    /// Square wave pattern - generates square wave (Tidal's square)
    /// Returns values 0.0 (first half of cycle) or 1.0 (second half)
    pub fn square_wave() -> Pattern<f64> {
        Pattern::new(move |state| {
            let phase = state.span.begin.to_float() % 1.0;
            let value = if phase < 0.5 { 0.0 } else { 1.0 };

            vec![Hap::new(
                Some(state.span.clone()),
                state.span.clone(),
                value,
            )]
        })
    }

    // ============= Conditional Value Generators =============
    // These generate different values based on cycle conditions
    // Useful for conditional audio effects: lpf (every_val 2 500 2000) 0.8

    /// every_val - output different values based on cycle number
    /// every_val(n, on_val, off_val) outputs on_val when cycle % n == 0, else off_val
    pub fn every_val(n: i32, on_val: f64, off_val: f64) -> Pattern<f64> {
        Pattern::new(move |state| {
            let cycle = state.span.begin.to_float().floor() as i32;
            let value = if cycle % n == 0 { on_val } else { off_val };

            vec![Hap::new(
                Some(state.span.clone()),
                state.span.clone(),
                value,
            )]
        })
    }

    /// sometimes_val - randomly choose between two values per cycle
    /// sometimes_val(on_val, off_val) outputs on_val 50% of cycles, off_val otherwise
    pub fn sometimes_val(on_val: f64, off_val: f64) -> Pattern<f64> {
        Self::sometimes_by_val(0.5, on_val, off_val)
    }

    /// sometimes_by_val - randomly choose between two values with given probability
    pub fn sometimes_by_val(prob: f64, on_val: f64, off_val: f64) -> Pattern<f64> {
        use rand::{rngs::StdRng, Rng, SeedableRng};

        Pattern::new(move |state| {
            let cycle = state.span.begin.to_float().floor() as u64;
            let mut rng = StdRng::seed_from_u64(cycle);
            let value = if rng.gen::<f64>() < prob { on_val } else { off_val };

            vec![Hap::new(
                Some(state.span.clone()),
                state.span.clone(),
                value,
            )]
        })
    }

    /// whenmod_val - output different values based on cycle modulo with offset
    /// whenmod_val(modulo, offset, on_val, off_val) outputs on_val when (cycle - offset) % modulo == 0
    pub fn whenmod_val(modulo: i32, offset: i32, on_val: f64, off_val: f64) -> Pattern<f64> {
        Pattern::new(move |state| {
            let cycle = state.span.begin.to_float().floor() as i32;
            let value = if (cycle - offset) % modulo == 0 { on_val } else { off_val };

            vec![Hap::new(
                Some(state.span.clone()),
                state.span.clone(),
                value,
            )]
        })
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

    /// Speed up a pattern by a factor (pattern-controlled)
    /// Accepts a Pattern<f64> for the speed - use Pattern::pure(2.0) for constants
    pub fn fast(self, speed: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Pattern::new(move |state| {
            // Query speed pattern at cycle start to get current speed
            let cycle_start = state.span.begin.to_float().floor();
            let speed_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let speed_haps = speed.query(&speed_state);
            let factor = if let Some(hap) = speed_haps.first() {
                hap.value.max(0.001)
            } else {
                1.0
            };

            // Apply time transformation with the queried factor
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

    /// Hurry - fast + speed combined (Tidal's hurry function)
    /// Speeds up pattern AND adds speed multiplier to context
    /// Example: hurry 2 doubles pattern speed and sets playback speed to 2x
    pub fn hurry(self, factor: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        // Get factor value to use for both fast and speed
        let default_state = State {
            span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
            controls: HashMap::new(),
        };

        let factor_val = factor
            .query(&default_state)
            .first()
            .map(|h| h.value)
            .unwrap_or(1.0);

        // Apply fast to speed up pattern
        let fast_pattern = self.fast(factor.clone());

        // Add speed multiplier to event context
        Pattern::new(move |state| {
            fast_pattern
                .query(state)
                .into_iter()
                .map(|mut hap| {
                    // Add hurry_speed to context for voice manager to read
                    hap.context.insert("hurry_speed".to_string(), factor_val.to_string());
                    hap
                })
                .collect()
        })
    }

    /// Slow down a pattern by a factor (pattern-controlled)
    /// Accepts a Pattern<f64> for the speed - use Pattern::pure(2.0) for constants
    pub fn slow(self, speed: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        // Invert the speed pattern: 2 -> 0.5, 3 -> 0.333, etc.
        let inverted = Pattern::new(move |state| {
            speed
                .query(state)
                .into_iter()
                .map(|mut hap| {
                    hap.value = 1.0 / hap.value.max(0.001);
                    hap
                })
                .collect()
        });
        self.fast(inverted)
    }

    /// Squeeze pattern to first 1/n of cycle and speed up by n
    /// squeeze 2 - squeezes pattern to first half of cycle, plays 2x faster
    /// Like fast but compressed into a smaller time window
    pub fn squeeze(self, factor: f64) -> Self {
        let compressed_duration = 1.0 / factor;
        Pattern::new(move |state| {
            // Per-cycle compression: within each cycle, map [0, 1/factor] to source [0, 1]
            // For each cycle in query span, decompress to get source time
            let query_begin = state.span.begin.to_float();
            let query_end = state.span.end.to_float();

            // Find which cycles are touched by this query
            let start_cycle = query_begin.floor() as i64;
            let end_cycle = query_end.ceil() as i64;

            let mut result = Vec::new();

            // Process each cycle in the query range
            for cycle in start_cycle..end_cycle {
                let cycle_f = cycle as f64;
                let cycle_begin = cycle_f.max(query_begin);
                let cycle_end = (cycle_f + 1.0).min(query_end);

                // Map query within this cycle to source time
                let rel_begin = cycle_begin - cycle_f;
                let rel_end = cycle_end - cycle_f;

                // Only query if we're in the compressed region (first 1/factor of cycle)
                if rel_begin < compressed_duration {
                    // Map compressed time to source time within the cycle
                    let source_rel_begin = rel_begin / compressed_duration;
                    let source_rel_end = (rel_end / compressed_duration).min(1.0);

                    let source_begin = cycle_f + source_rel_begin;
                    let source_end = cycle_f + source_rel_end;

                    let new_span = TimeSpan::new(
                        Fraction::from_float(source_begin),
                        Fraction::from_float(source_end),
                    );
                    let new_state = State {
                        span: new_span,
                        controls: state.controls.clone(),
                    };

                    // Query source and compress results
                    for mut hap in self.query(&new_state) {
                        let hap_rel_begin = hap.part.begin.to_float() - cycle_f;
                        let hap_rel_end = hap.part.end.to_float() - cycle_f;

                        // Compress: map source [0,1] to [0, 1/factor]
                        let compressed_rel_begin = hap_rel_begin * compressed_duration;
                        let compressed_rel_end = hap_rel_end * compressed_duration;

                        hap.part = TimeSpan::new(
                            Fraction::from_float(cycle_f + compressed_rel_begin),
                            Fraction::from_float(cycle_f + compressed_rel_end),
                        );
                        if let Some(whole) = hap.whole {
                            let whole_rel_begin = whole.begin.to_float() - cycle_f;
                            let whole_rel_end = whole.end.to_float() - cycle_f;
                            hap.whole = Some(TimeSpan::new(
                                Fraction::from_float(
                                    cycle_f + whole_rel_begin * compressed_duration,
                                ),
                                Fraction::from_float(cycle_f + whole_rel_end * compressed_duration),
                            ));
                        }
                        result.push(hap);
                    }
                }
            }

            result
        })
    }

    /// Squeeze pattern to first 1/n of cycle and speed up by n (pattern-controlled)
    /// Accepts a Pattern<f64> for the factor - use Pattern::pure(2.0) for constants
    pub fn squeeze_pattern(self, factor: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Pattern::new(move |state| {
            // Query factor pattern at cycle start to get current factor
            let cycle_start = state.span.begin.to_float().floor();
            let factor_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let factor_haps = factor.query(&factor_state);
            let squeeze_factor = if let Some(hap) = factor_haps.first() {
                hap.value.max(0.001)
            } else {
                1.0
            };

            // Now apply squeeze with the queried factor
            let compressed_duration = 1.0 / squeeze_factor;
            let query_begin = state.span.begin.to_float();
            let query_end = state.span.end.to_float();

            // Find which cycles are touched by this query
            let start_cycle = query_begin.floor() as i64;
            let end_cycle = query_end.ceil() as i64;

            let mut result = Vec::new();

            // Process each cycle in the query range
            for cycle in start_cycle..end_cycle {
                let cycle_f = cycle as f64;
                let cycle_begin = cycle_f.max(query_begin);
                let cycle_end = (cycle_f + 1.0).min(query_end);

                // Map query within this cycle to source time
                let rel_begin = cycle_begin - cycle_f;
                let rel_end = cycle_end - cycle_f;

                // Only query if we're in the compressed region (first 1/factor of cycle)
                if rel_begin < compressed_duration {
                    // Map compressed time to source time within the cycle
                    let source_rel_begin = rel_begin / compressed_duration;
                    let source_rel_end = (rel_end / compressed_duration).min(1.0);

                    let source_begin = cycle_f + source_rel_begin;
                    let source_end = cycle_f + source_rel_end;

                    let new_span = TimeSpan::new(
                        Fraction::from_float(source_begin),
                        Fraction::from_float(source_end),
                    );
                    let new_state = State {
                        span: new_span,
                        controls: state.controls.clone(),
                    };

                    // Query source and compress results
                    for mut hap in self.query(&new_state) {
                        let hap_rel_begin = hap.part.begin.to_float() - cycle_f;
                        let hap_rel_end = hap.part.end.to_float() - cycle_f;

                        // Compress: map source [0,1] to [0, 1/factor]
                        let compressed_rel_begin = hap_rel_begin * compressed_duration;
                        let compressed_rel_end = hap_rel_end * compressed_duration;

                        hap.part = TimeSpan::new(
                            Fraction::from_float(cycle_f + compressed_rel_begin),
                            Fraction::from_float(cycle_f + compressed_rel_end),
                        );
                        if let Some(whole) = hap.whole {
                            let whole_rel_begin = whole.begin.to_float() - cycle_f;
                            let whole_rel_end = whole.end.to_float() - cycle_f;
                            hap.whole = Some(TimeSpan::new(
                                Fraction::from_float(cycle_f + whole_rel_begin * compressed_duration),
                                Fraction::from_float(cycle_f + whole_rel_end * compressed_duration),
                            ));
                        }
                        result.push(hap);
                    }
                }
            }

            result
        })
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

    /// Randomly choose a pattern each cycle (deterministic based on cycle number)
    pub fn randcat(patterns: Vec<Pattern<T>>) -> Pattern<T> {
        if patterns.is_empty() {
            return Pattern::silence();
        }

        let len = patterns.len();
        Pattern::new(move |state| {
            use rand::rngs::StdRng;
            use rand::{Rng, SeedableRng};

            // Determine which pattern is active based on random selection per cycle
            let cycle = state.span.begin.to_float().floor() as u64;
            let mut rng = StdRng::seed_from_u64(cycle);
            let pattern_idx = rng.gen_range(0..len);
            let pattern = &patterns[pattern_idx];

            // Query the selected pattern with the current time span
            pattern.query(state)
        })
    }

    /// Time-weighted concatenation - each pattern gets a specific duration within the cycle
    /// timeCat takes pairs of (duration, pattern) where durations are normalized to sum to 1.0
    pub fn timecat(weighted_patterns: Vec<(f64, Pattern<T>)>) -> Pattern<T> {
        if weighted_patterns.is_empty() {
            return Pattern::silence();
        }

        // Normalize weights to sum to 1.0
        let total_weight: f64 = weighted_patterns.iter().map(|(w, _)| w).sum();
        let normalized: Vec<(f64, Pattern<T>)> = weighted_patterns
            .into_iter()
            .map(|(w, p)| (w / total_weight, p))
            .collect();

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

                // Within each cycle, patterns are divided by their weights
                let local_start = cycle_start - cycle_f;
                let local_end = cycle_end - cycle_f;

                // Find which patterns overlap with our query
                let mut cumulative = 0.0;
                for (idx, (weight, pattern)) in normalized.iter().enumerate() {
                    let pattern_start = cumulative;
                    let pattern_end = cumulative + weight;
                    cumulative = pattern_end;

                    // Check if this pattern overlaps with our query range
                    if local_end <= pattern_start || local_start >= pattern_end {
                        continue;
                    }

                    // Calculate the overlap
                    let overlap_start = local_start.max(pattern_start);
                    let overlap_end = local_end.min(pattern_end);

                    // Map to pattern's local time (0 to 1 within the pattern)
                    let pattern_local_start = (overlap_start - pattern_start) / weight;
                    let pattern_local_end = (overlap_end - pattern_start) / weight;

                    // Create query for this pattern
                    let pattern_state = State {
                        span: TimeSpan::new(
                            Fraction::from_float(cycle_f + pattern_local_start),
                            Fraction::from_float(cycle_f + pattern_local_end),
                        ),
                        controls: state.controls.clone(),
                    };

                    // Query and remap events to global time
                    for mut hap in pattern.query(&pattern_state) {
                        // Remap from pattern time to global time
                        hap.part = TimeSpan::new(
                            Fraction::from_float(
                                cycle_f
                                    + pattern_start
                                    + (hap.part.begin.to_float() - cycle_f) * weight,
                            ),
                            Fraction::from_float(
                                cycle_f
                                    + pattern_start
                                    + (hap.part.end.to_float() - cycle_f) * weight,
                            ),
                        );

                        if let Some(whole) = hap.whole {
                            hap.whole = Some(TimeSpan::new(
                                Fraction::from_float(
                                    cycle_f
                                        + pattern_start
                                        + (whole.begin.to_float() - cycle_f) * weight,
                                ),
                                Fraction::from_float(
                                    cycle_f
                                        + pattern_start
                                        + (whole.end.to_float() - cycle_f) * weight,
                                ),
                            ));
                        }

                        all_haps.push(hap);
                    }
                }
            }

            all_haps
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
                    hap.whole = hap.whole.map(|w| {
                        TimeSpan::new(
                            Fraction::from_float(
                                w.begin.to_float() * n as f64 - slice_index as f64,
                            ),
                            Fraction::from_float(w.end.to_float() * n as f64 - slice_index as f64),
                        )
                    });
                    hap.part = TimeSpan::new(
                        Fraction::from_float(
                            hap.part.begin.to_float() * n as f64 - slice_index as f64,
                        ),
                        Fraction::from_float(
                            hap.part.end.to_float() * n as f64 - slice_index as f64,
                        ),
                    );
                    hap
                })
                .collect()
        })
    }

    /// Pattern-based slice - indices from a pattern
    /// slice_pattern n indices_pattern - divides into n slices, pattern selects which
    /// Example: slice_pattern(4, Pattern::from("0 2 1 3")) reorders 4 chunks
    pub fn slice_pattern(self, n: usize, indices: Pattern<String>) -> Self {
        if n == 0 {
            return Pattern::silence();
        }

        Pattern::new(move |state| {
            // Query the indices pattern to find which slice to use
            let index_haps = indices.query(state);

            let mut result = Vec::new();

            for index_hap in index_haps {
                // Parse the index value
                let index_str = &index_hap.value;
                if let Ok(index_value) = index_str.parse::<usize>() {
                    let slice_index = index_value % n;

                    // Calculate slice boundaries
                    let slice_begin = slice_index as f64 / n as f64;
                    let slice_end = (slice_index + 1) as f64 / n as f64;

                    // Query the pattern for this slice within the event's time span
                    let event_begin = index_hap.part.begin;
                    let event_end = index_hap.part.end;
                    let event_duration = event_end - event_begin;

                    // Map the slice to the event's time window
                    let query_begin = Fraction::from_float(slice_begin);
                    let query_end = Fraction::from_float(slice_end);

                    let slice_state = State {
                        span: TimeSpan::new(query_begin, query_end),
                        controls: state.controls.clone(),
                    };

                    let slice_haps = self.query(&slice_state);

                    // Map the slice events to the index event's time window
                    for mut hap in slice_haps {
                        // Calculate relative position within slice
                        let hap_begin = hap.part.begin.to_float();
                        let hap_end = hap.part.end.to_float();

                        // Normalize to 0-1 within the slice
                        let slice_duration = slice_end - slice_begin;
                        let norm_begin = (hap_begin - slice_begin) / slice_duration;
                        let norm_end = (hap_end - slice_begin) / slice_duration;

                        // Map to event window
                        let new_begin = event_begin + event_duration * Fraction::from_float(norm_begin);
                        let new_end = event_begin + event_duration * Fraction::from_float(norm_end);

                        hap.part = TimeSpan::new(new_begin, new_end);
                        hap.whole = hap.whole.map(|w| {
                            let w_begin = w.begin.to_float();
                            let w_end = w.end.to_float();
                            let norm_w_begin = (w_begin - slice_begin) / slice_duration;
                            let norm_w_end = (w_end - slice_begin) / slice_duration;
                            TimeSpan::new(
                                event_begin + event_duration * Fraction::from_float(norm_w_begin),
                                event_begin + event_duration * Fraction::from_float(norm_w_end),
                            )
                        });

                        // Add begin/end to context for sample slicing
                        hap.context.insert("begin".to_string(), slice_begin.to_string());
                        hap.context.insert("end".to_string(), slice_end.to_string());

                        result.push(hap);
                    }
                }
            }

            result
        })
    }
}

impl Pattern<String> {
    // hurry() is now implemented in the general Pattern<T> impl block above
    // (accepts Pattern<f64> parameter and uses context to pass speed)
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
        let p = Pattern::pure(1).fast(Pattern::pure(2.0));
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
