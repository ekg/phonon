//! Actual sample playback using fundsp's envelope function

use fundsp::prelude::*;
use std::sync::Arc;

/// Create a real sample player that plays the actual sample data
pub fn create_sample_player(samples: Arc<Vec<f32>>) -> Box<dyn AudioUnit> {
    if samples.is_empty() {
        return Box::new(fundsp::hacker::zero());
    }
    
    // Clone the samples for the closure
    let samples_clone = samples.clone();
    
    // Create an envelope that outputs the actual sample values
    // This is triggered once and plays through the entire sample
    let sample_envelope = fundsp::hacker::envelope(move |t| {
        let sample_index = (t * 44100.0) as usize;
        if sample_index < samples_clone.len() {
            samples_clone[sample_index] as f64
        } else {
            0.0
        }
    });
    
    // Return the envelope as the audio unit
    Box::new(sample_envelope)
}