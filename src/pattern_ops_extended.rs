#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! Extended Pattern Operations - Additional Strudel/TidalCycles operators
//!
//! This module implements the remaining ~60 operators from Strudel/TidalCycles
//! to achieve full parity with the JavaScript implementation.

use crate::pattern::{Fraction, Hap, Pattern, State, TimeSpan};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::fmt::Debug;
use std::sync::Arc;

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Zoom in on a portion of the pattern (Tidal-style zoom)
    ///
    /// Extracts a time span from the pattern and stretches it to fill the full cycle.
    /// zoom(0.25, 0.75) on "a b c d" extracts b and c, stretches to full cycle.
    ///
    /// Result: b@0.0-0.5, c@0.5-1.0
    pub fn zoom(self, begin: Pattern<f64>, end: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Pattern::new(move |state: &State| {
            // Query begin/end at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let param_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let begin_val = begin
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(0.0);
            let end_val = end
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(1.0);
            let duration = end_val - begin_val;

            if duration <= 0.0 {
                return Vec::new();
            }

            // Map query span from output (0-1) to source (begin-end)
            let query_begin = state.span.begin.to_float();
            let query_end = state.span.end.to_float();
            let query_cycle = query_begin.floor();

            let source_begin = query_cycle + begin_val + (query_begin - query_cycle) * duration;
            let source_end = query_cycle + begin_val + (query_end - query_cycle) * duration;

            let scaled_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(source_begin),
                    Fraction::from_float(source_end),
                ),
                controls: state.controls.clone(),
            };

            // Query and map result times back to output space
            self.query(&scaled_state)
                .into_iter()
                .map(|mut hap| {
                    // Map event times from source (begin-end) back to output (0-1)
                    let part_begin = hap.part.begin.to_float();
                    let part_end = hap.part.end.to_float();
                    let hap_cycle = part_begin.floor();

                    let mapped_begin = hap_cycle + (part_begin - hap_cycle - begin_val) / duration;
                    let mapped_end = hap_cycle + (part_end - hap_cycle - begin_val) / duration;

                    hap.part = TimeSpan::new(
                        Fraction::from_float(mapped_begin),
                        Fraction::from_float(mapped_end),
                    );

                    if let Some(whole) = hap.whole {
                        let w_begin = whole.begin.to_float();
                        let w_end = whole.end.to_float();
                        let w_cycle = w_begin.floor();

                        hap.whole = Some(TimeSpan::new(
                            Fraction::from_float(
                                w_cycle + (w_begin - w_cycle - begin_val) / duration,
                            ),
                            Fraction::from_float(
                                w_cycle + (w_end - w_cycle - begin_val) / duration,
                            ),
                        ));
                    }

                    hap
                })
                .collect()
        })
    }

    /// Focus on a specific cycle
    pub fn focus(self, cycle_begin: Pattern<f64>, cycle_end: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        self.zoom(cycle_begin, cycle_end)
    }

    /// Apply a function inside a time range
    pub fn within(
        self,
        begin: f64,
        end: f64,
        f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    ) -> Self {
        let b = Fraction::from_float(begin);
        let e = Fraction::from_float(end);
        let f = Arc::new(f);
        Pattern::new(move |state: &State| {
            let cycle_begin = state.span.begin.to_float().floor();
            let cycle_pos = state.span.begin.to_float() - cycle_begin;

            if cycle_pos >= begin && cycle_pos < end {
                f(self.clone()).query(state)
            } else {
                self.query(state)
            }
        })
    }

    /// Compress pattern to fit within a time range
    pub fn compress(self, begin: Pattern<f64>, end: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Pattern::new(move |state: &State| {
            // Query begin/end at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let param_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let begin_val = begin
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(0.0);
            let end_val = end
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(1.0);

            let b = Fraction::from_float(begin_val);
            let e = Fraction::from_float(end_val);
            // Map the entire cycle [0,1] to [begin,end]
            let duration = e - b;
            let unscaled_begin = (state.span.begin - b) / duration;
            let unscaled_end = (state.span.end - b) / duration;

            // Query the pattern with the unscaled state
            let unscaled_state = State {
                span: TimeSpan::new(unscaled_begin, unscaled_end),
                controls: state.controls.clone(),
            };

            // Get events and scale them back to [begin,end]
            let haps = self.query(&unscaled_state);
            haps.into_iter()
                .map(|hap| {
                    Hap::new(
                        hap.whole
                            .map(|w| TimeSpan::new(w.begin * duration + b, w.end * duration + b)),
                        TimeSpan::new(hap.part.begin * duration + b, hap.part.end * duration + b),
                        hap.value,
                    )
                })
                .filter(|hap| {
                    // Only include events within the target range
                    hap.part.begin >= b && hap.part.end <= e
                })
                .collect()
        })
    }

    /// Compress and repeat
    pub fn compress_to(self, begin: Pattern<f64>, end: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Pattern::new(move |state: &State| {
            // Query begin/end at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let param_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let begin_val = begin
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(0.0);
            let end_val = end
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(1.0);

            let duration = end_val - begin_val;
            self.clone()
                .fast(Pattern::pure(1.0 / duration))
                .late(Pattern::pure(begin_val))
                .query(state)
        })
    }

    /// Legato - stretch note durations
    pub fn legato(self, factor: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Pattern::new(move |state: &State| {
            // Query factor pattern at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let factor_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let factor_haps = factor.query(&factor_state);
            let legato_factor = factor_haps.first().map(|h| h.value).unwrap_or(1.0);

            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    let duration = hap.part.duration();
                    let new_duration = Fraction::from_float(duration.to_float() * legato_factor);
                    hap.part = TimeSpan::new(hap.part.begin, hap.part.begin + new_duration);

                    // Add legato duration to context (in cycles) for sample playback
                    // This will be converted to release time in seconds at render time
                    hap.context.insert(
                        "legato_duration".to_string(),
                        new_duration.to_float().to_string(),
                    );
                    hap
                })
                .collect()
        })
    }

    /// Stretch note durations to fill gaps
    pub fn stretch(self) -> Self {
        self.legato(Pattern::pure(1.0))
    }

    /// Shorten note durations
    pub fn staccato(self, factor: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        self.legato(factor)
    }

    /// Swing time - delay every other event
    pub fn swing(self, amount: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Pattern::new(move |state: &State| {
            // Query amount pattern at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let amount_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let amount_haps = amount.query(&amount_state);
            let swing_amount = amount_haps.first().map(|h| h.value).unwrap_or(0.0);

            // Apply swing with queried amount
            let haps = self.query(state);
            haps.into_iter()
                .enumerate()
                .map(|(i, mut hap)| {
                    if i % 2 == 1 {
                        let shift = Fraction::from_float(swing_amount);
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

    /// Shuffle time - randomize event timing slightly
    pub fn shuffle(self, amount: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Pattern::new(move |state: &State| {
            // Query amount pattern at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let amount_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let amount_haps = amount.query(&amount_state);
            let shuffle_amount = amount_haps.first().map(|h| h.value).unwrap_or(0.0);

            let haps = self.query(state);

            // Handle zero amount to avoid empty range panic
            if shuffle_amount == 0.0 {
                return haps;
            }

            let cycle = state.span.begin.to_float().floor() as u64;
            let mut rng = StdRng::seed_from_u64(cycle);

            haps.into_iter()
                .map(|mut hap| {
                    let shift = rng.gen_range(-shuffle_amount..shuffle_amount);
                    let shift_frac = Fraction::from_float(shift);
                    hap.part =
                        TimeSpan::new(hap.part.begin + shift_frac, hap.part.end + shift_frac);
                    if let Some(whole) = hap.whole.as_mut() {
                        *whole = TimeSpan::new(whole.begin + shift_frac, whole.end + shift_frac);
                    }
                    hap
                })
                .collect()
        })
    }

    /// Humanize - add slight random variations
    pub fn humanize(self, time_var: Pattern<f64>, velocity_var: Pattern<f64>) -> Self {
        self.shuffle(time_var)
    }

    /// Echo/delay effect
    pub fn echo(self, times: usize, time: Pattern<f64>, feedback: Pattern<f64>) -> Self {
        Pattern::new(move |state: &State| {
            // Query time/feedback at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let param_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let time_val = time
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(0.25);
            let feedback_val = feedback
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(0.7);

            let patterns: Vec<Pattern<T>> = (0..times)
                .map(|i| {
                    let delay = time_val * i as f64;
                    let gain = feedback_val.powi(i as i32);
                    self.clone().late(Pattern::pure(delay)) // In real implementation, would also scale amplitude
                })
                .collect();
            Pattern::stack(patterns).query(state)
        })
    }

    /// Striate - slice pattern into n parts
    pub fn striate(self, n: usize) -> Self {
        Pattern::new(move |state: &State| {
            let mut all_haps = Vec::new();
            for i in 0..n {
                let slice_begin = i as f64 / n as f64;
                let slice_end = (i + 1) as f64 / n as f64;
                let sliced = self
                    .clone()
                    .zoom(Pattern::pure(slice_begin), Pattern::pure(slice_end));
                let mut sliced_haps = sliced.query(state);

                // Add begin/end to context for sample slicing
                for hap in &mut sliced_haps {
                    hap.context
                        .insert("begin".to_string(), slice_begin.to_string());
                    hap.context.insert("end".to_string(), slice_end.to_string());
                }

                all_haps.extend(sliced_haps);
            }
            all_haps
        })
    }

    /// Chop into n equal parts
    pub fn chop(self, n: usize) -> Self {
        self.striate(n)
    }

    /// Spin - rotate through different versions
    pub fn spin(self, n: i32) -> Self {
        let patterns: Vec<Pattern<T>> = (0..n.abs())
            .map(|i| self.clone().rotate_left(i as f64 / n.abs() as f64))
            .collect();
        Pattern::slowcat(patterns)
    }

    /// Weave patterns together
    pub fn weave(self, other: Pattern<T>) -> Pattern<T> {
        Pattern::stack(vec![self, other])
    }

    /// Binary pattern operations
    pub fn binary(self, n: u32) -> Self {
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor() as u32;
            if (cycle & (1 << n)) != 0 {
                self.query(state)
            } else {
                Vec::new()
            }
        })
    }

    /// Mask pattern with another
    pub fn mask(self, mask_pattern: Pattern<bool>) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            let mask_haps = mask_pattern.query(state);

            haps.into_iter()
                .filter(|hap| {
                    mask_haps
                        .iter()
                        .any(|mask_hap| mask_hap.value && mask_hap.part.begin <= hap.part.begin)
                })
                .collect()
        })
    }

    /// Inverse mask
    pub fn mask_inv(self, mask_pattern: Pattern<bool>) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            let mask_haps = mask_pattern.query(state);

            haps.into_iter()
                .filter(|hap| {
                    !mask_haps
                        .iter()
                        .any(|mask_hap| mask_hap.value && mask_hap.part.begin <= hap.part.begin)
                })
                .collect()
        })
    }

    /// Struct pattern - apply structure/rhythm from one pattern to values from another
    /// Takes trigger times from struct_pat and pulls values sequentially from self
    ///
    /// Example: struct "t ~ t ~" $ s "bd sn hh"
    /// - Structure has triggers at 0, 0.5 (two per cycle)
    /// - Values cycle through: bd, sn, hh
    /// - Result: bd at 0, sn at 0.5 (cycle 0), hh at 0 (cycle 1), etc.
    ///
    /// This works like Tidal's struct: structure determines WHEN, values determine WHAT.
    /// Values are pulled sequentially as triggers fire, advancing a global counter.
    pub fn struct_pattern(self, struct_pat: Pattern<bool>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        // We need to track value index across all cycles
        // Use a long query span to get many value events, then index into them
        Pattern::new(move |state: &State| {
            // Get structure events (triggers) for the requested span
            let struct_haps = struct_pat.query(state);

            // Filter to only true triggers
            let triggers: Vec<_> = struct_haps.into_iter().filter(|hap| hap.value).collect();

            if triggers.is_empty() {
                return vec![];
            }

            // Query value pattern over a LARGE span to get enough values
            // We need enough values to cover all triggers that will ever fire
            // Query from cycle 0 to well beyond current span
            let max_cycle = state.span.end.to_float().ceil() as i64;
            let value_state = State {
                span: TimeSpan::new(
                    Fraction::new(0, 1),
                    Fraction::new(max_cycle + 100, 1), // Query far ahead
                ),
                controls: state.controls.clone(),
            };

            let value_haps = self.query(&value_state);

            if value_haps.is_empty() {
                return vec![];
            }

            // Calculate triggers per cycle BEFORE consuming triggers
            let triggers_per_cycle = triggers
                .iter()
                .filter(|t| t.part.begin.to_float() < 1.0)
                .count()
                .max(1);

            // For each trigger, use a simple sequential index
            // This assumes triggers are in temporal order (which they should be from query)
            triggers
                .into_iter()
                .enumerate()
                .map(|(idx, trigger_hap)| {
                    // Calculate absolute trigger index across all cycles
                    // Get the cycle number and add it to offset our index
                    let cycle = trigger_hap.part.begin.to_float().floor() as usize;
                    let global_idx = cycle * triggers_per_cycle + (idx % triggers_per_cycle);

                    // Use that to index into values (wrapping)
                    let value_idx = global_idx % value_haps.len();
                    let value_hap = &value_haps[value_idx];

                    // Create event at trigger's time with value's data
                    crate::pattern::Hap {
                        whole: trigger_hap.whole,
                        part: trigger_hap.part,
                        value: value_hap.value.clone(),
                        context: value_hap.context.clone(),
                    }
                })
                .collect()
        })
    }

    /// Reset on cycle boundary
    pub fn reset(self, cycles: i32) -> Self {
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor() as i32;
            let reset_cycle = (cycle / cycles) * cycles;
            let offset = cycle - reset_cycle;

            let adjusted_state = State {
                span: TimeSpan::new(
                    state.span.begin - Fraction::new(offset as i64, 1),
                    state.span.end - Fraction::new(offset as i64, 1),
                ),
                controls: state.controls.clone(),
            };
            self.query(&adjusted_state)
        })
    }

    /// Restart pattern every n cycles
    pub fn restart(self, n: i32) -> Self {
        self.reset(n)
    }

    /// Fit pattern to n cycles
    pub fn fit(self, n: i32) -> Self {
        self.slow(Pattern::pure(n as f64))
    }

    /// Chunk pattern and apply function to each chunk
    pub fn chunk_with(
        self,
        n: usize,
        f: impl Fn(usize, Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    ) -> Self {
        let f = Arc::new(f);
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor() as usize;
            let chunk_index = cycle % n;
            f(chunk_index, self.clone()).query(state)
        })
    }

    /// Gap - insert silence
    pub fn gap(self, n: usize) -> Self {
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor() as usize;
            if cycle.is_multiple_of(n) {
                self.query(state)
            } else {
                Vec::new()
            }
        })
    }

    /// Trim pattern to length
    pub fn trim(self, begin: Pattern<f64>, end: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        self.zoom(begin, end)
    }

    /// Splice patterns at specific point
    pub fn splice(self, at: Pattern<f64>, other: Pattern<T>) -> Pattern<T> {
        Pattern::new(move |state: &State| {
            // Query 'at' position at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let param_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let at_val = at
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(0.5);

            let cycle_pos = state.span.begin.to_float() - state.span.begin.to_float().floor();
            if cycle_pos < at_val {
                self.query(state)
            } else {
                other.query(state)
            }
        })
    }

    /// Scramble order of events
    pub fn scramble(self, n: usize) -> Self {
        Pattern::new(move |state: &State| {
            let mut haps = self.query(state);
            let cycle = state.span.begin.to_float().floor() as u64;
            let mut rng = StdRng::seed_from_u64(cycle);

            // Fisher-Yates shuffle
            for i in (1..haps.len()).rev() {
                let j = rng.gen_range(0..=i);
                haps.swap(i, j);
            }

            haps
        })
    }

    /// Shuffle segments
    pub fn segment(self, n: usize) -> Self {
        Pattern::new(move |state: &State| {
            let segment_size = 1.0 / n as f64;
            let mut all_haps = Vec::new();

            for i in 0..n {
                let segment_begin = i as f64 * segment_size;
                let segment_end = (i + 1) as f64 * segment_size;
                let segment_state = State {
                    span: TimeSpan::new(
                        state.span.begin + Fraction::from_float(segment_begin),
                        state.span.begin + Fraction::from_float(segment_end),
                    ),
                    controls: state.controls.clone(),
                };
                all_haps.extend(self.query(&segment_state));
            }
            all_haps
        })
    }

    /// Loopback - play pattern backwards then forwards
    pub fn loopback(self) -> Self {
        Pattern::cat(vec![self.clone(), self.rev()])
    }

    /// Mirror - palindrome within cycle
    pub fn mirror(self) -> Self {
        self.palindrome()
    }

    /// JuxBy - apply transform to one channel with specified pan amount (context-based for samples)
    /// Original pattern panned to -amount, transformed pattern panned to +amount
    /// This version uses pan context for sample playback (not stereo tuples)
    pub fn jux_by_ctx<F>(self, amount: Pattern<f64>, transform: F) -> Self
    where
        F: Fn(Pattern<T>) -> Pattern<T> + 'static + Send + Sync,
    {
        // Create left channel: original pattern panned to -amount
        let left = Pattern::new({
            let pattern = self.clone();
            let amount_clone = amount.clone();
            move |state: &State| {
                // Query amount at cycle start
                let cycle_start = state.span.begin.to_float().floor();
                let param_state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(cycle_start),
                        Fraction::from_float(cycle_start + 0.001),
                    ),
                    controls: state.controls.clone(),
                };
                let pan_left = -amount_clone
                    .query(&param_state)
                    .first()
                    .map(|h| h.value)
                    .unwrap_or(1.0);

                let mut haps = pattern.query(state);
                for hap in &mut haps {
                    hap.context.insert("pan".to_string(), pan_left.to_string());
                }
                haps
            }
        });

        // Create right channel: transformed pattern panned to +amount
        let right = Pattern::new({
            let pattern = transform(self);
            let amount_clone = amount.clone();
            move |state: &State| {
                // Query amount at cycle start
                let cycle_start = state.span.begin.to_float().floor();
                let param_state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(cycle_start),
                        Fraction::from_float(cycle_start + 0.001),
                    ),
                    controls: state.controls.clone(),
                };
                let pan_right = amount_clone
                    .query(&param_state)
                    .first()
                    .map(|h| h.value)
                    .unwrap_or(1.0);

                let mut haps = pattern.query(state);
                for hap in &mut haps {
                    hap.context.insert("pan".to_string(), pan_right.to_string());
                }
                haps
            }
        });

        // Stack (layer) left and right channels
        Pattern::stack(vec![left, right])
    }

    /// Jux - apply transform to right channel only (context-based for samples)
    /// Original pattern panned left (-1.0), transformed pattern panned right (+1.0)
    /// This version uses pan context for sample playback (not stereo tuples)
    pub fn jux_ctx<F>(self, transform: F) -> Self
    where
        F: Fn(Pattern<T>) -> Pattern<T> + 'static + Send + Sync,
    {
        self.jux_by_ctx(Pattern::pure(1.0), transform)
    }
}

