//! Mock Plugin Instance for Testing
//!
//! Provides a deterministic plugin implementation that generates predictable
//! audio output for automated testing. Behaves like a simple sine synth.

use super::instance::MidiEvent;
use super::types::*;
use std::f32::consts::PI;

/// Mock plugin that generates sine waves from MIDI notes
/// Used for testing without external plugin dependencies
#[derive(Clone, Debug)]
pub struct MockPluginInstance {
    /// Plugin info
    info: PluginInfo,
    /// Sample rate
    sample_rate: f32,
    /// Current phase for each voice (up to 16 voices)
    phases: [f32; 16],
    /// Currently playing notes (MIDI note number, velocity) for each voice
    voices: [Option<(u8, u8)>; 16],
    /// Volume parameter (0.0 - 1.0)
    volume: f32,
    /// Pitch bend in semitones (-2 to +2)
    pitch_bend: f32,
    /// Whether initialized
    initialized: bool,
    /// Total samples processed (for testing)
    samples_processed: u64,
}

impl Default for MockPluginInstance {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPluginInstance {
    /// Create a new mock plugin instance
    pub fn new() -> Self {
        Self {
            info: Self::mock_plugin_info(),
            sample_rate: 44100.0,
            phases: [0.0; 16],
            voices: [None; 16],
            volume: 0.8,
            pitch_bend: 0.0,
            initialized: false,
            samples_processed: 0,
        }
    }

    /// Get mock plugin info
    pub fn mock_plugin_info() -> PluginInfo {
        PluginInfo {
            id: PluginId {
                format: PluginFormat::Vst3,
                identifier: "com.phonon.mock-synth".to_string(),
                name: "MockSynth".to_string(),
            },
            vendor: "Phonon Test".to_string(),
            version: "1.0.0".to_string(),
            category: PluginCategory::Instrument,
            num_inputs: 0,
            num_outputs: 2,
            parameters: vec![
                ParameterInfo {
                    index: 0,
                    name: "Volume".to_string(),
                    short_name: "Vol".to_string(),
                    default_value: 0.8,
                    min_value: 0.0,
                    max_value: 1.0,
                    unit: "".to_string(),
                    step_count: 0,
                    automatable: true,
                },
                ParameterInfo {
                    index: 1,
                    name: "Pitch Bend".to_string(),
                    short_name: "Bend".to_string(),
                    default_value: 0.5, // Center = 0 semitones
                    min_value: 0.0,     // -2 semitones
                    max_value: 1.0,     // +2 semitones
                    unit: "st".to_string(),
                    step_count: 0,
                    automatable: true,
                },
            ],
            factory_presets: vec!["Init".to_string(), "Lead".to_string()],
            has_gui: false,
            path: "mock://MockSynth".to_string(),
        }
    }

