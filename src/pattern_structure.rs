#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! Advanced Structural Pattern Operations
//!
//! Implements bite, ply, linger, inside, outside, iter, and more

use crate::pattern::{Fraction, Hap, Pattern, State, TimeSpan};
use std::sync::Arc;

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Slice pattern into N bits and select which bits to play using a selector pattern
    /// bite n selector_pattern - slices into n equal segments, selector chooses which to play
    /// Example: bite 4 (Pattern::from_string("0 1 2 3")) plays all 4 segments in order
    ///          bite 4 (Pattern::from_string("2 0")) plays 2nd segment, then 0th segment
    pub fn bite(self, n: usize, selector: Pattern<String>) -> Pattern<T> {
        if n == 0 {
            return Pattern::silence();
        }

        Pattern::new(move |state: &State| {
            let mut result = Vec::new();

            // Query the selector pattern to get which segment indices to play
            let selector_events = selector.query(state);

            for selector_hap in selector_events {
                // Parse the selector value as an integer (which segment to play)
                if let Ok(segment_idx) = selector_hap.value.parse::<i32>() {
                    let segment_idx = segment_idx.rem_euclid(n as i32) as usize;

                    // Calculate the time range for this segment in the original pattern
                    let segment_size = 1.0 / n as f64;
                    let segment_start = segment_idx as f64 * segment_size;
                    let segment_end = segment_start + segment_size;

                    // Calculate the cycle this event is in
                    let cycle = selector_hap.part.begin.to_float().floor();

                    // Map the selector event's time span to query the appropriate segment
                    let query_begin = cycle + segment_start;
                    let query_end = cycle + segment_end;

                    let segment_state = State {
                        span: TimeSpan::new(
                            Fraction::from_float(query_begin),
                            Fraction::from_float(query_end),
                        ),
                        controls: state.controls.clone(),
                    };

                    // Query the source pattern for this segment
                    let segment_events = self.query(&segment_state);

                    // Rescale the events from the segment to fit in the selector event's time span
                    for mut event in segment_events {
                        // Calculate relative position within the segment (0.0 to 1.0)
                        let rel_begin = (event.part.begin.to_float() - query_begin) / segment_size;
                        let rel_end = (event.part.end.to_float() - query_begin) / segment_size;

                        // Map to the selector event's time span
                        let event_duration = selector_hap.part.duration().to_float();
                        let new_begin = selector_hap.part.begin.to_float() + rel_begin * event_duration;
                        let new_end = selector_hap.part.begin.to_float() + rel_end * event_duration;

                        event.part = TimeSpan::new(
                            Fraction::from_float(new_begin),
                            Fraction::from_float(new_end),
                        );

                        if let Some(whole) = event.whole {
                            let rel_whole_begin = (whole.begin.to_float() - query_begin) / segment_size;
                            let rel_whole_end = (whole.end.to_float() - query_begin) / segment_size;

                            let new_whole_begin = selector_hap.part.begin.to_float() + rel_whole_begin * event_duration;
                            let new_whole_end = selector_hap.part.begin.to_float() + rel_whole_end * event_duration;

                            event.whole = Some(TimeSpan::new(
                                Fraction::from_float(new_whole_begin),
                                Fraction::from_float(new_whole_end),
                            ));
                        }

                        result.push(event);
                    }
                }
            }

            result
        })
    }

    /// "Chew" through a pattern
    pub fn chew(self, n: usize) -> Self {
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor() as usize;
            let offset = (cycle % n) as f64 / n as f64;

            let adjusted_state = State {
                span: TimeSpan::new(
                    state.span.begin + Fraction::from_float(offset),
                    state.span.end + Fraction::from_float(offset),
                ),
                controls: state.controls.clone(),
            };

            self.query(&adjusted_state)
        })
    }

    /// Repeat each event n times (ply)
    pub fn ply(self, n: usize) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            let mut result = Vec::new();

            for hap in haps {
                let duration = hap.part.duration();
                let step = duration / Fraction::new(n as i64, 1);

                for i in 0..n {
                    let begin = hap.part.begin + step * Fraction::new(i as i64, 1);
                    let end = begin + step;

                    result.push(Hap::new(
                        hap.whole,
                        TimeSpan::new(begin, end),
                        hap.value.clone(),
                    ));
                }
            }
            result
        })
    }

    /// Linger on values for longer
    pub fn linger(self, factor: f64) -> Self {
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor();
            let lingered_cycle = (cycle / factor).floor();

            let adjusted_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(lingered_cycle),
                    Fraction::from_float(lingered_cycle + 1.0),
                ),
                controls: state.controls.clone(),
            };

            let haps = self.query(&adjusted_state);

            // Stretch the haps to fill the current cycle
            haps.into_iter()
                .map(|hap| {
                    let relative_begin =
                        (hap.part.begin.to_float() - lingered_cycle) * factor + cycle;
                    let relative_end = (hap.part.end.to_float() - lingered_cycle) * factor + cycle;

                    Hap::new(
                        hap.whole,
                        TimeSpan::new(
                            Fraction::from_float(relative_begin),
                            Fraction::from_float(relative_end),
                        ),
                        hap.value,
                    )
                })
                .filter(|hap| hap.part.begin < state.span.end && hap.part.end > state.span.begin)
                .collect()
        })
    }

    /// Apply function inside subdivisions
    pub fn inside(
        self,
        n: f64,
        f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    ) -> Pattern<T> {
        let f = Arc::new(f);
        Pattern::new(move |state: &State| {
            // Speed up time by n
            let fast_state = State {
                span: TimeSpan::new(
                    state.span.begin * Fraction::from_float(n),
                    state.span.end * Fraction::from_float(n),
                ),
                controls: state.controls.clone(),
            };

            // Apply function to fast version
            let fast_pattern = f(self.clone());
            fast_pattern.query(&fast_state)
        })
    }

    /// Apply function outside subdivisions
    pub fn outside(
        self,
        n: f64,
        f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    ) -> Pattern<T> {
        let f = Arc::new(f);
        Pattern::new(move |state: &State| {
            // Slow down time by n
            let slow_state = State {
                span: TimeSpan::new(
                    state.span.begin / Fraction::from_float(n),
                    state.span.end / Fraction::from_float(n),
                ),
                controls: state.controls.clone(),
            };

            // Apply function to slow version
            let slow_pattern = f(self.clone());
            slow_pattern.query(&slow_state)
        })
    }

    /// Iterate pattern shifting by 1/n each cycle
    pub fn iter(self, n: usize) -> Self {
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor() as usize;
            let shift = (cycle % n) as f64 / n as f64;

            let shifted_state = State {
                span: TimeSpan::new(
                    state.span.begin - Fraction::from_float(shift),
                    state.span.end - Fraction::from_float(shift),
                ),
                controls: state.controls.clone(),
            };

            self.query(&shifted_state)
        })
    }

    /// Iterate pattern backwards
    pub fn iter_back(self, n: usize) -> Self {
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor() as usize;
            let shift = (cycle % n) as f64 / n as f64;

            let shifted_state = State {
                span: TimeSpan::new(
                    state.span.begin + Fraction::from_float(shift),
                    state.span.end + Fraction::from_float(shift),
                ),
                controls: state.controls.clone(),
            };

            self.query(&shifted_state)
        })
    }

    /// Fast with gaps
    pub fn fast_gap(self, factor: f64) -> Self {
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor();

            // Only show pattern in first 1/factor of each cycle
            let cycle_pos = state.span.begin.to_float() - cycle;
            if cycle_pos >= 1.0 / factor {
                return Vec::new();
            }

            // Fast the pattern
            let fast_state = State {
                span: TimeSpan::new(
                    state.span.begin * Fraction::from_float(factor),
                    state.span.end * Fraction::from_float(factor),
                ),
                controls: state.controls.clone(),
            };

            self.query(&fast_state)
        })
    }

    /// Compress with gaps
    pub fn compress_gap(self, begin: f64, end: f64) -> Self {
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor();
            let cycle_pos = state.span.begin.to_float() - cycle;

            // Only show pattern in specified range
            if cycle_pos < begin || cycle_pos >= end {
                return Vec::new();
            }

            // Map to compressed range
            let duration = end - begin;
            let mapped_begin = (cycle_pos - begin) / duration;
            let mapped_end = mapped_begin + (state.span.duration().to_float() / duration);

            let compressed_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle + mapped_begin),
                    Fraction::from_float(cycle + mapped_end),
                ),
                controls: state.controls.clone(),
            };

            self.query(&compressed_state)
        })
    }

    /// Chunk pattern with gaps between chunks
    pub fn chunk_gap(
        self,
        n: usize,
        f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    ) -> Self {
        let f = Arc::new(f);
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor() as usize;
            let chunk_index = cycle % n;

            // Apply function only to specific chunk
            if chunk_index == 0 {
                f(self.clone()).query(state)
            } else {
                self.query(state)
            }
        })
    }

    /// Unit generator pattern
    pub fn ur(self, n: usize, pat_of_pats: Pattern<String>) -> Pattern<T> {
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor() as usize;
            let ur_cycle = cycle / n;

            // Get pattern selector for this ur cycle
            let selector_state = State {
                span: TimeSpan::new(
                    Fraction::new(ur_cycle as i64, 1),
                    Fraction::new((ur_cycle + 1) as i64, 1),
                ),
                controls: state.controls.clone(),
            };

            let selectors = pat_of_pats.query(&selector_state);
            if selectors.is_empty() {
                return self.query(state);
            }

            // Apply selected transformation
            // This is simplified - real ur would parse the selector string
            self.query(state)
        })
    }

    /// Inhabit pattern - fill with another pattern
    pub fn inhabit<U: Clone + Send + Sync + 'static>(self, inhabitant: Pattern<U>) -> Pattern<U> {
        Pattern::new(move |state: &State| {
            let triggers = self.query(state);
            let mut result = Vec::new();

            for trigger in triggers {
                // Query inhabitant pattern at trigger time
                let inhabit_state = State {
                    span: trigger.part,
                    controls: state.controls.clone(),
                };

                let inhabited = inhabitant.query(&inhabit_state);
                result.extend(inhabited);
            }

            result
        })
    }

    /// Space out events
    pub fn space_out(self, lengths: Vec<f64>) -> Self {
        Pattern::new(move |state: &State| {
            if lengths.is_empty() {
                return Vec::new();
            }

            let total_length: f64 = lengths.iter().sum();
            let haps = self.query(state);
            let mut result = Vec::new();

            let mut current_pos = 0.0;
            for (i, hap) in haps.iter().enumerate() {
                let length = lengths[i % lengths.len()];
                let begin = Fraction::from_float(current_pos / total_length);
                let end = Fraction::from_float((current_pos + length) / total_length);

                result.push(Hap::new(
                    hap.whole,
                    TimeSpan::new(
                        state.span.begin + begin * state.span.duration(),
                        state.span.begin + end * state.span.duration(),
                    ),
                    hap.value.clone(),
                ));

                current_pos += length;
            }

            result
        })
    }

    /// Discretize continuous patterns
    pub fn discretise(self, n: usize) -> Self {
        Pattern::new(move |state: &State| {
            let mut result = Vec::new();
            let step = state.span.duration() / Fraction::new(n as i64, 1);

            for i in 0..n {
                let begin = state.span.begin + step * Fraction::new(i as i64, 1);
                let end = begin + step;

                let sample_state = State {
                    span: TimeSpan::new(begin, begin + Fraction::new(1, 1000)), // Small sample
                    controls: state.controls.clone(),
                };

                if let Some(hap) = self.query(&sample_state).first() {
                    result.push(Hap::new(
                        Some(TimeSpan::new(begin, end)),
                        TimeSpan::new(begin, end),
                        hap.value.clone(),
                    ));
                }
            }

            result
        })
    }

    /// Superimpose function results
    pub fn superimpose(
        self,
        f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    ) -> Pattern<T> {
        let f = Arc::new(f);
        Pattern::new(move |state: &State| {
            let mut result = self.query(state);
            let transformed = f(self.clone()).query(state);
            result.extend(transformed);
            result
        })
    }

    /// Layer multiple transformations
    pub fn layer(self, fs: Vec<Box<dyn Fn(Pattern<T>) -> Pattern<T> + Send + Sync>>) -> Pattern<T> {
        Pattern::new(move |state: &State| {
            let mut result = Vec::new();

            for f in &fs {
                let transformed = f(self.clone()).query(state);
                result.extend(transformed);
            }

            result
        })
    }

    /// Step sequencing
    pub fn steps(self, steps: Vec<Option<T>>, durations: Vec<f64>) -> Pattern<T> {
        Pattern::new(move |state: &State| {
            let total_duration: f64 = durations.iter().sum();
            let mut result = Vec::new();
            let mut current_pos = 0.0;

            for (i, step) in steps.iter().enumerate() {
                if let Some(value) = step {
                    let duration = durations[i % durations.len()];
                    let begin = Fraction::from_float(current_pos / total_duration);
                    let end = Fraction::from_float((current_pos + duration) / total_duration);

                    let hap_begin = state.span.begin + begin * state.span.duration();
                    let hap_end = state.span.begin + end * state.span.duration();

                    if hap_begin < state.span.end && hap_end > state.span.begin {
                        result.push(Hap::new(
                            Some(state.span),
                            TimeSpan::new(hap_begin, hap_end),
                            value.clone(),
                        ));
                    }
                }
                current_pos += durations[i % durations.len()];
            }

            result
        })
    }

    /// Swing specific beats
    pub fn swing_by(self, amount: f64, selector: Pattern<bool>) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            let selections = selector.query(state);

            haps.into_iter()
                .enumerate()
                .map(|(i, mut hap)| {
                    // Check if this event should be swung
                    let should_swing = selections
                        .iter()
                        .any(|s| s.value && s.part.begin <= hap.part.begin);

                    if should_swing && i % 2 == 1 {
                        let shift = Fraction::from_float(amount);
                        hap.part = TimeSpan::new(hap.part.begin + shift, hap.part.end + shift);
                        if let Some(whole) = hap.whole.as_mut() {
                            *whole = TimeSpan::new(whole.begin + shift, whole.end + shift);
                        }
                    }
                    hap
                })
                .collect()
        })
    }
}