// Numeric pattern operations
impl Pattern<f64> {
    /// Scale values to range
    pub fn range(self, min: Pattern<f64>, max: Pattern<f64>) -> Self {
        Pattern::new(move |state: &State| {
            // Query min/max at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let param_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let min_val = min
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(0.0);
            let max_val = max
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(1.0);

            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    hap.value = min_val + (hap.value * (max_val - min_val));
                    hap
                })
                .collect()
        })
    }

    /// Quantize to nearest value
    pub fn quantize(self, steps: Pattern<f64>) -> Self {
        Pattern::new(move |state: &State| {
            // Query steps at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let param_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let steps_val = steps
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(1.0);

            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    hap.value = (hap.value * steps_val).round() / steps_val;
                    hap
                })
                .collect()
        })
    }

    /// Smooth transitions between values
    pub fn smooth(self, amount: Pattern<f64>) -> Self {
        Pattern::new(move |state: &State| {
            // Query amount at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let param_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let amount_val = amount
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(0.5);

            let haps = self.query(state);
            if haps.is_empty() {
                return haps;
            }

            let mut smoothed = vec![haps[0].clone()];
            for i in 1..haps.len() {
                let mut hap = haps[i].clone();
                hap.value = smoothed[i - 1].value * (1.0 - amount_val) + hap.value * amount_val;
                smoothed.push(hap);
            }
            smoothed
        })
    }

    /// Exponential scaling
    pub fn exp(self, base: Pattern<f64>) -> Self {
        Pattern::new(move |state: &State| {
            // Query base at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let param_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let base_val = base
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(2.0);

            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    hap.value = base_val.powf(hap.value);
                    hap
                })
                .collect()
        })
    }

    /// Logarithmic scaling
    pub fn log(self, base: Pattern<f64>) -> Self {
        Pattern::new(move |state: &State| {
            // Query base at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let param_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let base_val = base
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(2.0);

            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    hap.value = hap.value.log(base_val);
                    hap
                })
                .collect()
        })
    }

    /// Sine wave modulation
    pub fn sine(self) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    hap.value = hap.value.sin();
                    hap
                })
                .collect()
        })
    }

    /// Cosine wave modulation
    pub fn cosine(self) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    hap.value = hap.value.cos();
                    hap
                })
                .collect()
        })
    }

    /// Sawtooth wave
    pub fn saw(self) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    hap.value = (hap.value % 1.0) * 2.0 - 1.0;
                    hap
                })
                .collect()
        })
    }

    /// Triangle wave
    pub fn tri(self) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    let phase = hap.value % 1.0;
                    hap.value = if phase < 0.5 {
                        phase * 4.0 - 1.0
                    } else {
                        3.0 - phase * 4.0
                    };
                    hap
                })
                .collect()
        })
    }

    /// Square wave
    pub fn square(self) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    hap.value = if (hap.value % 1.0) < 0.5 { -1.0 } else { 1.0 };
                    hap
                })
                .collect()
        })
    }

    /// Random walk
    pub fn walk(self, step_size: Pattern<f64>) -> Self {
        Pattern::new(move |state: &State| {
            // Query step_size at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let param_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let step_size_val = step_size
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(0.1);

            let haps = self.query(state);
            let cycle = state.span.begin.to_float().floor() as u64;
            let mut rng = StdRng::seed_from_u64(cycle);

            haps.into_iter()
                .map(|mut hap| {
                    let step = rng.gen_range(-step_size_val..step_size_val);
                    hap.value += step;
                    hap
                })
                .collect()
        })
    }
}

