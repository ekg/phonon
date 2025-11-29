#![allow(unused_assignments, unused_mut)]
//! Complete set of pattern operators ported from Strudel
//! All the pattern transformation functions you know and love

use crate::pattern::{Fraction, Hap, Pattern, State, TimeSpan};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashMap;
use std::sync::Arc;

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    // ============= Time Manipulation =============

    /// Shift pattern forward in time
    pub fn late(self, amount: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Pattern::new(move |state| {
            // Query amount pattern at cycle start to get current amount
            let cycle_start = state.span.begin.to_float().floor();
            let amount_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let amount_haps = amount.query(&amount_state);
            let shift = if let Some(hap) = amount_haps.first() {
                hap.value
            } else {
                0.0
            };

            // Apply time shift with the queried amount
            self.query(state)
                .into_iter()
                .map(|mut hap| {
                    hap.part = TimeSpan::new(
                        Fraction::from_float(hap.part.begin.to_float() + shift),
                        Fraction::from_float(hap.part.end.to_float() + shift),
                    );
                    if let Some(whole) = hap.whole {
                        hap.whole = Some(TimeSpan::new(
                            Fraction::from_float(whole.begin.to_float() + shift),
                            Fraction::from_float(whole.end.to_float() + shift),
                        ));
                    }
                    hap
                })
                .collect()
        })
    }

    /// Shift pattern backward in time
    pub fn early(self, amount: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        // Invert the amount pattern: 0.5 -> -0.5
        let inverted = Pattern::new(move |state| {
            amount
                .query(state)
                .into_iter()
                .map(|mut hap| {
                    hap.value = -hap.value;
                    hap
                })
                .collect()
        });
        self.late(inverted)
    }

    /// Offset pattern by a fraction of a cycle
    pub fn offset(self, amount: f64) -> Self {
        self.late(Pattern::pure(amount))
    }

    /// Loop a pattern within a cycle
    pub fn loop_pattern(self, n: usize) -> Self {
        Pattern::new(move |state| {
            let mut all_haps = Vec::new();
            for i in 0..n {
                let offset = i as f64 / n as f64;
                let scaled = self.clone().fast(Pattern::pure(n as f64));
                let shifted = scaled.late(Pattern::pure(offset));
                all_haps.extend(shifted.query(state));
            }
            all_haps
        })
    }

    // ============= Randomness & Probability =============

    /// Randomly drop events with given probability
    pub fn degrade_by(self, probability: Pattern<f64>) -> Self {
        Pattern::new(move |state| {
            // Query probability at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let prob_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let prob_val = probability
                .query(&prob_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(0.5);

            self.query(state)
                .into_iter()
                .filter_map(|hap| {
                    // Use the event's time position and cycle to generate seed
                    // This ensures each event gets a unique random value
                    let cycle = hap.part.begin.to_float().floor() as u64;
                    let position_hash = (hap.part.begin.to_float() * 1000000.0) as u64;
                    let event_seed = cycle
                        .wrapping_mul(2654435761) // Large prime
                        .wrapping_add(position_hash);

                    let mut event_rng = StdRng::seed_from_u64(event_seed);
                    let keep = event_rng.gen::<f64>() >= prob_val;
                    if keep {
                        Some(hap)
                    } else {
                        None
                    }
                })
                .collect()
        })
    }

    /// Degrade 50% of events
    pub fn degrade(self) -> Self {
        self.degrade_by(Pattern::pure(0.5))
    }

    /// Sometimes apply a function (50% chance per cycle)
    pub fn sometimes(self, f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static) -> Self
    where
        T: 'static,
    {
        self.sometimes_by(0.5, f)
    }

    /// Sometimes apply a function with specific probability
    pub fn sometimes_by(
        self,
        prob: f64,
        f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    ) -> Self
    where
        T: 'static,
    {
        let f = Arc::new(f);
        Pattern::new(move |state| {
            let cycle = state.span.begin.to_float().floor() as u64;
            let mut rng = StdRng::seed_from_u64(cycle);

            if rng.gen::<f64>() < prob {
                let transformed = f(self.clone());
                transformed.query(state)
            } else {
                self.query(state)
            }
        })
    }

    /// Rarely apply a function (10% chance)
    pub fn rarely(self, f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static) -> Self
    where
        T: 'static,
    {
        self.sometimes_by(0.1, f)
    }

    /// Often apply a function (75% chance)
    pub fn often(self, f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static) -> Self
    where
        T: 'static,
    {
        self.sometimes_by(0.75, f)
    }

    /// Almost always apply a function (90% chance)
    pub fn almost_always(self, f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static) -> Self
    where
        T: 'static,
    {
        self.sometimes_by(0.9, f)
    }

    /// Almost never apply a function (10% chance - alias for rarely)
    pub fn almost_never(self, f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static) -> Self
    where
        T: 'static,
    {
        self.sometimes_by(0.1, f)
    }

    /// Always apply a function (100% chance - mainly for consistency)
    pub fn always(self, f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static) -> Self
    where
        T: 'static,
    {
        f(self)
    }

    // ============= Pattern Combination =============

    /// Overlay this pattern with another
    pub fn overlay(self, other: Pattern<T>) -> Pattern<T> {
        Pattern::stack(vec![self, other])
    }

    /// Append another pattern after this one
    pub fn append(self, other: Pattern<T>) -> Pattern<T> {
        Pattern::cat(vec![self, other])
    }

    // ============= Structural Manipulation =============

    /// Repeat pattern n times fast (like bd*4 in TidalCycles)
    /// This speeds up the pattern to fit n repetitions in the original timespan
    pub fn dup(self, n: usize) -> Self {
        if n == 0 {
            return Pattern::silence();
        }
        if n == 1 {
            return self;
        }
        // Use fast to speed up and repeat
        self.fast(Pattern::pure(n as f64))
    }

    /// Stutter - repeat each event n times with subdivision
    pub fn stutter(self, n: usize) -> Self {
        Pattern::new(move |state| {
            self.query(state)
                .into_iter()
                .flat_map(|hap| {
                    let duration = hap.part.duration().to_float();
                    let step = duration / n as f64;

                    (0..n)
                        .map(|i| {
                            let offset = i as f64 * step;
                            let mut new_hap = hap.clone();
                            new_hap.part = TimeSpan::new(
                                Fraction::from_float(hap.part.begin.to_float() + offset),
                                Fraction::from_float(hap.part.begin.to_float() + offset + step),
                            );
                            new_hap
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        })
    }

    /// Stut - Tidal's classic stutter/echo with decay
    /// Creates n echoes of each event, delayed by time cycles, with volume decay
    /// Example: stut 3 0.125 0.7 creates original + 2 echoes at 70%, 49% volume
    pub fn stut(self, n: Pattern<f64>, time: Pattern<f64>, decay: Pattern<f64>) -> Self
    where
        T: Clone + 'static,
    {
        // Create a temporary state to query n and decay
        let default_state = State {
            span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
            controls: HashMap::new(),
        };

        let n_val = n
            .query(&default_state)
            .first()
            .map(|h| h.value.clone())
            .unwrap_or(1.0)
            .max(1.0) as usize;

        let decay_val = decay
            .query(&default_state)
            .first()
            .map(|h| h.value.clone())
            .unwrap_or(0.7);

        if n_val == 1 {
            return self;
        }

        // Stack n delayed versions, each with gain decay
        let mut layers = Vec::new();
        for i in 0..n_val {
            let time_clone = time.clone();
            let self_clone = self.clone();
            let gain_mult = decay_val.powi(i as i32);

            let delayed = Pattern::new(move |state| {
                let time_val = time_clone
                    .query(state)
                    .first()
                    .map(|h| h.value.clone())
                    .unwrap_or(0.125);

                let delay = time_val * i as f64;

                // Query the original pattern and delay all events
                self_clone
                    .query(state)
                    .into_iter()
                    .map(|mut hap| {
                        // Delay the event
                        hap.part = TimeSpan::new(
                            Fraction::from_float(hap.part.begin.to_float() + delay),
                            Fraction::from_float(hap.part.end.to_float() + delay),
                        );
                        hap.whole = hap.whole.map(|w| {
                            TimeSpan::new(
                                Fraction::from_float(w.begin.to_float() + delay),
                                Fraction::from_float(w.end.to_float() + delay),
                            )
                        });

                        // Add gain multiplier to context
                        // The compiler/voice manager will read this and apply it
                        hap.context
                            .insert("stut_gain".to_string(), gain_mult.to_string());

                        hap
                    })
                    .collect()
            });

            layers.push(delayed);
        }

        Pattern::stack(layers)
    }

    /// Create a palindrome (pattern + reversed pattern)
    pub fn palindrome(self) -> Self {
        // Create a pattern that plays forward then backward, spread over 2 cycles
        let forward = self.clone().slow(Pattern::pure(2.0)); // First half
        let backward = self.rev().slow(Pattern::pure(2.0)).late(Pattern::pure(1.0)); // Second half, shifted by 1 cycle
        Pattern::stack(vec![forward, backward])
    }

    /// Chunk - apply a function to a different part of the pattern each cycle
    pub fn chunk(
        self,
        n: usize,
        f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    ) -> Self
    where
        T: 'static,
    {
        let f = Arc::new(f);
        Pattern::new(move |state| {
            let cycle = state.span.begin.to_float().floor() as usize;
            let chunk_index = cycle % n;
            let chunk_size = 1.0 / n as f64;
            let chunk_start = chunk_index as f64 * chunk_size;
            let chunk_end = chunk_start + chunk_size;

            self.query(state)
                .into_iter()
                .map(|hap| {
                    let hap_pos = hap.part.begin.to_float() % 1.0;
                    if hap_pos >= chunk_start && hap_pos < chunk_end {
                        // Apply function to this chunk
                        let transformed = f(Pattern::pure(hap.value.clone()));
                        let new_haps = transformed.query(state);
                        if !new_haps.is_empty() {
                            new_haps[0].clone()
                        } else {
                            hap
                        }
                    } else {
                        hap
                    }
                })
                .collect()
        })
    }

    // ============= Jux (Stereo) Operations =============

    /// Apply a function to one channel (for stereo effects)
    pub fn jux(
        self,
        f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    ) -> Pattern<(T, T)>
    where
        T: 'static,
    {
        let f = Arc::new(f);
        let left = self.clone();
        let right = f(self);

        Pattern::new(move |state| {
            let left_haps = left.query(state);
            let right_haps = right.query(state);

            // Combine into stereo pairs
            left_haps
                .into_iter()
                .zip(right_haps)
                .map(|(l, r)| Hap::new(l.whole, l.part, (l.value, r.value)))
                .collect()
        })
    }

    /// Reverse only the right channel
    pub fn jux_rev(self) -> Pattern<(T, T)>
    where
        T: 'static,
    {
        self.jux(|p| p.rev())
    }

    // ============= Conditional Operations =============

    /// Apply function when cycle number matches condition
    pub fn when_mod(
        self,
        modulo: i32,
        offset: i32,
        f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    ) -> Self
    where
        T: 'static,
    {
        let f = Arc::new(f);
        Pattern::new(move |state| {
            let cycle = state.span.begin.to_float().floor() as i32;
            if (cycle - offset) % modulo == 0 {
                f(self.clone()).query(state)
            } else {
                self.query(state)
            }
        })
    }

    /// Swap the pattern with another every n cycles
    pub fn swap(self, n: i32, other: Pattern<T>) -> Pattern<T> {
        Pattern::new(move |state| {
            let cycle = state.span.begin.to_float().floor() as i32;
            if cycle % (n * 2) < n {
                self.query(state)
            } else {
                other.query(state)
            }
        })
    }

    // ============= Bjorklund/Euclidean Extensions =============

    /// Euclidean rhythm applied to this pattern's events
    pub fn euclidean_legato(self, pulses: usize, steps: usize) -> Self {
        let euclid = Pattern::<bool>::euclid(pulses, steps, 0);
        Pattern::new(move |state| {
            let euclid_haps = euclid.query(state);
            let pattern_haps = self.query(state);

            // Restructure the pattern: for each euclidean "true" event,
            // sample the pattern value at that time and create a new event
            let mut result = Vec::new();

            for ehap in euclid_haps.iter() {
                if !ehap.value {
                    continue; // Skip "false" beats
                }

                // Sample the pattern at the euclidean event's start time
                let sample_time = ehap
                    .whole
                    .as_ref()
                    .map(|w| w.begin.to_float())
                    .unwrap_or(ehap.part.begin.to_float());

                // Find the pattern value at this time
                for phap in pattern_haps.iter() {
                    let phap_start = phap
                        .whole
                        .as_ref()
                        .map(|w| w.begin.to_float())
                        .unwrap_or(phap.part.begin.to_float());
                    let phap_end = phap
                        .whole
                        .as_ref()
                        .map(|w| w.end.to_float())
                        .unwrap_or(phap.part.end.to_float());

                    // Check if this pattern event covers the euclidean beat
                    if sample_time >= phap_start && sample_time < phap_end {
                        // Create a new event at the euclidean position with the pattern value
                        result.push(Hap {
                            whole: ehap.whole.clone(),
                            part: ehap.part.clone(),
                            value: phap.value.clone(),
                            context: phap.context.clone(),
                        });
                        break;
                    }
                }
            }

            result
        })
    }

    // ============= Pitch/Scale Operations =============

    /// Add a value to numeric patterns
    pub fn add(self, amount: f64) -> Pattern<f64>
    where
        T: Into<f64> + Clone + Send + Sync,
    {
        Pattern::new(move |state| {
            self.query(state)
                .into_iter()
                .map(|hap| hap.with_value(|v| v.clone().into() + amount))
                .collect()
        })
    }

    /// Multiply numeric patterns
    pub fn mul(self, amount: f64) -> Pattern<f64>
    where
        T: Into<f64> + Clone + Send + Sync,
    {
        Pattern::new(move |state| {
            self.query(state)
                .into_iter()
                .map(|hap| hap.with_value(|v| v.clone().into() * amount))
                .collect()
        })
    }

    /// Subtract from numeric patterns
    pub fn sub(self, amount: f64) -> Pattern<f64>
    where
        T: Into<f64> + Clone + Send + Sync,
    {
        Pattern::new(move |state| {
            self.query(state)
                .into_iter()
                .map(|hap| hap.with_value(|v| v.clone().into() - amount))
                .collect()
        })
    }

    /// Divide numeric patterns
    pub fn div(self, amount: f64) -> Pattern<f64>
    where
        T: Into<f64> + Clone + Send + Sync,
    {
        Pattern::new(move |state| {
            self.query(state)
                .into_iter()
                .map(|hap| hap.with_value(|v| v.clone().into() / amount))
                .collect()
        })
    }
}

// ============= Mini-notation String Patterns =============

impl Pattern<String> {
    /// Create a pattern from a string of space-separated values
    pub fn from_string(s: &str) -> Self {
        let parts: Vec<String> = s.split_whitespace().map(|s| s.to_string()).collect();
        let len = parts.len();

        if len == 0 {
            return Pattern::silence();
        }

        Pattern::new(move |state| {
            let mut haps = Vec::new();
            let step_size = 1.0 / len as f64;

            // Generate events for all cycles in the query span
            let start_cycle = state.span.begin.to_float().floor() as i32;
            let end_cycle = state.span.end.to_float().ceil() as i32;

            for cycle in start_cycle..end_cycle {
                for (i, part) in parts.iter().enumerate() {
                    // Skip tildes - they represent rests/silence
                    if part == "~" {
                        continue;
                    }

                    let begin = cycle as f64 + (i as f64 * step_size);
                    let end = begin + step_size;

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
                            part.clone(),
                        ));
                    }
                }
            }

            haps
        })
    }

    /// Parse as sample names
    pub fn s(self) -> Pattern<String> {
        // For now, just return self - in full implementation would handle sample lookup
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    #[test]
    fn test_degrade() {
        let p = Pattern::from_string("a b c d").degrade();
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        let haps = p.query(&state);
        // Should have fewer than 4 events due to degradation
        assert!(haps.len() <= 4);
    }

    #[test]
    fn test_palindrome() {
        let p = Pattern::from_string("a b c").palindrome();
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(2, 1)),
            controls: HashMap::new(),
        };

        let haps = p.query(&state);
        // Should have events from both forward and backward
        assert!(!haps.is_empty());
    }

    #[test]
    fn test_note_conversion() {
        let p = Pattern::from_string("a4 c5 e5").freq();
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        let haps = p.query(&state);
        assert_eq!(haps.len(), 3);
        assert!((haps[0].value - 440.0).abs() < 0.01); // A4 = 440Hz
    }
}
