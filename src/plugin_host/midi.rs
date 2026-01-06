//! MIDI Event Handling
//!
//! Converts Phonon note patterns to MIDI events for plugin instruments.
//! Provides sample-accurate MIDI event buffering.

use super::instance::MidiEvent;

/// Buffer of MIDI events for a processing block
#[derive(Clone, Debug, Default)]
pub struct MidiEventBuffer {
    /// Events sorted by sample offset
    events: Vec<MidiEvent>,
    /// Whether events are sorted
    sorted: bool,
}

impl MidiEventBuffer {
    /// Create a new empty buffer
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            sorted: true,
        }
    }

    /// Create a buffer with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
            sorted: true,
        }
    }

    /// Add an event to the buffer
    pub fn push(&mut self, event: MidiEvent) {
        // Check if we need to re-sort
        if let Some(last) = self.events.last() {
            if event.sample_offset < last.sample_offset {
                self.sorted = false;
            }
        }
        self.events.push(event);
    }

    /// Add a note-on event
    pub fn note_on(&mut self, sample_offset: usize, channel: u8, note: u8, velocity: u8) {
        self.push(MidiEvent::note_on(sample_offset, channel, note, velocity));
    }

    /// Add a note-off event
    pub fn note_off(&mut self, sample_offset: usize, channel: u8, note: u8) {
        self.push(MidiEvent::note_off(sample_offset, channel, note));
    }

    /// Add a control change event
    pub fn control_change(&mut self, sample_offset: usize, channel: u8, controller: u8, value: u8) {
        self.push(MidiEvent::control_change(
            sample_offset,
            channel,
            controller,
            value,
        ));
    }

    /// Add a pitch bend event
    pub fn pitch_bend(&mut self, sample_offset: usize, channel: u8, value: i16) {
        self.push(MidiEvent::pitch_bend(sample_offset, channel, value));
    }

    /// Sort events by sample offset
    pub fn sort(&mut self) {
        if !self.sorted {
            self.events.sort_by_key(|e| e.sample_offset);
            self.sorted = true;
        }
    }

    /// Get all events (sorted)
    pub fn events(&mut self) -> &[MidiEvent] {
        self.sort();
        &self.events
    }

    /// Get events in a sample range
    pub fn events_in_range(&mut self, start: usize, end: usize) -> Vec<&MidiEvent> {
        self.sort();
        self.events
            .iter()
            .filter(|e| e.sample_offset >= start && e.sample_offset < end)
            .collect()
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
        self.sorted = true;
    }

    /// Number of events
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Convert note number to MIDI note
    /// A4 = 69 = 440 Hz
    pub fn note_name_to_midi(name: &str) -> Option<u8> {
        let name = name.to_lowercase();
        let bytes = name.as_bytes();

        if bytes.is_empty() {
            return None;
        }

        // Parse note name (c, d, e, f, g, a, b)
        let base_note = match bytes[0] {
            b'c' => 0,
            b'd' => 2,
            b'e' => 4,
            b'f' => 5,
            b'g' => 7,
            b'a' => 9,
            b'b' => 11,
            _ => return None,
        };

        let mut pos = 1;
        let mut modifier = 0i8;

        // Check for sharp/flat
        if pos < bytes.len() {
            match bytes[pos] {
                b'#' | b's' => {
                    modifier = 1;
                    pos += 1;
                }
                b'b' | b'f' => {
                    modifier = -1;
                    pos += 1;
                }
                _ => {}
            }
        }

        // Parse octave
        let octave: i8 = if pos < bytes.len() {
            let octave_str = &name[pos..];
            octave_str.parse().ok()?
        } else {
            4 // Default to octave 4
        };

        // Calculate MIDI note (C4 = 60)
        let midi_note = 12i16 * (octave as i16 + 1) + base_note as i16 + modifier as i16;

        if midi_note >= 0 && midi_note <= 127 {
            Some(midi_note as u8)
        } else {
            None
        }
    }

    /// Convert frequency to nearest MIDI note
    pub fn freq_to_midi(freq: f64) -> u8 {
        let midi = 69.0 + 12.0 * (freq / 440.0).log2();
        midi.round().clamp(0.0, 127.0) as u8
    }

    /// Convert MIDI note to frequency
    pub fn midi_to_freq(note: u8) -> f64 {
        440.0 * 2.0_f64.powf((note as f64 - 69.0) / 12.0)
    }
}

/// Note event from pattern evaluation
#[derive(Clone, Debug)]
pub struct NoteEvent {
    /// Start time in samples from buffer start
    pub start_sample: usize,
    /// Duration in samples
    pub duration_samples: usize,
    /// MIDI note number (0-127)
    pub note: u8,
    /// Velocity (0-127)
    pub velocity: u8,
    /// MIDI channel (0-15)
    pub channel: u8,
}

impl NoteEvent {
    /// Create a new note event
    pub fn new(start_sample: usize, duration_samples: usize, note: u8, velocity: u8) -> Self {
        Self {
            start_sample,
            duration_samples,
            note,
            velocity,
            channel: 0,
        }
    }

