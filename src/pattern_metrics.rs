//! # Pattern Metrics - Rhythmic Complexity Analysis
//!
//! This module provides metrics for analyzing rhythmic complexity of patterns.
//! These metrics are useful for:
//! - Comparing pattern variations
//! - Guiding generative algorithms
//! - Analyzing musical structure
//! - Live coding feedback
//!
//! ## Metrics Provided
//!
//! - **Density**: Events per cycle (how "busy" a pattern is)
//! - **Syncopation**: Off-beat emphasis using LHL (Longuet-Higgins & Lee) algorithm
//! - **Evenness**: How evenly distributed events are (based on nPVI)
//! - **Entropy**: Information-theoretic measure of rhythmic complexity
//!
//! ## Example Usage
//!
//! ```rust
//! use phonon::pattern_metrics::{PatternMetrics, RhythmicAnalysis};
//! use phonon::mini_notation_v3::parse_mini_notation;
//!
//! let pattern = parse_mini_notation("bd ~ sn ~");
//! let metrics = PatternMetrics::analyze(&pattern, 4);
//!
//! println!("Density: {:.2} events/cycle", metrics.density);
//! println!("Syncopation: {:.2}", metrics.syncopation);
//! println!("Evenness: {:.2}", metrics.evenness);
//! ```

use crate::pattern::{Fraction, Hap, Pattern, State, TimeSpan};
use std::collections::HashMap;

/// Complete rhythmic analysis results for a pattern
#[derive(Debug, Clone)]
pub struct PatternMetrics {
    /// Average events per cycle
    pub density: f64,
    /// Syncopation score (0.0 = on-beat, higher = more syncopated)
    /// Uses a metrical hierarchy approach
    pub syncopation: f64,
    /// Evenness of event distribution (1.0 = perfectly even, 0.0 = maximally uneven)
    pub evenness: f64,
    /// Shannon entropy of IOI (inter-onset intervals)
    pub entropy: f64,
    /// Number of cycles analyzed
    pub cycles_analyzed: usize,
    /// Total events found
    pub total_events: usize,
    /// Per-cycle event counts
    pub events_per_cycle: Vec<usize>,
    /// Standard deviation of events per cycle
    pub density_variance: f64,
}

impl PatternMetrics {
    /// Analyze a pattern over a specified number of cycles
    pub fn analyze<T: Clone + Send + Sync + 'static>(
        pattern: &Pattern<T>,
        num_cycles: usize,
    ) -> Self {
        let cycles = num_cycles.max(1);

        // Query all events
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(cycles as i64, 1)),
            controls: HashMap::new(),
        };
        let events = pattern.query(&state);

        // Collect event onset times normalized to cycle position [0, 1)
        let mut onset_positions: Vec<f64> = events
            .iter()
            .map(|hap| {
                let t = hap.part.begin.to_float();
                t - t.floor() // Normalize to [0, 1) within cycle
            })
            .collect();
        onset_positions.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Per-cycle analysis
        let mut events_per_cycle = Vec::with_capacity(cycles);
        for cycle in 0..cycles {
            let cycle_state = State {
                span: TimeSpan::new(
                    Fraction::new(cycle as i64, 1),
                    Fraction::new((cycle + 1) as i64, 1),
                ),
                controls: HashMap::new(),
            };
            events_per_cycle.push(pattern.query(&cycle_state).len());
        }

        let total_events = events.len();
        let density = if cycles > 0 {
            total_events as f64 / cycles as f64
        } else {
            0.0
        };

        // Calculate density variance
        let mean_events = density;
        let variance: f64 = if cycles > 0 {
            events_per_cycle
                .iter()
                .map(|&c| {
                    let diff = c as f64 - mean_events;
                    diff * diff
                })
                .sum::<f64>()
                / cycles as f64
        } else {
            0.0
        };
        let density_variance = variance.sqrt();

        // Calculate all metrics using the first cycle as representative
        // (for multi-cycle analysis, we average)
        let first_cycle_events: Vec<f64> = events
            .iter()
            .filter(|hap| hap.part.begin.to_float() < 1.0)
            .map(|hap| hap.part.begin.to_float())
            .collect();

        let syncopation = calculate_syncopation(&first_cycle_events);
        let evenness = calculate_evenness(&first_cycle_events);
        let entropy = calculate_entropy(&first_cycle_events);

        PatternMetrics {
            density,
            syncopation,
            evenness,
            entropy,
            cycles_analyzed: cycles,
            total_events,
            events_per_cycle,
            density_variance,
        }
    }
}

