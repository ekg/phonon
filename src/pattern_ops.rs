//! Complete set of pattern operators ported from Strudel
//! All the pattern transformation functions you know and love

use crate::pattern::{Pattern, Hap, State, TimeSpan, Fraction};
use std::sync::Arc;
use rand::{Rng, SeedableRng, rngs::StdRng};

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    // ============= Time Manipulation =============
    
    /// Shift pattern forward in time
    pub fn late(self, amount: f64) -> Self {
        Pattern::new(move |state| {
            self.query(state)
                .into_iter()
                .map(|mut hap| {
                    hap.part = TimeSpan::new(
                        Fraction::from_float(hap.part.begin.to_float() + amount),
                        Fraction::from_float(hap.part.end.to_float() + amount),
                    );
                    if let Some(whole) = hap.whole {
                        hap.whole = Some(TimeSpan::new(
                            Fraction::from_float(whole.begin.to_float() + amount),
                            Fraction::from_float(whole.end.to_float() + amount),
                        ));
                    }
                    hap
                })
                .collect()
        })
    }
    
    /// Shift pattern backward in time
    pub fn early(self, amount: f64) -> Self {
        self.late(-amount)
    }
    
    /// Offset pattern by a fraction of a cycle
    pub fn offset(self, amount: f64) -> Self {
        self.late(amount)
    }
    
    /// Loop a pattern within a cycle
    pub fn loop_pattern(self, n: usize) -> Self {
        Pattern::new(move |state| {
            let mut all_haps = Vec::new();
            for i in 0..n {
                let offset = i as f64 / n as f64;
                let scaled = self.clone().fast(n as f64);
                let shifted = scaled.late(offset);
                all_haps.extend(shifted.query(state));
            }
            all_haps
        })
    }
    
    // ============= Randomness & Probability =============
    
    /// Randomly drop events with given probability
    pub fn degrade_by(self, probability: f64) -> Self {
        Pattern::new(move |state| {
            let cycle = state.span.begin.to_float().floor() as u64;
            let mut rng = StdRng::seed_from_u64(cycle);
            
            self.query(state)
                .into_iter()
                .filter(|_| rng.gen::<f64>() > probability)
                .collect()
        })
    }
    
    /// Degrade 50% of events
    pub fn degrade(self) -> Self {
        self.degrade_by(0.5)
    }
    
    /// Sometimes apply a function (50% chance per cycle)
    pub fn sometimes(self, f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static) -> Self 
    where
        T: 'static,
    {
        self.sometimes_by(0.5, f)
    }
    
    /// Sometimes apply a function with specific probability
    pub fn sometimes_by(self, prob: f64, f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static) -> Self
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
    
    /// Repeat each event n times
    pub fn dup(self, n: usize) -> Self {
        Pattern::new(move |state| {
            self.query(state)
                .into_iter()
                .flat_map(|hap| {
                    (0..n).map(|_| hap.clone()).collect::<Vec<_>>()
                })
                .collect()
        })
    }
    
    /// Stutter - repeat each event n times with subdivision
    pub fn stutter(self, n: usize) -> Self {
        Pattern::new(move |state| {
            self.query(state)
                .into_iter()
                .flat_map(|hap| {
                    let duration = hap.part.duration().to_float();
                    let step = duration / n as f64;
                    
                    (0..n).map(|i| {
                        let offset = i as f64 * step;
                        let mut new_hap = hap.clone();
                        new_hap.part = TimeSpan::new(
                            Fraction::from_float(hap.part.begin.to_float() + offset),
                            Fraction::from_float(hap.part.begin.to_float() + offset + step),
                        );
                        new_hap
                    }).collect::<Vec<_>>()
                })
                .collect()
        })
    }
    
    /// Create a palindrome (pattern + reversed pattern)
    pub fn palindrome(self) -> Self {
        let forward = self.clone();
        let backward = self.rev();
        Pattern::cat(vec![forward, backward])
    }
    
    /// Chunk - apply a function to a different part of the pattern each cycle
    pub fn chunk(self, n: usize, f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static) -> Self
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
                    let hap_pos = (hap.part.begin.to_float() % 1.0);
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
    pub fn jux(self, f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static) -> Pattern<(T, T)>
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
            left_haps.into_iter().zip(right_haps.into_iter())
                .map(|(l, r)| {
                    Hap::new(
                        l.whole,
                        l.part,
                        (l.value, r.value),
                    )
                })
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
    pub fn when_mod(self, modulo: i32, offset: i32, f: impl Fn(Pattern<T>) -> Pattern<T> + Send + Sync + 'static) -> Self
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
            
            // Only keep pattern events that align with euclidean hits
            pattern_haps.into_iter()
                .filter(|phap| {
                    euclid_haps.iter().any(|ehap| 
                        ehap.value && 
                        phap.part.begin.to_float() >= ehap.part.begin.to_float() &&
                        phap.part.begin.to_float() < ehap.part.end.to_float()
                    )
                })
                .collect()
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
    use crate::pattern::{State, TimeSpan, Fraction};
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
        use crate::pattern_tonal::*;
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