//! Tonal and Musical Pattern Operations
//! 
//! Implements musical operators for note manipulation, scales, chords, etc.

use crate::pattern::{Pattern, State, Hap};
use std::collections::HashMap;

/// MIDI note number type
pub type MidiNote = u8;

/// Note names to MIDI mapping
lazy_static::lazy_static! {
    static ref NOTE_TO_MIDI: HashMap<String, MidiNote> = {
        let mut m = HashMap::new();
        // Octave -1
        m.insert("c-1".to_string(), 0);
        m.insert("cs-1".to_string(), 1); m.insert("df-1".to_string(), 1);
        m.insert("d-1".to_string(), 2);
        m.insert("ds-1".to_string(), 3); m.insert("ef-1".to_string(), 3);
        m.insert("e-1".to_string(), 4);
        m.insert("f-1".to_string(), 5);
        m.insert("fs-1".to_string(), 6); m.insert("gf-1".to_string(), 6);
        m.insert("g-1".to_string(), 7);
        m.insert("gs-1".to_string(), 8); m.insert("af-1".to_string(), 8);
        m.insert("a-1".to_string(), 9);
        m.insert("as-1".to_string(), 10); m.insert("bf-1".to_string(), 10);
        m.insert("b-1".to_string(), 11);
        
        // Add all octaves from 0 to 10
        for octave in 0..=10 {
            let base = (octave + 1) * 12;
            m.insert(format!("c{}", octave), base);
            m.insert(format!("cs{}", octave), base + 1);
            m.insert(format!("df{}", octave), base + 1);
            m.insert(format!("d{}", octave), base + 2);
            m.insert(format!("ds{}", octave), base + 3);
            m.insert(format!("ef{}", octave), base + 3);
            m.insert(format!("e{}", octave), base + 4);
            m.insert(format!("f{}", octave), base + 5);
            m.insert(format!("fs{}", octave), base + 6);
            m.insert(format!("gf{}", octave), base + 6);
            m.insert(format!("g{}", octave), base + 7);
            m.insert(format!("gs{}", octave), base + 8);
            m.insert(format!("af{}", octave), base + 8);
            m.insert(format!("a{}", octave), base + 9);
            m.insert(format!("as{}", octave), base + 10);
            m.insert(format!("bf{}", octave), base + 10);
            m.insert(format!("b{}", octave), base + 11);
        }
        m
    };
    
    static ref SCALES: HashMap<&'static str, Vec<i32>> = {
        let mut m = HashMap::new();
        m.insert("major", vec![0, 2, 4, 5, 7, 9, 11]);
        m.insert("minor", vec![0, 2, 3, 5, 7, 8, 10]);
        m.insert("harmonic", vec![0, 2, 3, 5, 7, 8, 11]);
        m.insert("melodic", vec![0, 2, 3, 5, 7, 9, 11]);
        m.insert("dorian", vec![0, 2, 3, 5, 7, 9, 10]);
        m.insert("phrygian", vec![0, 1, 3, 5, 7, 8, 10]);
        m.insert("lydian", vec![0, 2, 4, 6, 7, 9, 11]);
        m.insert("mixolydian", vec![0, 2, 4, 5, 7, 9, 10]);
        m.insert("locrian", vec![0, 1, 3, 5, 6, 8, 10]);
        m.insert("pentatonic", vec![0, 2, 4, 7, 9]);
        m.insert("penta", vec![0, 2, 4, 7, 9]);
        m.insert("blues", vec![0, 3, 5, 6, 7, 10]);
        m.insert("chromatic", vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        m.insert("whole", vec![0, 2, 4, 6, 8, 10]);
        m.insert("wholetone", vec![0, 2, 4, 6, 8, 10]);
        m.insert("augmented", vec![0, 3, 4, 7, 8, 11]);
        m.insert("diminished", vec![0, 2, 3, 5, 6, 8, 9, 11]);
        m.insert("iwato", vec![0, 1, 5, 6, 10]);
        m.insert("hirajoshi", vec![0, 2, 3, 7, 8]);
        m.insert("kumoi", vec![0, 2, 3, 7, 9]);
        m.insert("pelog", vec![0, 1, 3, 7, 8]);
        m.insert("spanish", vec![0, 1, 4, 5, 7, 8, 10]);
        m.insert("bartok", vec![0, 2, 4, 5, 7, 8, 10]);
        m.insert("egyptian", vec![0, 2, 5, 7, 10]);
        m.insert("romanian", vec![0, 2, 3, 6, 7, 9, 10]);
        m.insert("enigmatic", vec![0, 1, 4, 6, 8, 10, 11]);
        m
    };
    
    static ref CHORD_INTERVALS: HashMap<&'static str, Vec<i32>> = {
        let mut m = HashMap::new();
        // Triads
        m.insert("major", vec![0, 4, 7]);
        m.insert("maj", vec![0, 4, 7]);
        m.insert("M", vec![0, 4, 7]);
        m.insert("minor", vec![0, 3, 7]);
        m.insert("min", vec![0, 3, 7]);
        m.insert("m", vec![0, 3, 7]);
        m.insert("diminished", vec![0, 3, 6]);
        m.insert("dim", vec![0, 3, 6]);
        m.insert("augmented", vec![0, 4, 8]);
        m.insert("aug", vec![0, 4, 8]);
        m.insert("sus2", vec![0, 2, 7]);
        m.insert("sus4", vec![0, 5, 7]);
        
        // Seventh chords
        m.insert("maj7", vec![0, 4, 7, 11]);
        m.insert("M7", vec![0, 4, 7, 11]);
        m.insert("min7", vec![0, 3, 7, 10]);
        m.insert("m7", vec![0, 3, 7, 10]);
        m.insert("dom7", vec![0, 4, 7, 10]);
        m.insert("7", vec![0, 4, 7, 10]);
        m.insert("dim7", vec![0, 3, 6, 9]);
        m.insert("hdim7", vec![0, 3, 6, 10]);
        m.insert("m7b5", vec![0, 3, 6, 10]);
        m.insert("aug7", vec![0, 4, 8, 10]);
        m.insert("mM7", vec![0, 3, 7, 11]);
        m.insert("m/maj7", vec![0, 3, 7, 11]);
        
        // Extended chords
        m.insert("maj9", vec![0, 4, 7, 11, 14]);
        m.insert("min9", vec![0, 3, 7, 10, 14]);
        m.insert("dom9", vec![0, 4, 7, 10, 14]);
        m.insert("9", vec![0, 4, 7, 10, 14]);
        m.insert("maj11", vec![0, 4, 7, 11, 14, 17]);
        m.insert("min11", vec![0, 3, 7, 10, 14, 17]);
        m.insert("dom11", vec![0, 4, 7, 10, 14, 17]);
        m.insert("11", vec![0, 4, 7, 10, 14, 17]);
        m.insert("maj13", vec![0, 4, 7, 11, 14, 17, 21]);
        m.insert("min13", vec![0, 3, 7, 10, 14, 17, 21]);
        m.insert("dom13", vec![0, 4, 7, 10, 14, 17, 21]);
        m.insert("13", vec![0, 4, 7, 10, 14, 17, 21]);
        
        // Other
        m.insert("6", vec![0, 4, 7, 9]);
        m.insert("m6", vec![0, 3, 7, 9]);
        m.insert("6/9", vec![0, 4, 7, 9, 14]);
        m.insert("5", vec![0, 7]); // Power chord
        m.insert("power", vec![0, 7]);
        m
    };
}