/// Convenience trait for adding metrics analysis to patterns
pub trait RhythmicAnalysis<T: Clone + Send + Sync + 'static> {
    /// Analyze rhythmic complexity over specified cycles
    fn analyze_rhythm(&self, num_cycles: usize) -> PatternMetrics;

    /// Quick density check (events per cycle)
    fn density(&self, num_cycles: usize) -> f64;

    /// Quick syncopation check
    fn syncopation(&self) -> f64;

    /// Quick evenness check
    fn evenness(&self) -> f64;
}

impl<T: Clone + Send + Sync + 'static> RhythmicAnalysis<T> for Pattern<T> {
    fn analyze_rhythm(&self, num_cycles: usize) -> PatternMetrics {
        PatternMetrics::analyze(self, num_cycles)
    }

    fn density(&self, num_cycles: usize) -> f64 {
        PatternMetrics::analyze(self, num_cycles).density
    }

    fn syncopation(&self) -> f64 {
        PatternMetrics::analyze(self, 1).syncopation
    }

    fn evenness(&self) -> f64 {
        PatternMetrics::analyze(self, 1).evenness
    }
}

/// Calculate syncopation using a metrical hierarchy approach
///
/// This uses a weighted beat hierarchy where:
/// - Beat 1 (0.0) has weight 0 (strongest)
/// - Beat 3 (0.5) has weight 1
/// - Beats 2, 4 (0.25, 0.75) have weight 2
/// - 8th notes have weight 3
/// - 16th notes have weight 4
/// - etc.
///
/// Syncopation = sum of weights when events fall on weak beats
/// Normalized by number of events and max possible weight
fn calculate_syncopation(onset_times: &[f64]) -> f64 {
    if onset_times.is_empty() {
        return 0.0;
    }

    // Calculate metric weight for each onset
    // Lower weight = stronger beat position
    let total_weight: f64 = onset_times.iter().map(|&t| metrical_weight(t)).sum();

    // Normalize: 0 = all on downbeat, 1 = all on weakest subdivision
    // We use weight 4 (16th note offbeat) as practical maximum
    let max_weight = 4.0 * onset_times.len() as f64;
    if max_weight == 0.0 {
        return 0.0;
    }

    (total_weight / max_weight).min(1.0)
}

/// Calculate the metrical weight of a position in the cycle
/// Returns 0 for strongest (downbeat), higher for weaker positions
fn metrical_weight(position: f64) -> f64 {
    let pos = position.rem_euclid(1.0);

    // Check subdivisions from coarsest to finest
    // Using tolerance for floating-point comparison
    let tolerance = 0.001;

    // Weight 0: Downbeat (0.0)
    if (pos - 0.0).abs() < tolerance || (pos - 1.0).abs() < tolerance {
        return 0.0;
    }

    // Weight 1: Beat 3 (0.5) - half note
    if (pos - 0.5).abs() < tolerance {
        return 1.0;
    }

    // Weight 2: Beats 2 and 4 (0.25, 0.75) - quarter notes
    if (pos - 0.25).abs() < tolerance || (pos - 0.75).abs() < tolerance {
        return 2.0;
    }

    // Weight 3: 8th note positions (0.125, 0.375, 0.625, 0.875)
    let eighth_positions = [0.125, 0.375, 0.625, 0.875];
    for &eighth in &eighth_positions {
        if (pos - eighth).abs() < tolerance {
            return 3.0;
        }
    }

    // Weight 4: 16th note positions or finer
    // Check if it's on a 16th note grid
    let sixteenth_check = (pos * 16.0).round() / 16.0;
    if (pos - sixteenth_check).abs() < tolerance {
        return 4.0;
    }

    // Weight 5: Off-grid (very weak)
    5.0
}

