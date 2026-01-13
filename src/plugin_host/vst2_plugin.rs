//! VST2 Plugin Hosting
//!
//! Provides hosting for legacy VST2 plugins using the `vst` crate.
//! VST2 is deprecated but many plugins still use this format.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use vst::host::{Host, PluginLoader, PluginInstance};
use vst::plugin::Plugin;

use super::types::{PluginError, PluginResult, PluginInfo, PluginId, PluginFormat, PluginCategory};

/// Simple host implementation for VST2 plugins
struct Vst2Host;

impl Host for Vst2Host {
    fn automate(&self, _index: i32, _value: f32) {
        // Parameter automation callback
    }
}

/// VST2 plugin instance wrapper
pub struct Vst2PluginInstance {
    /// The loaded plugin instance
    plugin: PluginInstance,
    /// Plugin info
    pub info: PluginInfo,
    /// Sample rate
    sample_rate: f32,
    /// Block size
    block_size: usize,
    /// Plugin path
    path: PathBuf,
}

impl Vst2PluginInstance {
    /// Load a VST2 plugin from a path
    pub fn load(path: &Path) -> PluginResult<Self> {
        let host = Arc::new(Mutex::new(Vst2Host));

        let mut loader = PluginLoader::load(path, host)
            .map_err(|e| PluginError::LoadError(format!("VST2 load failed: {}", e)))?;

        let mut plugin = loader.instance()
            .map_err(|e| PluginError::LoadError(format!("VST2 instance failed: {}", e)))?;

        // Get plugin info
        let vst_info = plugin.get_info();

        let category = match vst_info.category {
            vst::plugin::Category::Synth => PluginCategory::Instrument,
            vst::plugin::Category::Generator => PluginCategory::Instrument,
            _ => PluginCategory::Effect,
        };

        let info = PluginInfo {
            id: PluginId {
                format: PluginFormat::Vst2,
                identifier: format!("{}", vst_info.unique_id),
                name: vst_info.name.clone(),
            },
            vendor: vst_info.vendor.clone(),
            version: format!("{}", vst_info.version),
            category,
            num_inputs: vst_info.inputs as usize,
            num_outputs: vst_info.outputs as usize,
            parameters: vec![],
            factory_presets: vec![],
            has_gui: false,
            path: path.to_string_lossy().to_string(),
        };

        // Initialize the plugin
        plugin.init();

        Ok(Self {
            plugin,
            info,
            sample_rate: 44100.0,
            block_size: 512,
            path: path.to_path_buf(),
        })
    }

    /// Initialize the plugin with sample rate and block size
    pub fn initialize(&mut self, sample_rate: f32, block_size: usize) -> PluginResult<()> {
        self.sample_rate = sample_rate;
        self.block_size = block_size;

        self.plugin.set_sample_rate(sample_rate);
        self.plugin.set_block_size(block_size as i64);
        self.plugin.resume();

        Ok(())
    }

    /// Get the number of parameters
    pub fn parameter_count(&self) -> usize {
        self.plugin.get_info().parameters as usize
    }

    /// Get parameter name
    pub fn get_parameter_name(&mut self, index: usize) -> String {
        let params = self.plugin.get_parameter_object();
        params.get_parameter_name(index as i32)
    }

    /// Set a parameter value (normalized 0-1)
    pub fn set_parameter(&mut self, index: usize, value: f32) -> PluginResult<()> {
        let params = self.plugin.get_parameter_object();
        params.set_parameter(index as i32, value);
        Ok(())
    }

    /// Get a parameter value
    pub fn get_parameter(&mut self, index: usize) -> f32 {
        let params = self.plugin.get_parameter_object();
        params.get_parameter(index as i32)
    }

