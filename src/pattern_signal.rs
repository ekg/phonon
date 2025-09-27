//! Signal and Continuous Pattern Operations
//!
//! Implements continuous patterns, random generators, and signal processing

use crate::pattern::{Fraction, Hap, Pattern, State, TimeSpan};
use rand::distributions::{Distribution, WeightedIndex};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Create a continuous signal pattern from a function
pub fn signal<T: Clone + Send + Sync + 'static>(
    f: impl Fn(f64) -> T + Send + Sync + 'static,
) -> Pattern<T> {
    Pattern::new(move |state: &State| {
        // Sample the signal at regular intervals
        let begin = state.span.begin.to_float();
        let end = state.span.end.to_float();
        let samples = ((end - begin) * 32.0).ceil() as usize; // 32 samples per cycle

        let mut haps = Vec::new();
        for i in 0..samples {
            let t = begin + (end - begin) * (i as f64) / (samples as f64);
            let next_t = begin + (end - begin) * ((i + 1) as f64) / (samples as f64);

            let value = f(t);
            haps.push(Hap::new(
                Some(state.span),
                TimeSpan::new(Fraction::from_float(t), Fraction::from_float(next_t)),
                value,
            ));
        }
        haps
    })
}

/// Sine wave signal (0 to 1)
pub fn sine() -> Pattern<f64> {
    signal(|t| (t * std::f64::consts::TAU).sin() * 0.5 + 0.5)
}

/// Cosine wave signal (0 to 1)
pub fn cosine() -> Pattern<f64> {
    signal(|t| (t * std::f64::consts::TAU).cos() * 0.5 + 0.5)
}

/// Sawtooth wave signal (0 to 1)
pub fn saw() -> Pattern<f64> {
    signal(|t| t % 1.0)
}

/// Inverse sawtooth wave signal (1 to 0)
pub fn isaw() -> Pattern<f64> {
    signal(|t| 1.0 - (t % 1.0))
}

/// Triangle wave signal (0 to 1)
pub fn tri() -> Pattern<f64> {
    signal(|t| {
        let phase = t % 1.0;
        if phase < 0.5 {
            phase * 2.0
        } else {
            2.0 - phase * 2.0
        }
    })
}

/// Square wave signal (0 or 1)
pub fn square() -> Pattern<f64> {
    signal(|t| if (t % 1.0) < 0.5 { 0.0 } else { 1.0 })
}

/// Perlin noise pattern
pub fn perlin() -> Pattern<f64> {
    Pattern::new(move |state: &State| {
        let begin = state.span.begin.to_float();
        let end = state.span.end.to_float();
        let samples = ((end - begin) * 32.0).ceil() as usize;

        let mut haps = Vec::new();
        for i in 0..samples {
            let t = begin + (end - begin) * (i as f64) / (samples as f64);
            let next_t = begin + (end - begin) * ((i + 1) as f64) / (samples as f64);

            // Simple Perlin-like noise using multiple sine waves
            let value = (0..5)
                .map(|octave| {
                    let freq = 2.0_f64.powi(octave);
                    let amp = 0.5_f64.powi(octave);
                    (t * freq * std::f64::consts::TAU).sin() * amp
                })
                .sum::<f64>()
                * 0.5
                + 0.5;

            haps.push(Hap::new(
                Some(state.span),
                TimeSpan::new(Fraction::from_float(t), Fraction::from_float(next_t)),
                value.max(0.0).min(1.0),
            ));
        }
        haps
    })
}

/// Random value pattern (0 to 1)
pub fn rand() -> Pattern<f64> {
    Pattern::new(move |state: &State| {
        let cycle = state.span.begin.to_float().floor() as u64;
        let mut rng = StdRng::seed_from_u64(cycle);

        vec![Hap::new(
            Some(state.span),
            state.span,
            rng.gen_range(0.0..1.0),
        )]
    })
}

/// Integer random pattern
pub fn irand(max: i32) -> Pattern<i32> {
    Pattern::new(move |state: &State| {
        let cycle = state.span.begin.to_float().floor() as u64;
        let mut rng = StdRng::seed_from_u64(cycle);

        vec![Hap::new(
            Some(state.span),
            state.span,
            rng.gen_range(0..max),
        )]
    })
}

/// Choose randomly from a list
pub fn choose<T: Clone + Send + Sync + 'static>(choices: Vec<T>) -> Pattern<T> {
    Pattern::new(move |state: &State| {
        if choices.is_empty() {
            return Vec::new();
        }

        let cycle = state.span.begin.to_float().floor() as u64;
        let mut rng = StdRng::seed_from_u64(cycle);
        let index = rng.gen_range(0..choices.len());

        vec![Hap::new(
            Some(state.span),
            state.span,
            choices[index].clone(),
        )]
    })
}