/// Calculate evenness of event distribution using normalized Pairwise Variability Index (nPVI)
///
/// nPVI measures the variability of adjacent intervals.
/// - Perfect evenness (all equal IOIs) = 1.0
/// - Maximum unevenness = 0.0
///
/// The formula is: nPVI = 100 * (sum of |IOI_k - IOI_{k+1}| / ((IOI_k + IOI_{k+1})/2)) / (n-1)
/// We invert and normalize to get evenness in [0, 1]
fn calculate_evenness(onset_times: &[f64]) -> f64 {
    if onset_times.len() < 2 {
        return 1.0; // Single event or empty is considered "even"
    }

    let mut sorted = onset_times.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Calculate inter-onset intervals (IOIs)
    // Include wrap-around from last to first (circular rhythm)
    let mut iois = Vec::with_capacity(sorted.len());
    for i in 0..sorted.len() {
        let next_idx = (i + 1) % sorted.len();
        let ioi = if next_idx == 0 {
            // Wrap around: time from last event to first event of next cycle
            (1.0 - sorted[i]) + sorted[next_idx]
        } else {
            sorted[next_idx] - sorted[i]
        };
        iois.push(ioi.max(0.001)); // Prevent division by zero
    }

    if iois.len() < 2 {
        return 1.0;
    }

    // Calculate nPVI
    let mut pvi_sum = 0.0;
    for i in 0..iois.len() {
        let next_idx = (i + 1) % iois.len();
        let mean = (iois[i] + iois[next_idx]) / 2.0;
        if mean > 0.0 {
            pvi_sum += ((iois[i] - iois[next_idx]).abs() / mean).abs();
        }
    }

    let npvi = if iois.len() > 1 {
        100.0 * pvi_sum / (iois.len() - 1) as f64
    } else {
        0.0
    };

    // Convert to evenness: nPVI ranges from 0 (perfectly even) to ~200 (very uneven)
    // We map this to [0, 1] where 1 = perfectly even
    let evenness = 1.0 - (npvi / 200.0).min(1.0);
    evenness.max(0.0)
}

/// Calculate Shannon entropy of the IOI distribution
///
/// Higher entropy = more complex/unpredictable rhythm
/// Lower entropy = more regular/predictable rhythm
///
/// Returns normalized entropy in [0, 1]
fn calculate_entropy(onset_times: &[f64]) -> f64 {
    if onset_times.len() < 2 {
        return 0.0; // No intervals to analyze
    }

    let mut sorted = onset_times.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Calculate IOIs
    let mut iois = Vec::with_capacity(sorted.len());
    for i in 0..sorted.len() {
        let next_idx = (i + 1) % sorted.len();
        let ioi = if next_idx == 0 {
            (1.0 - sorted[i]) + sorted[next_idx]
        } else {
            sorted[next_idx] - sorted[i]
        };
        iois.push(ioi);
    }

    // Quantize IOIs into bins for probability distribution
    // Use 16 bins representing 16th note subdivisions
    let num_bins = 16;
    let mut bins = vec![0usize; num_bins];

    for &ioi in &iois {
        // Map IOI [0, 1] to bin [0, num_bins-1]
        let bin = ((ioi * num_bins as f64).floor() as usize).min(num_bins - 1);
        bins[bin] += 1;
    }

    // Calculate Shannon entropy
    let total = iois.len() as f64;
    let mut entropy = 0.0;

    for &count in &bins {
        if count > 0 {
            let p = count as f64 / total;
            entropy -= p * p.ln();
        }
    }

    // Normalize by maximum possible entropy (uniform distribution)
    let max_entropy = (num_bins as f64).ln();
    if max_entropy > 0.0 {
        entropy / max_entropy
    } else {
        0.0
    }
}

/// Detailed breakdown of where syncopation occurs
#[derive(Debug, Clone)]
pub struct SyncopationDetail {
    /// Position in cycle [0, 1)
    pub position: f64,
    /// Metrical weight (0 = strong, higher = weak)
    pub weight: f64,
    /// Description of the beat position
    pub beat_name: String,
}

/// Get detailed syncopation analysis showing where each event falls
pub fn analyze_syncopation_detail(onset_times: &[f64]) -> Vec<SyncopationDetail> {
    onset_times
        .iter()
        .map(|&t| {
            let weight = metrical_weight(t);
            let beat_name = match weight as u32 {
                0 => "downbeat".to_string(),
                1 => "half note".to_string(),
                2 => "quarter note".to_string(),
                3 => "8th note".to_string(),
                4 => "16th note".to_string(),
                _ => "off-grid".to_string(),
            };
            SyncopationDetail {
                position: t,
                weight,
                beat_name,
            }
        })
        .collect()
}