/// Time concatenation - concatenate patterns with specific durations
pub fn timecat<T: Clone + Send + Sync + 'static>(specs: Vec<(f64, Pattern<T>)>) -> Pattern<T> {
    Pattern::new(move |state: &State| {
        let total_duration: f64 = specs.iter().map(|(d, _)| d).sum();
        let mut result = Vec::new();
        let mut current_pos = 0.0;

        for (duration, pattern) in &specs {
            let begin = current_pos / total_duration;
            let end = (current_pos + duration) / total_duration;

            let pattern_state = State {
                span: TimeSpan::new(Fraction::from_float(begin), Fraction::from_float(end)),
                controls: state.controls.clone(),
            };

            let haps = pattern.query(&pattern_state);

            // Adjust hap times to fit in the current state
            for hap in haps {
                let adjusted_begin = state.span.begin
                    + Fraction::from_float(begin) * state.span.duration()
                    + (hap.part.begin - Fraction::from_float(begin)) * state.span.duration();
                let adjusted_end = adjusted_begin + hap.part.duration() * state.span.duration();

                result.push(Hap::new(
                    hap.whole,
                    TimeSpan::new(adjusted_begin, adjusted_end),
                    hap.value,
                ));
            }

            current_pos += duration;
        }

        result
    })
}