    /// Process audio (stereo in/out) using raw pointers
    pub fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]], samples: usize) -> PluginResult<()> {
        // Prepare input buffers
        let mut input_vecs: Vec<Vec<f32>> = if inputs.is_empty() {
            vec![vec![0.0f32; samples]; 2]
        } else {
            inputs.iter().map(|ch| ch[..samples].to_vec()).collect()
        };

        let mut output_vecs: Vec<Vec<f32>> = outputs.iter()
            .map(|_| vec![0.0f32; samples])
            .collect();

        // Get raw pointers
        let input_ptrs: Vec<*const f32> = input_vecs.iter()
            .map(|v| v.as_ptr())
            .collect();
        let mut output_ptrs: Vec<*mut f32> = output_vecs.iter_mut()
            .map(|v| v.as_mut_ptr())
            .collect();

        // Create AudioBuffer from raw pointers
        unsafe {
            let buffer = vst::buffer::AudioBuffer::from_raw(
                input_ptrs.len(),
                output_ptrs.len(),
                input_ptrs.as_ptr(),
                output_ptrs.as_mut_ptr(),
                samples,
            );
            self.plugin.process(&mut { buffer });
        }

        // Copy outputs back
        for (i, output) in outputs.iter_mut().enumerate() {
            if i < output_vecs.len() {
                let len = samples.min(output.len());
                output[..len].copy_from_slice(&output_vecs[i][..len]);
            }
        }

        Ok(())
    }

    /// Process with MIDI events
    pub fn process_with_midi(
        &mut self,
        midi_events: &[super::instance::MidiEvent],
        outputs: &mut [&mut [f32]],
        samples: usize,
    ) -> PluginResult<()> {
        // Send MIDI events
        for event in midi_events {
            let midi_data = [event.status, event.data1, event.data2];
            let midi_event = vst::api::MidiEvent {
                event_type: vst::api::EventType::Midi,
                byte_size: std::mem::size_of::<vst::api::MidiEvent>() as i32,
                delta_frames: event.sample_offset as i32,
                flags: vst::api::MidiEventFlags::REALTIME_EVENT.bits(),
                note_length: 0,
                note_offset: 0,
                midi_data,
                _midi_reserved: 0,
                detune: 0,
                note_off_velocity: 0,
                _reserved1: 0,
                _reserved2: 0,
            };

            unsafe {
                let event_ptr = &midi_event as *const vst::api::MidiEvent as *const vst::api::Event;
                let events = vst::api::Events {
                    num_events: 1,
                    _reserved: 0,
                    events: [event_ptr as *mut _; 2],
                };
                self.plugin.process_events(&events);
            }
        }

        // Process audio
        self.process(&[], outputs, samples)
    }

    /// Get plugin name
    pub fn name(&self) -> &str {
        &self.info.id.name
    }
}

/// Scan a directory for VST2 plugins (quick scan - just list files)
pub fn scan_vst2_directory(dir: &Path) -> Vec<PluginInfo> {
    let mut plugins = Vec::new();

    if !dir.exists() {
        return plugins;
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "so" || ext == "dll" || ext == "vst") {
                // Quick scan - just list the file without loading
                if let Some(stem) = path.file_stem() {
                    plugins.push(PluginInfo {
                        id: PluginId {
                            format: PluginFormat::Vst2,
                            identifier: stem.to_string_lossy().to_string(),
                            name: stem.to_string_lossy().to_string(),
                        },
                        vendor: "Unknown".to_string(),
                        version: "1.0".to_string(),
                        category: PluginCategory::Effect,  // Assume effect, will be corrected on load
                        num_inputs: 2,
                        num_outputs: 2,
                        parameters: vec![],
                        factory_presets: vec![],
                        has_gui: false,
                        path: path.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }

    plugins
}

/// Create a VST2 plugin instance by name
pub fn create_vst2_plugin_by_name(name: &str) -> PluginResult<Vst2PluginInstance> {
    // Search common VST2 directories
    let search_dirs = [
        dirs::home_dir().map(|h| h.join(".vst")),
        dirs::home_dir().map(|h| h.join(".vst2")),
        Some(std::path::PathBuf::from("/usr/lib/vst")),
        Some(std::path::PathBuf::from("/usr/local/lib/vst")),
    ];

    for dir_opt in search_dirs.iter() {
        if let Some(dir) = dir_opt {
            if dir.exists() {
                // Try exact name match with .so extension
                let so_path = dir.join(format!("{}.so", name));
                if so_path.exists() {
                    return Vst2PluginInstance::load(&so_path);
                }

                // Try case-insensitive search
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(stem) = path.file_stem() {
                            if stem.to_string_lossy().to_lowercase() == name.to_lowercase() {
                                return Vst2PluginInstance::load(&path);
                            }
                        }
                    }
                }
            }
        }
    }

    Err(PluginError::NotFound(format!("VST2 plugin '{}' not found", name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_vst2_directory() {
        if let Some(home) = dirs::home_dir() {
            let vst_dir = home.join(".vst");
            if vst_dir.exists() {
                let plugins = scan_vst2_directory(&vst_dir);
                println!("Found {} VST2 plugins", plugins.len());
                for p in plugins.iter().take(5) {
                    println!("  - {}", p.id.name);
                }
            }
        }
    }
}