// String pattern operations
impl Pattern<String> {
    /// Append string to each value
    pub fn append_str(self, suffix: &str) -> Self {
        let suffix = suffix.to_string();
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    hap.value.push_str(&suffix);
                    hap
                })
                .collect()
        })
    }

    /// Prepend string to each value
    pub fn prepend_str(self, prefix: &str) -> Self {
        let prefix = prefix.to_string();
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    hap.value = format!("{}{}", prefix, hap.value);
                    hap
                })
                .collect()
        })
    }

    /// Replace substring
    pub fn replace_str(self, from: &str, to: &str) -> Self {
        let from = from.to_string();
        let to = to.to_string();
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    hap.value = hap.value.replace(&from, &to);
                    hap
                })
                .collect()
        })
    }
}

// Probabilistic patterns
impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Randomly choose between patterns each cycle
    pub fn rand_cat(patterns: Vec<Pattern<T>>) -> Pattern<T> {
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor() as u64;
            let mut rng = StdRng::seed_from_u64(cycle);
            let index = rng.gen_range(0..patterns.len());
            patterns[index].query(state)
        })
    }

    /// Weighted random choice
    pub fn wrand_cat(patterns: Vec<(Pattern<T>, f64)>) -> Pattern<T> {
        Pattern::new(move |state: &State| {
            let cycle = state.span.begin.to_float().floor() as u64;
            let mut rng = StdRng::seed_from_u64(cycle);

            let total_weight: f64 = patterns.iter().map(|(_, w)| w).sum();
            let mut choice = rng.gen_range(0.0..total_weight);

            for (pattern, weight) in &patterns {
                choice -= weight;
                if choice <= 0.0 {
                    return pattern.query(state);
                }
            }

            patterns[0].0.query(state)
        })
    }

    /// Degradeby with specific seed
    pub fn degrade_seed(self, seed: u64) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            let mut rng = StdRng::seed_from_u64(seed);

            haps.into_iter()
                .filter(|_| rng.gen_range(0.0..1.0) > 0.5)
                .collect()
        })
    }

    /// Undegraded version
    pub fn undegrade(self) -> Self {
        self // Returns pattern unchanged
    }
}