/// Wait for n cycles before playing pattern
pub fn wait<T: Clone + Send + Sync + 'static>(cycles: i32, pattern: Pattern<T>) -> Pattern<T> {
    Pattern::new(move |state: &State| {
        let cycle = state.span.begin.to_float().floor() as i32;

        if cycle < cycles {
            Vec::new()
        } else {
            let adjusted_state = State {
                span: TimeSpan::new(
                    state.span.begin - Fraction::new(cycles as i64, 1),
                    state.span.end - Fraction::new(cycles as i64, 1),
                ),
                controls: state.controls.clone(),
            };
            pattern.query(&adjusted_state)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Pattern;
    use std::collections::HashMap;

    #[test]
    fn test_ply() {
        let p = Pattern::from_string("a b");
        let plied = p.ply(3);

        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        let haps = plied.query(&state);
        assert_eq!(haps.len(), 6); // 2 events * 3 repetitions
    }

    #[test]
    fn test_iter() {
        let p = Pattern::from_string("a b c d");
        let iterated = p.iter(4);

        // First cycle should be unshifted
        let state1 = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps1 = iterated.query(&state1);
        assert_eq!(haps1[0].value, "a");

        // Second cycle should be shifted by 1/4
        let state2 = State {
            span: TimeSpan::new(Fraction::new(1, 1), Fraction::new(2, 1)),
            controls: HashMap::new(),
        };
        let haps2 = iterated.query(&state2);
        // After shift, "d" from previous cycle appears first
        assert_eq!(haps2[0].value, "d");
    }

    #[test]
    fn test_timecat() {
        let p1 = Pattern::from_string("a");
        let p2 = Pattern::from_string("b");
        let cat = timecat(vec![(1.0, p1), (2.0, p2)]);

        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        let haps = cat.query(&state);
        assert_eq!(haps.len(), 2);

        // "a" should take 1/3 of the cycle
        assert!(haps[0].part.duration().to_float() < 0.4);
        // "b" should take 2/3 of the cycle
        assert!(haps[1].part.duration().to_float() > 0.6);
    }
}