    /// Create from note name
    pub fn from_name(
        start_sample: usize,
        duration_samples: usize,
        name: &str,
        velocity: u8,
    ) -> Option<Self> {
        let note = MidiEventBuffer::note_name_to_midi(name)?;
        Some(Self::new(start_sample, duration_samples, note, velocity))
    }

    /// Convert to MIDI events (note-on and note-off)
    pub fn to_midi_events(&self) -> (MidiEvent, MidiEvent) {
        let note_on = MidiEvent::note_on(self.start_sample, self.channel, self.note, self.velocity);
        let note_off = MidiEvent::note_off(
            self.start_sample + self.duration_samples,
            self.channel,
            self.note,
        );
        (note_on, note_off)
    }
}

/// Convert a list of note events to a MIDI buffer
pub fn notes_to_midi_buffer(notes: &[NoteEvent]) -> MidiEventBuffer {
    let mut buffer = MidiEventBuffer::with_capacity(notes.len() * 2);

    for note in notes {
        let (on, off) = note.to_midi_events();
        buffer.push(on);
        buffer.push(off);
    }

    buffer.sort();
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_name_to_midi() {
        // Standard notes
        assert_eq!(MidiEventBuffer::note_name_to_midi("c4"), Some(60));
        assert_eq!(MidiEventBuffer::note_name_to_midi("a4"), Some(69));
        assert_eq!(MidiEventBuffer::note_name_to_midi("c5"), Some(72));

        // Sharps and flats
        assert_eq!(MidiEventBuffer::note_name_to_midi("c#4"), Some(61));
        assert_eq!(MidiEventBuffer::note_name_to_midi("cs4"), Some(61)); // Alternative sharp notation
        assert_eq!(MidiEventBuffer::note_name_to_midi("db4"), Some(61));
        assert_eq!(MidiEventBuffer::note_name_to_midi("df4"), Some(61)); // Alternative flat notation

        // Case insensitive
        assert_eq!(MidiEventBuffer::note_name_to_midi("C4"), Some(60));
        assert_eq!(MidiEventBuffer::note_name_to_midi("A#4"), Some(70));

        // Different octaves
        assert_eq!(MidiEventBuffer::note_name_to_midi("c0"), Some(12));
        assert_eq!(MidiEventBuffer::note_name_to_midi("c-1"), Some(0));
        assert_eq!(MidiEventBuffer::note_name_to_midi("g9"), Some(127));

        // Invalid
        assert_eq!(MidiEventBuffer::note_name_to_midi("x4"), None);
        assert_eq!(MidiEventBuffer::note_name_to_midi(""), None);
    }

    #[test]
    fn test_freq_to_midi() {
        assert_eq!(MidiEventBuffer::freq_to_midi(440.0), 69); // A4
        assert_eq!(MidiEventBuffer::freq_to_midi(261.63), 60); // C4 (approximately)
        assert_eq!(MidiEventBuffer::freq_to_midi(880.0), 81); // A5
    }

    #[test]
    fn test_midi_to_freq() {
        assert!((MidiEventBuffer::midi_to_freq(69) - 440.0).abs() < 0.01);
        assert!((MidiEventBuffer::midi_to_freq(60) - 261.63).abs() < 0.1);
        assert!((MidiEventBuffer::midi_to_freq(81) - 880.0).abs() < 0.01);
    }

    #[test]
    fn test_midi_buffer_sorting() {
        let mut buffer = MidiEventBuffer::new();

        // Add events out of order
        buffer.note_on(100, 0, 60, 100);
        buffer.note_on(0, 0, 62, 100);
        buffer.note_on(50, 0, 64, 100);

        let events = buffer.events();
        assert_eq!(events[0].sample_offset, 0);
        assert_eq!(events[1].sample_offset, 50);
        assert_eq!(events[2].sample_offset, 100);
    }

    #[test]
    fn test_note_event_to_midi() {
        let note = NoteEvent::new(0, 1000, 60, 100);
        let (on, off) = note.to_midi_events();

        assert!(on.is_note_on());
        assert_eq!(on.sample_offset, 0);
        assert_eq!(on.data1, 60);
        assert_eq!(on.data2, 100);

        assert!(off.is_note_off());
        assert_eq!(off.sample_offset, 1000);
        assert_eq!(off.data1, 60);
    }

    #[test]
    fn test_notes_to_midi_buffer() {
        let notes = vec![
            NoteEvent::new(0, 1000, 60, 100),
            NoteEvent::new(500, 500, 64, 80),
        ];

        let buffer = notes_to_midi_buffer(&notes);
        assert_eq!(buffer.len(), 4); // 2 note-on + 2 note-off

        // Events should be sorted
        let events = &buffer.events;
        assert_eq!(events[0].sample_offset, 0); // C4 note-on
        assert_eq!(events[1].sample_offset, 500); // E4 note-on
        assert_eq!(events[2].sample_offset, 1000); // Both note-offs at same time or C4 first
    }
}