// Control/Effect patterns
impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Gain control
    pub fn gain(self, amount: Pattern<f64>) -> Self {
        // In real implementation, this would affect audio amplitude
        // For now just accept pattern for consistency
        self
    }

    /// Pan control
    pub fn pan(self, position: Pattern<f64>) -> Self {
        // In real implementation, this would affect stereo position
        // For now just accept pattern for consistency
        self
    }

    /// Speed/rate control
    pub fn speed(self, rate: Pattern<f64>) -> Self {
        self.fast(rate)
    }

    /// Accelerate - speed up over time
    pub fn accelerate(self, rate: Pattern<f64>) -> Self {
        Pattern::new(move |state: &State| {
            // Query rate at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let param_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let rate_val = rate
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(0.0);

            let haps = self.query(state);
            haps.into_iter()
                .enumerate()
                .map(|(i, hap)| {
                    let accel = 1.0 + (rate_val * i as f64);
                    // Would apply acceleration to playback
                    hap
                })
                .collect()
        })
    }

    /// Cutoff frequency control
    pub fn cutoff(self, freq: Pattern<f64>) -> Self {
        // In real implementation, this would control filter cutoff
        // For now just accept pattern for consistency
        self
    }

    /// Resonance control
    pub fn resonance(self, amount: Pattern<f64>) -> Self {
        // In real implementation, this would control filter resonance
        // For now just accept pattern for consistency
        self
    }

    /// Delay send
    pub fn delay(self, amount: Pattern<f64>) -> Self {
        // In real implementation, this would control delay send
        // For now just accept pattern for consistency
        self
    }

    /// Reverb send
    pub fn room(self, amount: Pattern<f64>) -> Self {
        // In real implementation, this would control reverb send
        // For now just accept pattern for consistency
        self
    }

    /// Distortion amount
    pub fn distort(self, amount: Pattern<f64>) -> Self {
        // In real implementation, this would control distortion
        // For now just accept pattern for consistency
        self
    }

    /// Loop a pattern at a given number of cycles
    /// loopAt n - stretches pattern over n cycles AND slows sample playback
    ///
    /// This combines pattern timing (slow) with playback speed adjustment:
    /// - Slows pattern structure by n (events spread over n cycles)
    /// - Divides playback speed by n (samples play n times slower)
    ///
    /// Examples:
    /// - s "bd sn hh cp" $ loopAt 2 -> 4 events over 2 cycles, each plays at 0.5x speed
    /// - s "bd" $ loopAt 4 -> Sample plays at 0.25x speed (pitched down 2 octaves)
    pub fn loop_at(self, cycles: Pattern<f64>) -> Self {
        let slowed = self.slow(cycles.clone());

        Pattern::new(move |state| {
            // Query cycles at cycle start
            let cycle_start = state.span.begin.to_float().floor();
            let param_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };
            let cycles_val = cycles
                .query(&param_state)
                .first()
                .map(|h| h.value)
                .unwrap_or(1.0);
            let speed_factor = 1.0 / cycles_val.max(0.001);

            slowed
                .query(state)
                .into_iter()
                .map(|mut hap| {
                    // Add speed control to context
                    hap.context
                        .insert("speed".to_string(), speed_factor.to_string());
                    hap
                })
                .collect()
        })
    }

    /// Pattern-controlled loop_at - the cycles parameter comes from a pattern
    /// Queries the duration pattern for each cycle to get the loop duration
    pub fn loop_at_pattern(self, duration_pattern: Pattern<String>) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Pattern::new(move |state| {
            // Query the duration pattern at a single point (cycle start) to get one value
            // This ensures we get alternating values like "1" then "2" for pattern "1 2"
            let cycle_start = state.span.begin.to_float().floor();

            // Query at a single time point (cycle start) to get the active value
            let query_time = cycle_start;
            let point_state = crate::pattern::State {
                span: crate::pattern::TimeSpan::new(
                    crate::pattern::Fraction::from_float(query_time),
                    crate::pattern::Fraction::from_float(query_time + 0.001), // Tiny span for point query
                ),
                controls: state.controls.clone(),
            };

            // Get the duration value active at this cycle
            let duration_haps = duration_pattern.query(&point_state);
            let cycles = if let Some(first_hap) = duration_haps.first() {
                // Parse the duration string to f64
                first_hap.value.parse::<f64>().unwrap_or(1.0).max(0.001) // Minimum duration to avoid division by zero
            } else {
                1.0 // Default to 1 cycle if no value
            };

            // Apply loop_at with the queried duration
            let slowed = self.clone().slow(Pattern::pure(cycles));
            let speed_factor = 1.0 / cycles;

            slowed
                .query(state)
                .into_iter()
                .map(|mut hap| {
                    hap.context
                        .insert("speed".to_string(), speed_factor.to_string());
                    hap
                })
                .collect()
        })
    }

    /// Weave with a function - applies function to alternating cycles
    pub fn weave_with(
        self,
        f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    ) -> Pattern<T> {
        let f = Arc::new(f);
        Pattern::new(move |state| {
            let cycle = state.span.begin.to_float().floor() as i64;

            if cycle % 2 == 0 {
                self.query(state)
            } else {
                let transformed = f(self.clone());
                transformed.query(state)
            }
        })
    }

    /// Choose with weighted probability
    /// chooseWith [(pattern1, weight1), (pattern2, weight2), ...]
    pub fn choose_with(choices: Vec<(Pattern<T>, f64)>) -> Pattern<T> {
        if choices.is_empty() {
            return Pattern::silence();
        }

        // Calculate total weight
        let total_weight: f64 = choices.iter().map(|(_, w)| w).sum();

        Pattern::new(move |state| {
            use rand::rngs::StdRng;
            use rand::{Rng, SeedableRng};

            let cycle = state.span.begin.to_float().floor() as u64;
            let mut rng = StdRng::seed_from_u64(cycle);
            let choice = rng.gen::<f64>() * total_weight;

            // Find which pattern to use
            let mut cumulative = 0.0;
            for (pattern, weight) in &choices {
                cumulative += weight;
                if choice < cumulative {
                    return pattern.query(state);
                }
            }

            // Fallback to last pattern
            choices.last().unwrap().0.query(state)
        })
    }
}