/// Weighted choose from a list
pub fn wchoose<T: Clone + Send + Sync + 'static>(choices: Vec<(T, f64)>) -> Pattern<T> {
    Pattern::new(move |state: &State| {
        if choices.is_empty() {
            return Vec::new();
        }

        let cycle = state.span.begin.to_float().floor() as u64;
        let mut rng = StdRng::seed_from_u64(cycle);

        let weights: Vec<f64> = choices.iter().map(|(_, w)| *w).collect();
        let dist = WeightedIndex::new(&weights).unwrap();
        let index = dist.sample(&mut rng);

        vec![Hap::new(
            Some(state.span),
            state.span,
            choices[index].0.clone(),
        )]
    })
}

/// Pick values from a pattern
pub fn pick<T: Clone + Send + Sync + 'static>(
    pattern: Pattern<T>,
    indices: Pattern<usize>,
) -> Pattern<T> {
    Pattern::new(move |state: &State| {
        let value_haps = pattern.query(state);
        let index_haps = indices.query(state);

        let mut result = Vec::new();
        for index_hap in index_haps {
            if let Some(value_hap) = value_haps.get(index_hap.value % value_haps.len()) {
                result.push(Hap::new(
                    index_hap.whole,
                    index_hap.part,
                    value_hap.value.clone(),
                ));
            }
        }
        result
    })
}

/// Continuous envelope pattern
pub fn envelope(
    points: Vec<(f64, f64)>, // (time, value) pairs
    loop_duration: f64,
) -> Pattern<f64> {
    Pattern::new(move |state: &State| {
        if points.is_empty() {
            return Vec::new();
        }

        let begin = state.span.begin.to_float();
        let end = state.span.end.to_float();
        let samples = ((end - begin) * 32.0).ceil() as usize;

        let mut haps = Vec::new();
        for i in 0..samples {
            let t = begin + (end - begin) * (i as f64) / (samples as f64);
            let next_t = begin + (end - begin) * ((i + 1) as f64) / (samples as f64);

            // Find position in envelope
            let env_t = (t % loop_duration) / loop_duration;

            // Linear interpolation between points
            let mut value = points[0].1;
            for window in points.windows(2) {
                let (t1, v1) = window[0];
                let (t2, v2) = window[1];

                if env_t >= t1 && env_t <= t2 {
                    let factor = (env_t - t1) / (t2 - t1);
                    value = v1 + (v2 - v1) * factor;
                    break;
                }
            }

            haps.push(Hap::new(
                Some(state.span),
                TimeSpan::new(Fraction::from_float(t), Fraction::from_float(next_t)),
                value,
            ));
        }
        haps
    })
}

/// Run pattern (sample and hold)
pub fn run(n: usize) -> Pattern<usize> {
    Pattern::new(move |state: &State| {
        let mut haps = Vec::new();
        let duration = state.span.duration();
        let step = duration / Fraction::new(n as i64, 1);

        for i in 0..n {
            let begin = state.span.begin + step * Fraction::new(i as i64, 1);
            let end = begin + step;

            if end > state.span.end {
                break;
            }

            haps.push(Hap::new(
                Some(state.span),
                TimeSpan::new(begin, end),
                i,
            ));
        }
        haps
    })
}

/// Scan through values
pub fn scan(n: usize) -> Pattern<f64> {
    Pattern::new(move |state: &State| {
        let cycle = state.span.begin.to_float();
        let value = (cycle % n as f64) / n as f64;

        vec![Hap::new(
            Some(state.span),
            state.span,
            value,
        )]
    })
}

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Sample pattern at specific rate
    pub fn sample(self, rate: f64) -> Self {
        Pattern::new(move |state: &State| {
            let begin = state.span.begin.to_float();
            let end = state.span.end.to_float();
            let samples = ((end - begin) * rate).ceil() as usize;

            let mut result = Vec::new();
            for i in 0..samples {
                let t = begin + (end - begin) * (i as f64) / (samples as f64);
                let sample_state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(t),
                        Fraction::from_float(t + 1.0 / rate),
                    ),
                    controls: state.controls.clone(),
                };

                if let Some(hap) = self.query(&sample_state).first() {
                    result.push(hap.clone());
                }
            }
            result
        })
    }

    /// Smooth pattern values over time
    pub fn smoothe(self, amount: f64) -> Pattern<f64>
    where
        T: Into<f64> + Clone,
    {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            if haps.is_empty() {
                return Vec::new();
            }

            let mut result = Vec::new();
            let mut prev_value: Option<f64> = None;

            for hap in haps {
                let current: f64 = hap.value.clone().into();
                let smoothed = if let Some(prev) = prev_value {
                    prev * (1.0 - amount) + current * amount
                } else {
                    current
                };

                prev_value = Some(smoothed);
                result.push(Hap::new(hap.whole, hap.part, smoothed));
            }
            result
        })
    }
}

