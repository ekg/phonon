//! Pattern Query and Analysis Operations
//! 
//! Implements pattern introspection, query, and analysis functions

use crate::pattern::{Pattern, State, TimeSpan, Fraction, Hap};
use std::sync::Arc;
use std::fmt::Debug;

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Get the first cycle of a pattern
    pub fn first_cycle(self) -> Vec<Hap<T>> {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(0, 1),
                Fraction::new(1, 1),
            ),
            controls: std::collections::HashMap::new(),
        };
        self.query(&state)
    }
    
    /// Query a specific time arc
    pub fn query_arc(self, begin: f64, end: f64) -> Vec<Hap<T>> {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(begin),
                Fraction::from_float(end),
            ),
            controls: std::collections::HashMap::new(),
        };
        self.query(&state)
    }
    
    /// Split query into multiple smaller queries
    pub fn split_queries(self, n: usize) -> Pattern<Vec<Hap<T>>> {
        Pattern::new(move |state: &State| {
            let duration = state.span.duration();
            let step = duration / Fraction::new(n as i64, 1);
            
            let mut all_haps = Vec::new();
            for i in 0..n {
                let begin = state.span.begin + step * Fraction::new(i as i64, 1);
                let end = begin + step;
                
                let sub_state = State {
                    span: TimeSpan::new(begin, end),
                    controls: state.controls.clone(),
                };
                
                let haps = self.query(&sub_state);
                all_haps.push(Hap::new(
                    Some(TimeSpan::new(begin, end)),
                    TimeSpan::new(begin, end),
                    haps,
                ));
            }
            all_haps
        })
    }
    
    /// Process haps directly
    pub fn with_haps<F>(self, f: F) -> Self
    where
        F: Fn(Vec<Hap<T>>) -> Vec<Hap<T>> + Send + Sync + 'static,
    {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            f(haps)
        })
    }
    
    /// Process single hap
    pub fn with_hap<F>(self, f: F) -> Self
    where
        F: Fn(Hap<T>) -> Hap<T> + Send + Sync + 'static,
    {
        let f = Arc::new(f);
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            let f = f.clone();
            haps.into_iter().map(move |h| f(h)).collect()
        })
    }
    
    /// Process values only
    pub fn with_value<F, U>(self, f: F) -> Pattern<U>
    where
        F: Fn(T) -> U + Send + Sync + 'static,
        U: Clone + Send + Sync + 'static,
    {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter().map(|hap| {
                Hap::new(
                    hap.whole,
                    hap.part,
                    f(hap.value),
                )
            }).collect()
        })
    }
    
    /// Add context to pattern
    pub fn with_context<C: Clone + Send + Sync + 'static>(
        self,
        context: C
    ) -> Pattern<(T, C)> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter().map(|hap| {
                Hap::new(
                    hap.whole,
                    hap.part,
                    (hap.value, context.clone()),
                )
            }).collect()
        })
    }
    
    /// Focus on specific events
    pub fn focus_on<F>(self, predicate: F) -> Self
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter().filter(|hap| predicate(&hap.value)).collect()
        })
    }
    
    /// Get pattern metadata
    pub fn metadata(self) -> Pattern<PatternInfo<T>> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            
            vec![Hap::new(
                Some(state.span.clone()),
                state.span.clone(),
                PatternInfo {
                    event_count: haps.len(),
                    span: state.span.clone(),
                    events: haps,
                },
            )]
        })
    }
    
    /// Analyze pattern density
    pub fn density_analysis(self) -> Pattern<f64> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            let duration = state.span.duration().to_float();
            let density = haps.len() as f64 / duration;
            
            vec![Hap::new(
                Some(state.span.clone()),
                state.span.clone(),
                density,
            )]
        })
    }
    
    /// Get unique values in pattern
    pub fn unique(self) -> Pattern<Vec<T>>
    where
        T: PartialEq,
    {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            let mut unique_values = Vec::new();
            
            for hap in haps {
                if !unique_values.iter().any(|v| v == &hap.value) {
                    unique_values.push(hap.value);
                }
            }
            
            vec![Hap::new(
                Some(state.span.clone()),
                state.span.clone(),
                unique_values,
            )]
        })
    }
    
    /// Segment pattern by value changes
    pub fn segment_by_change(self) -> Pattern<Vec<T>>
    where
        T: PartialEq,
    {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            let mut segments = Vec::new();
            let mut current_segment = Vec::new();
            let mut last_value: Option<T> = None;
            
            for hap in haps {
                if let Some(ref last) = last_value {
                    if last != &hap.value && !current_segment.is_empty() {
                        segments.push(Hap::new(
                            Some(state.span.clone()),
                            state.span.clone(),
                            current_segment.clone(),
                        ));
                        current_segment.clear();
                    }
                }
                current_segment.push(hap.value.clone());
                last_value = Some(hap.value);
            }
            
            if !current_segment.is_empty() {
                segments.push(Hap::new(
                    Some(state.span.clone()),
                    state.span.clone(),
                    current_segment,
                ));
            }
            
            segments
        })
    }
}