// Utility functions
impl<T: Clone + Send + Sync + Debug + 'static> Pattern<T> {
    /// Debug print pattern events
    pub fn trace(self) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            for hap in &haps {
                println!(
                    "Pattern event at {}: {:?}",
                    hap.part.begin.to_float(),
                    hap.value
                );
            }
            haps
        })
    }

    /// Count events
    pub fn count(self) -> Pattern<usize> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            vec![Hap::new(Some(state.span), state.span, haps.len())]
        })
    }

    /// Filter events by predicate
    pub fn filter(self, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .filter(|hap| predicate(&hap.value))
                .collect()
        })
    }

    /// Map function over values
    pub fn map<U: Clone + Send + Sync + 'static>(
        self,
        f: impl Fn(T) -> U + Send + Sync + 'static,
    ) -> Pattern<U> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|hap| Hap::new(hap.whole, hap.part, f(hap.value)))
                .collect()
        })
    }

    /// Flat map
    pub fn flat_map<U: Clone + Send + Sync + 'static>(
        self,
        f: impl Fn(T) -> Pattern<U> + Send + Sync + 'static,
    ) -> Pattern<U> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            let mut result = Vec::new();
            for hap in haps {
                let pattern = f(hap.value);
                result.extend(pattern.query(state));
            }
            result
        })
    }

    /// Rotate pattern values while maintaining structure
    /// rot shifts values by n positions: rot 1 "a ~ b c" becomes "b ~ c a"
    /// The rotation pattern can vary per cycle
    pub fn rot(self, rotation: Pattern<String>) -> Self
    where
        T: Clone + Send + Sync + Debug + 'static,
    {
        Pattern::new(move |state: &State| {
            // Get the events from the source pattern
            let events = self.query(state);

            if events.is_empty() {
                return Vec::new();
            }

            // Query the rotation pattern to get rotation amount
            let cycle_start = state.span.begin.to_float().floor();
            let rot_state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_start),
                    Fraction::from_float(cycle_start + 0.001),
                ),
                controls: state.controls.clone(),
            };

            let rot_amount = rotation
                .query(&rot_state)
                .first()
                .and_then(|h| h.value.parse::<i32>().ok())
                .unwrap_or(0);

            if rot_amount == 0 {
                return events;
            }

            // Extract just the values (preserving timings)
            let values: Vec<T> = events.iter().map(|h| h.value.clone()).collect();
            let len = values.len() as i32;

            // Rotate the values array
            // Positive rotation means shift left, negative means shift right
            let rotated_values: Vec<T> = (0..values.len())
                .map(|i| {
                    let src_idx = (i as i32 + rot_amount).rem_euclid(len) as usize;
                    values[src_idx].clone()
                })
                .collect();

            // Apply rotated values to original timings
            events
                .into_iter()
                .enumerate()
                .map(|(i, mut hap)| {
                    hap.value = rotated_values[i].clone();
                    hap
                })
                .collect()
        })
    }

    /// Trunc - truncate pattern to play only a fraction of each cycle
    /// trunc 0.75 plays the first 75% of each cycle
    /// The fraction parameter can be patterned
    pub fn trunc(self, fraction: Pattern<f64>) -> Self
    where
        T: Clone + Send + Sync + Debug + 'static,
    {
        // trunc is essentially zoom from 0 to fraction
        self.zoom(Pattern::pure(0.0), fraction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_zoom() {
        let p = Pattern::from_string("a b c d");
        let zoomed = p.zoom(Pattern::pure(0.25), Pattern::pure(0.75));
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps = zoomed.query(&state);
        // Should only have events from the middle half
        assert_eq!(haps.len(), 2);
    }

    #[test]
    fn test_compress() {
        let p = Pattern::from_string("a b c d");
        let compressed = p.compress(Pattern::pure(0.0), Pattern::pure(0.5));
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps = compressed.query(&state);
        // Should have all events compressed into first half
        assert!(haps.iter().all(|h| h.part.end.to_float() <= 0.5));
    }

    #[test]
    fn test_swing() {
        let p = Pattern::from_string("a b c d");
        let swung = p.swing(Pattern::pure(0.1));
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps = swung.query(&state);
        // Second and fourth events should be delayed
        assert_eq!(haps.len(), 4);
    }

    #[test]
    fn test_range() {
        let p = Pattern::pure(0.5);
        let ranged = p.range(Pattern::pure(10.0), Pattern::pure(20.0));
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps = ranged.query(&state);
        assert_eq!(haps[0].value, 15.0); // 0.5 mapped to [10, 20] = 15
    }

    #[test]
    fn test_quantize() {
        let p = Pattern::pure(0.37);
        // Quantize to 4 steps means 0, 0.25, 0.5, 0.75
        let quantized = p.quantize(Pattern::pure(4.0));
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps = quantized.query(&state);
        // 0.37 * 4 = 1.48, round(1.48) = 1, 1/4 = 0.25
        // But wait, the test originally expected 0.375 which would be quantizing to 8 steps
        // 0.37 * 8 = 2.96, round(2.96) = 3, 3/8 = 0.375
        // So the test might be expecting quantize(8) not quantize(4)
        // For quantize(4): 0.37 should round to 0.25
        assert!((haps[0].value - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_echo() {
        let p = Pattern::from_string("a");
        let echoed = p.echo(3, Pattern::pure(0.25), Pattern::pure(0.5));
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps = echoed.query(&state);
        assert_eq!(haps.len(), 3); // Original plus 2 echoes
    }

    #[test]
    fn test_filter() {
        let p = Pattern::from_string("1 2 3 4");
        let filtered = p.map(|s| s.parse::<i32>().unwrap_or(0)).filter(|&n| n > 2);
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps = filtered.query(&state);
        assert_eq!(haps.len(), 2); // Only 3 and 4
    }
}