/// Continuous random walk
pub fn randwalk(step_size: f64, initial: f64) -> Pattern<f64> {
    Pattern::new(move |state: &State| {
        let begin = state.span.begin.to_float();
        let end = state.span.end.to_float();
        let steps = ((end - begin) * 16.0).ceil() as usize;

        let mut value = initial;
        let mut haps = Vec::new();

        for i in 0..steps {
            let t = begin + (end - begin) * (i as f64) / (steps as f64);
            let next_t = begin + (end - begin) * ((i + 1) as f64) / (steps as f64);

            // Use time as seed for deterministic walk
            let seed = (t * 1000000.0) as u64;
            let mut rng = StdRng::seed_from_u64(seed);

            value += rng.gen_range(-step_size..step_size);
            value = value.max(0.0).min(1.0);

            haps.push(Hap::new(
                Some(state.span),
                TimeSpan::new(Fraction::from_float(t), Fraction::from_float(next_t)),
                value,
            ));
        }
        haps
    })
}

/// Pink noise pattern
pub fn pink() -> Pattern<f64> {
    Pattern::new(move |state: &State| {
        let begin = state.span.begin.to_float();
        let end = state.span.end.to_float();
        let samples = ((end - begin) * 32.0).ceil() as usize;

        let mut haps = Vec::new();
        let mut b0 = 0.0;
        let mut b1 = 0.0;
        let mut b2 = 0.0;

        for i in 0..samples {
            let t = begin + (end - begin) * (i as f64) / (samples as f64);
            let next_t = begin + (end - begin) * ((i + 1) as f64) / (samples as f64);

            let seed = (t * 1000000.0) as u64;
            let mut rng = StdRng::seed_from_u64(seed);
            let white = rng.gen_range(-1.0..1.0);

            // Paul Kellet's economy pink noise filter
            b0 = 0.99765 * b0 + white * 0.0990460;
            b1 = 0.96300 * b1 + white * 0.2965164;
            b2 = 0.57000 * b2 + white * 1.0526913;
            let pink = (b0 + b1 + b2 + white * 0.1848) * 0.25;

            haps.push(Hap::new(
                Some(state.span),
                TimeSpan::new(Fraction::from_float(t), Fraction::from_float(next_t)),
                pink * 0.5 + 0.5, // Normalize to 0-1
            ));
        }
        haps
    })
}

/// Brown noise pattern
pub fn brown() -> Pattern<f64> {
    Pattern::new(move |state: &State| {
        let begin = state.span.begin.to_float();
        let end = state.span.end.to_float();
        let samples = ((end - begin) * 32.0).ceil() as usize;

        let mut haps = Vec::new();
        let mut brown = 0.0;

        for i in 0..samples {
            let t = begin + (end - begin) * (i as f64) / (samples as f64);
            let next_t = begin + (end - begin) * ((i + 1) as f64) / (samples as f64);

            let seed = (t * 1000000.0) as u64;
            let mut rng = StdRng::seed_from_u64(seed);
            let white = rng.gen_range(-1.0..1.0);

            // Integrate white noise for brown noise
            brown += white * 0.02;
            brown = f64::max(brown, -1.0).min(1.0);

            haps.push(Hap::new(
                Some(state.span),
                TimeSpan::new(Fraction::from_float(t), Fraction::from_float(next_t)),
                brown * 0.5 + 0.5, // Normalize to 0-1
            ));
        }
        haps
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_signal_patterns() {
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        // Test sine wave
        let sine_pattern = sine();
        let sine_haps = sine_pattern.query(&state);
        assert!(!sine_haps.is_empty());
        assert!(sine_haps.iter().all(|h| h.value >= 0.0 && h.value <= 1.0));

        // Test random
        let rand_pattern = rand();
        let rand_haps = rand_pattern.query(&state);
        assert_eq!(rand_haps.len(), 1);
        assert!(rand_haps[0].value >= 0.0 && rand_haps[0].value <= 1.0);

        // Test choose
        let choose_pattern = choose(vec!["a", "b", "c"]);
        let choose_haps = choose_pattern.query(&state);
        assert_eq!(choose_haps.len(), 1);
        assert!(["a", "b", "c"].contains(&choose_haps[0].value));
    }

    #[test]
    fn test_envelope() {
        let env = envelope(vec![(0.0, 0.0), (0.25, 1.0), (0.75, 0.5), (1.0, 0.0)], 1.0);

        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        let haps = env.query(&state);
        assert!(!haps.is_empty());

        // Check first value is near 0
        assert!(haps.first().unwrap().value < 0.1);
    }
}