/// Convert note name to MIDI note number
pub fn note_to_midi(note: &str) -> Option<MidiNote> {
    // Handle numeric input
    if let Ok(n) = note.parse::<MidiNote>() {
        return Some(n);
    }
    
    // Normalize note name and convert # to s
    let note_lower = note.to_lowercase().replace('#', "s");
    
    // Try direct lookup
    if let Some(&midi) = NOTE_TO_MIDI.get(&note_lower) {
        return Some(midi);
    }
    
    // Try to parse with default octave
    if note_lower.len() == 1 || (note_lower.len() == 2 && (note_lower.ends_with('s') || note_lower.ends_with('f'))) {
        let with_octave = format!("{}4", note_lower); // Default to octave 4
        if let Some(&midi) = NOTE_TO_MIDI.get(&with_octave) {
            return Some(midi);
        }
    }
    
    None
}

/// Convert frequency to MIDI note number
pub fn freq_to_midi(freq: f64) -> MidiNote {
    (69.0 + 12.0 * (freq / 440.0).log2()).round() as MidiNote
}

/// Convert MIDI note number to frequency
pub fn midi_to_freq(midi: MidiNote) -> f64 {
    440.0 * 2.0_f64.powf((midi as f64 - 69.0) / 12.0)
}

impl Pattern<String> {
    /// Convert note names to MIDI note numbers
    pub fn note(self) -> Pattern<f64> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter().filter_map(|hap| {
                note_to_midi(&hap.value).map(|midi| {
                    Hap::new(
                        hap.whole,
                        hap.part,
                        midi as f64
                    )
                })
            }).collect()
        })
    }
    
    /// Convert note names to frequencies
    pub fn freq(self) -> Pattern<f64> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter().filter_map(|hap| {
                note_to_midi(&hap.value).map(|midi| {
                    Hap::new(
                        hap.whole,
                        hap.part,
                        midi_to_freq(midi)
                    )
                })
            }).collect()
        })
    }
}