    /// Initialize with sample rate
    pub fn initialize(&mut self, sample_rate: f32, _max_block_size: usize) -> PluginResult<()> {
        self.sample_rate = sample_rate;
        self.initialized = true;
        self.phases = [0.0; 16];
        self.voices = [None; 16];
        Ok(())
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get plugin info
    pub fn info(&self) -> &PluginInfo {
        &self.info
    }

    /// Process MIDI events and generate audio
    pub fn process_with_midi(
        &mut self,
        midi_events: &[MidiEvent],
        outputs: &mut [&mut [f32]],
        num_samples: usize,
    ) -> PluginResult<()> {
        if !self.initialized {
            return Err(PluginError::ProcessError("Not initialized".to_string()));
        }

        // Process MIDI events (apply at sample offset)
        let mut events_by_sample: Vec<Vec<&MidiEvent>> = vec![Vec::new(); num_samples];
        for event in midi_events {
            let offset = event.sample_offset.min(num_samples - 1);
            events_by_sample[offset].push(event);
        }

        // Generate audio sample by sample
        for sample_idx in 0..num_samples {
            // Process MIDI events at this sample
            for event in &events_by_sample[sample_idx] {
                self.handle_midi_event(event);
            }

            // Generate audio from all active voices
            let mut sample = 0.0f32;
            for (voice_idx, voice) in self.voices.iter().enumerate() {
                if let Some((note, velocity)) = voice {
                    let freq = self.note_to_freq(*note);
                    let vel_scale = *velocity as f32 / 127.0;

                    // Generate sine wave
                    sample += (self.phases[voice_idx] * 2.0 * PI).sin() * vel_scale;

                    // Advance phase
                    self.phases[voice_idx] += freq / self.sample_rate;
                    if self.phases[voice_idx] >= 1.0 {
                        self.phases[voice_idx] -= 1.0;
                    }
                }
            }

            // Apply volume
            sample *= self.volume;

            // Soft clip to prevent harsh distortion
            sample = sample.tanh();

            // Write to outputs (stereo)
            if outputs.len() >= 2 {
                outputs[0][sample_idx] = sample;
                outputs[1][sample_idx] = sample;
            } else if !outputs.is_empty() {
                outputs[0][sample_idx] = sample;
            }

            self.samples_processed += 1;
        }

        Ok(())
    }

    /// Handle a single MIDI event
    fn handle_midi_event(&mut self, event: &MidiEvent) {
        if event.is_note_on() {
            // Find free voice
            if let Some(voice_idx) = self.voices.iter().position(|v| v.is_none()) {
                self.voices[voice_idx] = Some((event.data1, event.data2));
                self.phases[voice_idx] = 0.0; // Reset phase for clean attack
            }
        } else if event.is_note_off() {
            // Find and release voice playing this note
            if let Some(voice_idx) = self
                .voices
                .iter()
                .position(|v| v.map(|(n, _)| n) == Some(event.data1))
            {
                self.voices[voice_idx] = None;
            }
        }
        // Could handle CC, pitch bend, etc. here
    }

    /// Convert MIDI note to frequency (with pitch bend)
    fn note_to_freq(&self, note: u8) -> f32 {
        let semitones = note as f32 - 69.0 + self.pitch_bend;
        440.0 * 2.0f32.powf(semitones / 12.0)
    }

    /// Set parameter value
    pub fn set_parameter(&mut self, index: usize, value: f32) -> PluginResult<()> {
        match index {
            0 => {
                self.volume = value.clamp(0.0, 1.0);
                Ok(())
            }
            1 => {
                // Map 0-1 to -2 to +2 semitones
                self.pitch_bend = (value - 0.5) * 4.0;
                Ok(())
            }
            _ => Err(PluginError::ParameterError(format!(
                "Invalid parameter index: {}",
                index
            ))),
        }
    }

    /// Get parameter value
    pub fn get_parameter(&self, index: usize) -> PluginResult<f32> {
        match index {
            0 => Ok(self.volume),
            1 => Ok((self.pitch_bend / 4.0) + 0.5),
            _ => Err(PluginError::ParameterError(format!(
                "Invalid parameter index: {}",
                index
            ))),
        }
    }

    /// Get total samples processed (for testing)
    pub fn samples_processed(&self) -> u64 {
        self.samples_processed
    }

    /// Get number of active voices (for testing)
    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.is_some()).count()
    }

    /// Check if a specific note is playing (for testing)
    pub fn is_note_playing(&self, note: u8) -> bool {
        self.voices
            .iter()
            .any(|v| v.map(|(n, _)| n) == Some(note))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_plugin_creation() {
        let plugin = MockPluginInstance::new();
        assert!(!plugin.is_initialized());
        assert_eq!(plugin.info().id.name, "MockSynth");
    }

    #[test]
    fn test_mock_plugin_initialize() {
        let mut plugin = MockPluginInstance::new();
        assert!(plugin.initialize(44100.0, 512).is_ok());
        assert!(plugin.is_initialized());
    }

    #[test]
    fn test_mock_plugin_note_on_off() {
        let mut plugin = MockPluginInstance::new();
        plugin.initialize(44100.0, 512).unwrap();

        // Note on
        let note_on = MidiEvent::note_on(0, 0, 60, 100);
        let mut left = vec![0.0f32; 512];
        let mut right = vec![0.0f32; 512];

        {
            let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];
            plugin
                .process_with_midi(&[note_on], &mut outputs, 512)
                .unwrap();
        }

        assert!(plugin.is_note_playing(60));
        assert_eq!(plugin.active_voices(), 1);

        // Check that audio was generated (not silence)
        let max_sample = left.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max_sample > 0.01, "Expected audio output, got silence");