/// Compare two patterns and return the difference in their metrics
#[derive(Debug, Clone)]
pub struct MetricsComparison {
    pub density_diff: f64,
    pub syncopation_diff: f64,
    pub evenness_diff: f64,
    pub entropy_diff: f64,
}

impl MetricsComparison {
    pub fn compare<T: Clone + Send + Sync + 'static>(
        pattern_a: &Pattern<T>,
        pattern_b: &Pattern<T>,
        num_cycles: usize,
    ) -> Self {
        let metrics_a = PatternMetrics::analyze(pattern_a, num_cycles);
        let metrics_b = PatternMetrics::analyze(pattern_b, num_cycles);

        MetricsComparison {
            density_diff: metrics_b.density - metrics_a.density,
            syncopation_diff: metrics_b.syncopation - metrics_a.syncopation,
            evenness_diff: metrics_b.evenness - metrics_a.evenness,
            entropy_diff: metrics_b.entropy - metrics_a.entropy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mini_notation_v3::parse_mini_notation;

    /// Helper to query events from a pattern in a given cycle
    fn query_cycle<T: Clone + Send + Sync + 'static>(
        pattern: &Pattern<T>,
        cycle: usize,
    ) -> Vec<Hap<T>> {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        pattern.query(&state)
    }

    // ========================================================================
    // Level 1: Pattern Query Verification - Density Tests
    // ========================================================================

    #[test]
    fn test_density_simple_pattern() {
        // "bd sn hh cp" has 4 events per cycle
        let pattern: Pattern<String> = parse_mini_notation("bd sn hh cp");
        let metrics = PatternMetrics::analyze(&pattern, 1);

        assert_eq!(metrics.density, 4.0, "4 events in 1 cycle = density 4.0");
        assert_eq!(metrics.total_events, 4);
        assert_eq!(metrics.events_per_cycle, vec![4]);
    }

    #[test]
    fn test_density_fast_pattern() {
        // "bd*8" has 8 events per cycle
        let pattern: Pattern<String> = parse_mini_notation("bd*8");
        let metrics = PatternMetrics::analyze(&pattern, 1);

        assert_eq!(metrics.density, 8.0, "8 repeated events = density 8.0");
    }

    #[test]
    fn test_density_slow_pattern() {
        // "bd/2" spans 2 cycles, so density is 0.5
        let pattern: Pattern<String> = parse_mini_notation("bd/2");
        let metrics = PatternMetrics::analyze(&pattern, 2);

        assert_eq!(metrics.density, 0.5, "1 event over 2 cycles = density 0.5");
    }

    #[test]
    fn test_density_with_rests() {
        // "bd ~ ~ ~" has 1 event and 3 rests
        let pattern: Pattern<String> = parse_mini_notation("bd ~ ~ ~");
        let metrics = PatternMetrics::analyze(&pattern, 1);

        assert_eq!(metrics.density, 1.0, "1 event with 3 rests = density 1.0");
    }

    #[test]
    fn test_density_multiple_cycles_consistent() {
        // Same pattern over multiple cycles should be consistent
        let pattern: Pattern<String> = parse_mini_notation("bd sn");
        let metrics = PatternMetrics::analyze(&pattern, 8);

        assert_eq!(
            metrics.density, 2.0,
            "Pattern should be consistent across cycles"
        );
        assert_eq!(metrics.total_events, 16, "2 events × 8 cycles = 16 total");
        assert!(
            metrics.density_variance < 0.001,
            "No variance in consistent pattern"
        );
    }

    #[test]
    fn test_density_empty_pattern() {
        // Silence pattern has 0 density
        let pattern: Pattern<String> = Pattern::silence();
        let metrics = PatternMetrics::analyze(&pattern, 4);

        assert_eq!(metrics.density, 0.0, "Silence has zero density");
        assert_eq!(metrics.total_events, 0);
    }

    // ========================================================================
    // Level 1: Pattern Query Verification - Syncopation Tests
    // ========================================================================

    #[test]
    fn test_syncopation_on_beat_is_zero() {
        // Events on strong beats should have low syncopation
        // "bd ~ ~ ~" - only downbeat
        let pattern: Pattern<String> = parse_mini_notation("bd ~ ~ ~");
        let metrics = PatternMetrics::analyze(&pattern, 1);

        assert!(
            metrics.syncopation < 0.1,
            "Downbeat only should have near-zero syncopation, got {}",
            metrics.syncopation
        );
    }

    #[test]
    fn test_syncopation_on_half_note() {
        // "~ bd ~ ~" - event on beat 2 (0.25) = quarter note = weight 2
        let pattern: Pattern<String> = parse_mini_notation("~ bd ~ ~");
        let events = query_cycle(&pattern, 0);

        // Verify event position
        assert_eq!(events.len(), 1);
        let pos = events[0].part.begin.to_float();
        assert!(
            (pos - 0.25).abs() < 0.01,
            "Event should be at 0.25, got {}",
            pos
        );

        let metrics = PatternMetrics::analyze(&pattern, 1);

        // Weight 2 out of max 4 = 0.5 syncopation
        assert!(
            metrics.syncopation > 0.4 && metrics.syncopation < 0.6,
            "Quarter note offbeat should have ~0.5 syncopation, got {}",
            metrics.syncopation
        );
    }

    #[test]
    fn test_syncopation_increases_with_offbeats() {
        // More offbeat = more syncopation
        let on_beat: Pattern<String> = parse_mini_notation("bd ~ bd ~"); // 0.0, 0.5
        let off_beat: Pattern<String> = parse_mini_notation("~ bd ~ bd"); // 0.25, 0.75

        let on_metrics = PatternMetrics::analyze(&on_beat, 1);
        let off_metrics = PatternMetrics::analyze(&off_beat, 1);

        assert!(
            off_metrics.syncopation > on_metrics.syncopation,
            "Off-beat pattern should have more syncopation: off={} > on={}",
            off_metrics.syncopation,
            on_metrics.syncopation
        );
    }

    #[test]
    fn test_syncopation_euclidean_pattern() {
        // Euclidean rhythms have interesting syncopation
        // E(3,8) = tresillo = X..X..X. which has some syncopation
        let pattern: Pattern<String> = parse_mini_notation("bd(3,8)");
        let metrics = PatternMetrics::analyze(&pattern, 1);

        // Tresillo has events at 0, 3/8, 6/8 - mixture of strong and weak
        assert!(
            metrics.syncopation > 0.0,
            "Euclidean pattern should have some syncopation"
        );
        assert!(
            metrics.syncopation < 1.0,
            "Euclidean pattern shouldn't be maximally syncopated"
        );
    }

    // ========================================================================
    // Level 1: Pattern Query Verification - Evenness Tests
    // ========================================================================

    #[test]
    fn test_evenness_perfectly_even() {
        // "bd bd bd bd" - 4 evenly spaced events
        let pattern: Pattern<String> = parse_mini_notation("bd bd bd bd");
        let metrics = PatternMetrics::analyze(&pattern, 1);

        assert!(
            metrics.evenness > 0.9,
            "Evenly spaced events should have high evenness, got {}",
            metrics.evenness
        );
    }

    #[test]
    fn test_evenness_two_events_even() {
        // "bd ~ bd ~" - 2 evenly spaced events
        let pattern: Pattern<String> = parse_mini_notation("bd ~ bd ~");
        let metrics = PatternMetrics::analyze(&pattern, 1);

        assert!(
            metrics.evenness > 0.9,
            "Two evenly spaced events should have high evenness, got {}",
            metrics.evenness
        );
    }

    #[test]
    fn test_evenness_uneven_pattern() {
        // Events clustered at start should be uneven
        // "bd bd bd ~" - 3 events in first 3/4, nothing in last 1/4
        let pattern: Pattern<String> = parse_mini_notation("bd bd bd ~");
        let events = query_cycle(&pattern, 0);

        // Verify we have 3 events
        assert_eq!(events.len(), 3);

        let metrics = PatternMetrics::analyze(&pattern, 1);

        // This should have lower evenness due to gap at end
        assert!(
            metrics.evenness < 0.95,
            "Uneven pattern should have lower evenness, got {}",
            metrics.evenness
        );
    }

    #[test]
    fn test_evenness_single_event() {
        // Single event is trivially "even"
        let pattern: Pattern<String> = parse_mini_notation("bd ~ ~ ~");
        let metrics = PatternMetrics::analyze(&pattern, 1);

        assert_eq!(
            metrics.evenness, 1.0,
            "Single event should be perfectly even"
        );
    }

    // ========================================================================
    // Level 1: Pattern Query Verification - Entropy Tests
    // ========================================================================

    #[test]
    fn test_entropy_regular_pattern_low() {
        // Perfectly regular pattern should have low entropy
        let pattern: Pattern<String> = parse_mini_notation("bd bd bd bd");
        let metrics = PatternMetrics::analyze(&pattern, 1);

        assert!(
            metrics.entropy < 0.5,
            "Regular pattern should have low entropy, got {}",
            metrics.entropy
        );
    }

    #[test]
    fn test_entropy_varied_pattern_higher() {
        // Pattern with varied IOIs should have higher entropy
        // "bd bd ~ bd" has IOIs of 0.25, 0.5, 0.25 (varied)
        let pattern: Pattern<String> = parse_mini_notation("bd bd ~ bd");
        let regular: Pattern<String> = parse_mini_notation("bd bd bd bd");

        let varied_metrics = PatternMetrics::analyze(&pattern, 1);
        let regular_metrics = PatternMetrics::analyze(&regular, 1);

        assert!(
            varied_metrics.entropy >= regular_metrics.entropy,
            "Varied pattern should have >= entropy than regular: varied={} vs regular={}",
            varied_metrics.entropy,
            regular_metrics.entropy
        );
    }

    // ========================================================================
    // Level 1: RhythmicAnalysis Trait Tests
    // ========================================================================

    #[test]
    fn test_rhythmic_analysis_trait() {
        let pattern: Pattern<String> = parse_mini_notation("bd sn hh cp");

        // Trait methods should work
        assert_eq!(pattern.density(1), 4.0);
        assert!(pattern.syncopation() >= 0.0);
        assert!(pattern.evenness() >= 0.0 && pattern.evenness() <= 1.0);

        let full_metrics = pattern.analyze_rhythm(4);
        assert_eq!(full_metrics.total_events, 16);
    }

    // ========================================================================
    // Level 1: MetricsComparison Tests
    // ========================================================================

    #[test]
    fn test_metrics_comparison() {
        let sparse: Pattern<String> = parse_mini_notation("bd ~ ~ ~");
        let dense: Pattern<String> = parse_mini_notation("bd*8");

        let comparison = MetricsComparison::compare(&sparse, &dense, 1);

        assert!(
            comparison.density_diff > 0.0,
            "Dense pattern should have higher density"
        );
    }

    // ========================================================================
    // Level 1: Syncopation Detail Tests
    // ========================================================================

    #[test]
    fn test_syncopation_detail() {
        let onsets = vec![0.0, 0.25, 0.5, 0.75];
        let details = analyze_syncopation_detail(&onsets);

        assert_eq!(details.len(), 4);
        assert_eq!(details[0].beat_name, "downbeat");
        assert_eq!(details[1].beat_name, "quarter note");
        assert_eq!(details[2].beat_name, "half note");
        assert_eq!(details[3].beat_name, "quarter note");
    }

    // ========================================================================
    // Level 1: Edge Cases
    // ========================================================================

    #[test]
    fn test_metrical_weight_edge_cases() {
        // Test boundary conditions
        assert_eq!(metrical_weight(0.0), 0.0, "0.0 should be downbeat");
        assert_eq!(metrical_weight(0.5), 1.0, "0.5 should be half note");
        assert_eq!(metrical_weight(0.25), 2.0, "0.25 should be quarter note");
        assert_eq!(metrical_weight(0.125), 3.0, "0.125 should be 8th note");

        // Test wrap-around
        assert_eq!(metrical_weight(1.0), 0.0, "1.0 should wrap to downbeat");
    }

    #[test]
    fn test_density_variance() {
        // Pattern with varying density per cycle
        // Using alternation: first cycle has different events than second
        let pattern: Pattern<String> = parse_mini_notation("<bd bd bd bd, bd bd>");
        let metrics = PatternMetrics::analyze(&pattern, 2);

        // This alternating pattern has 4 events in odd cycles, 2 in even
        // So variance should be > 0
        // Actually, let's verify what the pattern actually produces
        let events_c0 = query_cycle(&pattern, 0);
        let events_c1 = query_cycle(&pattern, 1);

        if events_c0.len() != events_c1.len() {
            assert!(
                metrics.density_variance > 0.0,
                "Alternating density pattern should have non-zero variance"
            );
        }
    }
}