impl Pattern<f64> {
    /// Transpose by semitones
    pub fn transpose(self, semitones: i32) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter().map(|mut hap| {
                hap.value = (hap.value + semitones as f64).max(0.0).min(127.0);
                hap
            }).collect()
        })
    }
    
    /// Apply musical scale
    pub fn scale(self, scale_name: &str, root: MidiNote) -> Self {
        let scale_name = scale_name.to_string();
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            
            if let Some(scale_intervals) = SCALES.get(scale_name.as_str()) {
                haps.into_iter().map(|mut hap| {
                    let degree = hap.value as i32;
                    let octave = degree / scale_intervals.len() as i32;
                    let scale_degree = degree % scale_intervals.len() as i32;
                    
                    let interval = scale_intervals[scale_degree.abs() as usize];
                    hap.value = (root as i32 + octave * 12 + interval) as f64;
                    hap
                }).collect()
            } else {
                haps // Return unchanged if scale not found
            }
        })
    }
    
    /// Invert intervals around a pivot note
    pub fn inv(self, pivot: f64) -> Self {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter().map(|mut hap| {
                hap.value = 2.0 * pivot - hap.value;
                hap
            }).collect()
        })
    }
    
    /// Generate chord from root note
    pub fn chord(self, chord_type: &str) -> Pattern<Vec<f64>> {
        let chord_type = chord_type.to_string();
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            
            if let Some(intervals) = CHORD_INTERVALS.get(chord_type.as_str()) {
                haps.into_iter().map(|hap| {
                    let root = hap.value;
                    let chord_notes: Vec<f64> = intervals.iter()
                        .map(|&interval| root + interval as f64)
                        .collect();
                    
                    Hap::new(
                        hap.whole,
                        hap.part,
                        chord_notes
                    )
                }).collect()
            } else {
                // If chord type not found, return single note as vec
                haps.into_iter().map(|hap| {
                    Hap::new(
                        hap.whole,
                        hap.part,
                        vec![hap.value]
                    )
                }).collect()
            }
        })
    }
    
    /// Scale transpose (transpose within scale)
    pub fn scale_transpose(self, steps: i32, scale_name: &str, root: MidiNote) -> Self {
        let scale_name = scale_name.to_string();
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            
            if let Some(scale_intervals) = SCALES.get(scale_name.as_str()) {
                haps.into_iter().map(|mut hap| {
                    // Find current position in scale
                    let note = hap.value as i32 - root as i32;
                    let mut best_degree = 0;
                    let mut best_distance = i32::MAX;
                    
                    for (i, &interval) in scale_intervals.iter().enumerate() {
                        let distance = (note % 12 - interval).abs();
                        if distance < best_distance {
                            best_distance = distance;
                            best_degree = i as i32;
                        }
                    }
                    
                    // Transpose within scale
                    let octave = note / 12;
                    let new_degree = best_degree + steps;
                    let new_octave = octave + new_degree / scale_intervals.len() as i32;
                    let new_scale_degree = new_degree.rem_euclid(scale_intervals.len() as i32);
                    
                    let new_interval = scale_intervals[new_scale_degree as usize];
                    hap.value = (root as i32 + new_octave * 12 + new_interval) as f64;
                    hap
                }).collect()
            } else {
                haps
            }
        })
    }
}