        // Note off
        let note_off = MidiEvent::note_off(0, 0, 60);
        {
            let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];
            plugin
                .process_with_midi(&[note_off], &mut outputs, 512)
                .unwrap();
        }

        assert!(!plugin.is_note_playing(60));
        assert_eq!(plugin.active_voices(), 0);
    }

    #[test]
    fn test_mock_plugin_frequency_accuracy() {
        let mut plugin = MockPluginInstance::new();
        plugin.initialize(44100.0, 512).unwrap();
        plugin.set_parameter(0, 1.0).unwrap(); // Full volume

        // Play A4 (440 Hz, MIDI note 69)
        let note_on = MidiEvent::note_on(0, 0, 69, 127);
        let sample_rate = 44100.0;
        let expected_freq = 440.0;
        let samples_per_cycle = sample_rate / expected_freq;

        // Generate enough samples for several cycles
        let num_samples = (samples_per_cycle * 10.0) as usize;
        let mut left = vec![0.0f32; num_samples];
        let mut right = vec![0.0f32; num_samples];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];

        plugin
            .process_with_midi(&[note_on], &mut outputs, num_samples)
            .unwrap();

        // Find zero crossings to measure frequency
        let mut zero_crossings = 0;
        for i in 1..num_samples {
            if (left[i - 1] < 0.0 && left[i] >= 0.0) || (left[i - 1] >= 0.0 && left[i] < 0.0) {
                zero_crossings += 1;
            }
        }

        // Each cycle has 2 zero crossings
        let measured_cycles = zero_crossings as f32 / 2.0;
        let measured_freq = measured_cycles / (num_samples as f32 / sample_rate);

        // Allow 5% tolerance for measurement error
        let freq_error = (measured_freq - expected_freq).abs() / expected_freq;
        assert!(
            freq_error < 0.05,
            "Expected ~440 Hz, measured {} Hz ({}% error)",
            measured_freq,
            freq_error * 100.0
        );
    }

    #[test]
    fn test_mock_plugin_polyphony() {
        let mut plugin = MockPluginInstance::new();
        plugin.initialize(44100.0, 512).unwrap();

        // Play chord C-E-G
        let c = MidiEvent::note_on(0, 0, 60, 100);
        let e = MidiEvent::note_on(1, 0, 64, 100);
        let g = MidiEvent::note_on(2, 0, 67, 100);

        let mut left = vec![0.0f32; 512];
        let mut right = vec![0.0f32; 512];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];

        plugin
            .process_with_midi(&[c, e, g], &mut outputs, 512)
            .unwrap();

        assert_eq!(plugin.active_voices(), 3);
        assert!(plugin.is_note_playing(60));
        assert!(plugin.is_note_playing(64));
        assert!(plugin.is_note_playing(67));
    }

    #[test]
    fn test_mock_plugin_volume_parameter() {
        let mut plugin = MockPluginInstance::new();
        plugin.initialize(44100.0, 512).unwrap();

        // Test volume at 1.0
        plugin.set_parameter(0, 1.0).unwrap();
        let note_on = MidiEvent::note_on(0, 0, 69, 127);

        let mut left_loud = vec![0.0f32; 512];
        let mut right_loud = vec![0.0f32; 512];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left_loud, &mut right_loud];
        plugin
            .process_with_midi(&[note_on.clone()], &mut outputs, 512)
            .unwrap();

        let rms_loud: f32 = (left_loud.iter().map(|s| s * s).sum::<f32>() / 512.0).sqrt();

        // Reset and test volume at 0.5
        let mut plugin2 = MockPluginInstance::new();
        plugin2.initialize(44100.0, 512).unwrap();
        plugin2.set_parameter(0, 0.5).unwrap();

        let mut left_quiet = vec![0.0f32; 512];
        let mut right_quiet = vec![0.0f32; 512];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left_quiet, &mut right_quiet];
        plugin2
            .process_with_midi(&[note_on], &mut outputs, 512)
            .unwrap();

        let rms_quiet: f32 = (left_quiet.iter().map(|s| s * s).sum::<f32>() / 512.0).sqrt();

        // Loud should be roughly 2x the amplitude of quiet
        // (Actually more like 1.5x due to tanh compression)
        assert!(
            rms_loud > rms_quiet * 1.3,
            "Expected louder with higher volume: loud={}, quiet={}",
            rms_loud,
            rms_quiet
        );
    }
}