/// Pattern information structure
#[derive(Clone)]
pub struct PatternInfo<T> {
    pub event_count: usize,
    pub span: TimeSpan,
    pub events: Vec<Hap<T>>,
}

/// Pattern visualization
impl<T: Clone + Send + Sync + Debug + 'static> Pattern<T> {
    /// Show pattern information
    pub fn show(self) -> String {
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: std::collections::HashMap::new(),
        };
        
        let haps = self.query(&state);
        let mut output = format!("Pattern with {} events in first cycle:\n", haps.len());
        
        for hap in haps.iter().take(10) {
            output.push_str(&format!(
                "  [{:.3}-{:.3}]: {:?}\n",
                hap.part.begin.to_float(),
                hap.part.end.to_float(),
                hap.value
            ));
        }
        
        if haps.len() > 10 {
            output.push_str(&format!("  ... and {} more events\n", haps.len() - 10));
        }
        
        output
    }
    
    /// Draw ASCII line visualization
    pub fn draw_line(self) -> String {
        self.draw_line_sz(40)
    }
    
    /// Draw ASCII line visualization with size
    pub fn draw_line_sz(self, width: usize) -> String {
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: std::collections::HashMap::new(),
        };
        
        let haps = self.query(&state);
        let mut line = vec!['.'; width];
        
        for hap in haps {
            let begin = (hap.part.begin.to_float() * width as f64) as usize;
            let end = (hap.part.end.to_float() * width as f64) as usize;
            
            for i in begin..end.min(width) {
                line[i] = '#';
            }
        }
        
        line.iter().collect()
    }
    
    /// Create pattern visualization data
    pub fn visualize(self, cycles: f64) -> Vec<VisualizationEvent> {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(0, 1),
                Fraction::from_float(cycles),
            ),
            controls: std::collections::HashMap::new(),
        };
        
        let haps = self.query(&state);
        haps.into_iter().map(|hap| {
            VisualizationEvent {
                begin: hap.part.begin.to_float(),
                end: hap.part.end.to_float(),
                value: format!("{:?}", hap.value),
            }
        }).collect()
    }
}

/// Visualization event data
#[derive(Debug, Clone)]
pub struct VisualizationEvent {
    pub begin: f64,
    pub end: f64,
    pub value: String,
}

/// Pattern statistics
impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Count events per cycle
    pub fn event_count(self, cycles: f64) -> usize {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(0, 1),
                Fraction::from_float(cycles),
            ),
            controls: std::collections::HashMap::new(),
        };
        self.query(&state).len()
    }
    
    /// Get pattern duration statistics
    pub fn duration_stats(self) -> (f64, f64, f64) {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(0, 1),
                Fraction::new(1, 1),
            ),
            controls: std::collections::HashMap::new(),
        };
        
        let haps = self.query(&state);
        if haps.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        
        let durations: Vec<f64> = haps.iter()
            .map(|h| h.part.duration().to_float())
            .collect();
        
        let min = durations.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = durations.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let avg = durations.iter().sum::<f64>() / durations.len() as f64;
        
        (min, max, avg)
    }
}

/// Pattern comparison
impl<T: Clone + Send + Sync + PartialEq + 'static> Pattern<T> {
    /// Check if patterns are equivalent
    pub fn equivalent_to(self, other: Pattern<T>, cycles: f64) -> bool {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(0, 1),
                Fraction::from_float(cycles),
            ),
            controls: std::collections::HashMap::new(),
        };
        
        let haps1 = self.query(&state);
        let haps2 = other.query(&state);
        
        if haps1.len() != haps2.len() {
            return false;
        }
        
        for (h1, h2) in haps1.iter().zip(haps2.iter()) {
            if h1.value != h2.value || 
               h1.part.begin != h2.part.begin ||
               h1.part.end != h2.part.end {
                return false;
            }
        }
        
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Pattern;
    
    #[test]
    fn test_first_cycle() {
        let p = Pattern::from_string("a b c d");
        let haps = p.first_cycle();
        assert_eq!(haps.len(), 4);
    }
    
    #[test]
    fn test_query_arc() {
        let p = Pattern::from_string("a b c d");
        let haps = p.query_arc(0.25, 0.75);
        // Should get "b" and "c"
        assert!(haps.len() >= 2);
    }
    
    #[test]
    fn test_draw_line() {
        let p = Pattern::from_string("a ~ b ~");
        let line = p.draw_line_sz(8);
        // Should show events and gaps
        assert!(line.contains('#'));
        assert!(line.contains('.'));
    }
    
    #[test]
    fn test_equivalent() {
        let p1 = Pattern::from_string("a b c");
        let p2 = Pattern::from_string("a b c");
        let p3 = Pattern::from_string("a b d");
        
        assert!(p1.clone().equivalent_to(p2, 1.0));
        assert!(!p1.equivalent_to(p3, 1.0));
    }
}