impl Pattern<Vec<f64>> {
    /// Arpeggiate chord
    pub fn arp(self, pattern: &str) -> Pattern<f64> {
        let pattern = pattern.to_string();
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            let mut result = Vec::new();
            
            for hap in haps {
                if hap.value.is_empty() {
                    continue;
                }
                
                // Parse arp pattern (e.g., "up", "down", "updown", "random")
                let arp_sequence = match pattern.as_str() {
                    "up" => {
                        let mut notes = hap.value.clone();
                        notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        notes
                    },
                    "down" => {
                        let mut notes = hap.value.clone();
                        notes.sort_by(|a, b| b.partial_cmp(a).unwrap());
                        notes
                    },
                    "updown" => {
                        let mut notes = hap.value.clone();
                        notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let mut down = notes.clone();
                        down.reverse();
                        notes.extend(down.into_iter().skip(1));
                        notes
                    },
                    "downup" => {
                        let mut notes = hap.value.clone();
                        notes.sort_by(|a, b| b.partial_cmp(a).unwrap());
                        let mut up = notes.clone();
                        up.reverse();
                        notes.extend(up.into_iter().skip(1));
                        notes
                    },
                    "converge" => {
                        let mut notes = hap.value.clone();
                        notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let mut result = Vec::new();
                        let mut left = 0;
                        let mut right = notes.len() - 1;
                        while left <= right {
                            result.push(notes[left]);
                            if left != right {
                                result.push(notes[right]);
                            }
                            left += 1;
                            right = right.saturating_sub(1);
                        }
                        result
                    },
                    "diverge" => {
                        let mut notes = hap.value.clone();
                        notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let mid = notes.len() / 2;
                        let mut result = vec![notes[mid]];
                        for i in 1..=mid {
                            if mid >= i {
                                result.push(notes[mid - i]);
                            }
                            if mid + i < notes.len() {
                                result.push(notes[mid + i]);
                            }
                        }
                        result
                    },
                    _ => hap.value.clone(), // Default: as-is
                };
                
                // Distribute notes across the hap duration
                let duration = hap.part.duration();
                let step = duration / crate::pattern::Fraction::new(arp_sequence.len() as i64, 1);
                
                for (i, &note) in arp_sequence.iter().enumerate() {
                    let begin = hap.part.begin + step * crate::pattern::Fraction::new(i as i64, 1);
                    let end = begin + step;
                    
                    result.push(Hap::new(
                        hap.whole,
                        crate::pattern::TimeSpan::new(begin, end),
                        note
                    ));
                }
            }
            
            result
        })
    }
    
    /// Apply chord voicing
    pub fn voicing(self, voicing_type: &str) -> Self {
        let voicing_type = voicing_type.to_string();
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            
            haps.into_iter().map(|mut hap| {
                if hap.value.len() <= 1 {
                    return hap;
                }
                
                let mut notes = hap.value.clone();
                
                match voicing_type.as_str() {
                    "drop2" => {
                        // Move second highest note down an octave
                        notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        if notes.len() >= 2 {
                            let second_highest = notes.len() - 2;
                            notes[second_highest] -= 12.0;
                            notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        }
                    },
                    "drop3" => {
                        // Move third highest note down an octave
                        notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        if notes.len() >= 3 {
                            let third_highest = notes.len() - 3;
                            notes[third_highest] -= 12.0;
                            notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        }
                    },
                    "spread" => {
                        // Spread notes across octaves
                        notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        for i in 1..notes.len() {
                            if notes[i] - notes[i-1] < 3.0 {
                                notes[i] += 12.0;
                            }
                        }
                    },
                    "close" => {
                        // Keep notes within an octave
                        notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let root = notes[0];
                        for i in 1..notes.len() {
                            while notes[i] - root > 12.0 {
                                notes[i] -= 12.0;
                            }
                        }
                        notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    },
                    "invert1" => {
                        // First inversion
                        if notes.len() > 1 {
                            notes[0] += 12.0;
                            notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        }
                    },
                    "invert2" => {
                        // Second inversion
                        if notes.len() > 2 {
                            notes[0] += 12.0;
                            notes[1] += 12.0;
                            notes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        }
                    },
                    _ => {}, // Unknown voicing, leave as-is
                }
                
                hap.value = notes;
                hap
            }).collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::{Pattern, State, TimeSpan, Fraction};
    use std::collections::HashMap;
    
    #[test]
    fn test_note_to_midi() {
        assert_eq!(note_to_midi("c4"), Some(60));
        assert_eq!(note_to_midi("a4"), Some(69));
        assert_eq!(note_to_midi("c#4"), Some(61));
        assert_eq!(note_to_midi("cs4"), Some(61));
        assert_eq!(note_to_midi("df4"), Some(61));
        assert_eq!(note_to_midi("60"), Some(60));
    }
    
    #[test]
    fn test_midi_to_freq() {
        assert!((midi_to_freq(69) - 440.0).abs() < 0.01);
        assert!((midi_to_freq(60) - 261.63).abs() < 0.1);
    }
    
    #[test]
    fn test_pattern_note() {
        let p = Pattern::from_string("c4 e4 g4");
        let note_pattern = p.note();
        
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        
        let haps = note_pattern.query(&state);
        assert_eq!(haps.len(), 3);
        assert_eq!(haps[0].value, 60.0); // C4
        assert_eq!(haps[1].value, 64.0); // E4
        assert_eq!(haps[2].value, 67.0); // G4
    }
    
    #[test]
    fn test_transpose() {
        let p = Pattern::pure(60.0); // C4
        let transposed = p.transpose(7); // Up a fifth
        
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        
        let haps = transposed.query(&state);
        assert_eq!(haps[0].value, 67.0); // G4
    }
    
    #[test]
    fn test_scale() {
        let p = Pattern::from_string("0 1 2 3 4")
            .map(|s| s.parse::<f64>().unwrap_or(0.0));
        let scaled = p.scale("major", 60); // C major scale
        
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        
        let haps = scaled.query(&state);
        assert_eq!(haps[0].value, 60.0); // C
        assert_eq!(haps[1].value, 62.0); // D
        assert_eq!(haps[2].value, 64.0); // E
        assert_eq!(haps[3].value, 65.0); // F
        assert_eq!(haps[4].value, 67.0); // G
    }
    
    #[test]
    fn test_chord() {
        let p = Pattern::pure(60.0); // C4
        let chord = p.chord("maj7");
        
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        
        let haps = chord.query(&state);
        assert_eq!(haps[0].value, vec![60.0, 64.0, 67.0, 71.0]); // C E G B
    }
}