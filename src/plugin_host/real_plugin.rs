//! Real Plugin Instance using rack crate
//!
//! Wraps VST3 plugins via the rack crate for actual audio processing.
//! Provides conversion between Phonon's plugin types and rack's types.

use super::instance::MidiEvent as PhononMidiEvent;
use super::types::{
    ParameterInfo as PhononParameterInfo, PluginCategory, PluginError, PluginFormat, PluginId,
    PluginInfo as PhononPluginInfo, PluginResult,
};

// Import rack types - only available when vst3 feature is enabled
#[cfg(feature = "vst3")]
use rack::{Error as RackError, PluginInstance, PluginScanner};

/// Global lock serializing all VST3 FFI calls into the `rack` crate.
///
/// The underlying VST3 SDK / plugin shared libraries are NOT thread-safe:
/// scanning and loading `dlopen`s plugin `.so`s and runs their static
/// initializers, and instantiating/processing many plugins connects to the
/// X11 display. Doing any of these concurrently from multiple threads
/// segfaults (e.g. `cargo test` runs the VST3 integration tests in parallel by
/// default, which crashed the whole `test_vst3_plugins` binary with SIGSEGV).
///
/// Every entry point that reaches into `rack` acquires this lock so that at
/// most one VST3 FFI operation runs at a time, process-wide. The lock is held
/// only around the individual FFI call, never across another locking helper, so
/// there is no reentrant deadlock. Poisoning is recovered from — a panic in one
/// plugin call must not permanently disable plugin hosting.
#[cfg(feature = "vst3")]
fn vst3_ffi_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    // Make Xlib thread-safe before the first plugin (and thus the first X
    // connection) is ever loaded. This is idempotent and cheap after the first
    // call, and every VST3 FFI entry point funnels through this lock.
    ensure_x11_thread_safe();
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Ensure Xlib is put into thread-safe mode exactly once, before any X11 call.
///
/// JUCE-based VST3 plugins (Surge XT, Odin2, and every other plugin `scan()`
/// probes) spawn their own long-lived message/timer threads that talk to the
/// X display. Xlib is NOT thread-safe unless `XInitThreads()` is called before
/// the first Xlib call, so with multiple plugins' background threads live at
/// once (e.g. `cargo test` running the VST3 integration tests in parallel) the
/// process segfaults inside libX11. Every real JUCE host calls `XInitThreads()`
/// at startup for exactly this reason.
///
/// We can't link libX11 (the plugins `dlopen` it at runtime), so we `dlopen` it
/// ourselves with `RTLD_GLOBAL` — that resolves to the single process-wide
/// libX11 instance the plugins will also use — and call `XInitThreads()` once
/// via `dlsym`, guarded by a `Once` so it happens before the first plugin load.
#[cfg(all(feature = "vst3", target_os = "linux"))]
fn ensure_x11_thread_safe() {
    use std::sync::Once;
    static X11_INIT: Once = Once::new();
    X11_INIT.call_once(|| {
        // SAFETY: standard FFI. We dlopen libX11 and, if present, call
        // XInitThreads() before any other X11 call has run in this process
        // (this fires on the first VST3 FFI operation, ahead of any plugin
        // load). The library handle is intentionally leaked so libX11 stays
        // resident for the process lifetime.
        unsafe {
            let lib = libc::dlopen(
                c"libX11.so.6".as_ptr(),
                libc::RTLD_NOW | libc::RTLD_GLOBAL,
            );
            if lib.is_null() {
                return;
            }
            let sym = libc::dlsym(lib, c"XInitThreads".as_ptr());
            if sym.is_null() {
                return;
            }
            let x_init_threads: extern "C" fn() -> libc::c_int =
                std::mem::transmute(sym);
            let _ = x_init_threads();
        }
    });
}

/// No-op on non-Linux: Xlib thread-safety only applies to the X11 backend.
#[cfg(all(feature = "vst3", not(target_os = "linux")))]
fn ensure_x11_thread_safe() {}

/// Whether the current environment can safely host (JUCE-based) VST3 plugins.
///
/// On Linux, JUCE VST3 plugins spin up their own message/timer threads that
/// connect to an X display as soon as their *module* is loaded — which happens
/// during a plain metadata scan (`rack`'s scanner `dlopen`s + runs `ModuleEntry`
/// for every plugin in the system search paths), even for plugins we never
/// instantiate. With no reachable X display those threads race on the failing
/// connection and segfault the whole process. This is exactly what crashed the
/// `test_vst3_plugins` binary (SIGSEGV, uncounted) in this headless/Wayland
/// environment, and it reproduces even single-threaded because the race is
/// among the plugins' own background threads, not the test threads.
///
/// There is no host-side way to stop a plugin from spawning those threads, so
/// we simply refuse to load any real plugin unless a display is actually
/// available. The result is probed exactly once — before any plugin is loaded —
/// and cached.
///
/// Escape hatch: set `PHONON_VST3_FORCE_LOAD=1` to bypass the check (e.g. a
/// non-JUCE plugin set, or a platform where this heuristic is too conservative).
#[cfg(feature = "vst3")]
pub fn vst3_runtime_available() -> bool {
    use std::sync::OnceLock;
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(probe_vst3_runtime)
}

#[cfg(all(feature = "vst3", target_os = "linux"))]
fn probe_vst3_runtime() -> bool {
    if std::env::var_os("PHONON_VST3_FORCE_LOAD").is_some() {
        return true;
    }

    // Ensure Xlib is thread-safe before we (or, later, any plugin) touch X.
    ensure_x11_thread_safe();

    // Try to open an X display ourselves — single-threaded and fully controlled,
    // before any plugin is loaded. If we cannot, JUCE plugins cannot either, so
    // it is not safe to load them.
    //
    // SAFETY: standard FFI. dlopen libX11, resolve XOpenDisplay/XCloseDisplay,
    // open a display with the default (NULL) name and immediately close it if it
    // succeeds. The library handle is intentionally left resident.
    unsafe {
        let lib = libc::dlopen(
            c"libX11.so.6".as_ptr(),
            libc::RTLD_NOW | libc::RTLD_GLOBAL,
        );
        if lib.is_null() {
            return false;
        }
        let open = libc::dlsym(lib, c"XOpenDisplay".as_ptr());
        let close = libc::dlsym(lib, c"XCloseDisplay".as_ptr());
        if open.is_null() || close.is_null() {
            return false;
        }
        let x_open_display: extern "C" fn(*const libc::c_char) -> *mut libc::c_void =
            std::mem::transmute(open);
        let x_close_display: extern "C" fn(*mut libc::c_void) -> libc::c_int =
            std::mem::transmute(close);

        let display = x_open_display(std::ptr::null());
        if display.is_null() {
            return false;
        }
        x_close_display(display);
        true
    }
}

/// On macOS/Windows the VST3 hosting path used here does not have the headless-X
/// crash, so assume the runtime is available.
#[cfg(all(feature = "vst3", not(target_os = "linux")))]
fn probe_vst3_runtime() -> bool {
    true
}

/// Real VST3 plugin instance using rack crate
#[cfg(feature = "vst3")]
pub struct RealPluginInstance {
    /// The rack plugin instance (Option so we can take it for leaking)
    plugin: Option<rack::Plugin>,
    /// Phonon-style plugin info
    info: PhononPluginInfo,
    /// Sample rate
    sample_rate: f32,
    /// Max block size
    max_block_size: usize,
    /// Whether initialized
    initialized: bool,
    /// Cached actual input channel count (determined on first process call)
    /// None = not yet determined, Some(0) = no inputs, Some(2) = stereo inputs
    actual_input_channels: Option<usize>,
}

#[cfg(feature = "vst3")]
impl RealPluginInstance {
    /// Create a new real plugin instance from rack PluginInfo
    pub fn from_rack_info(
        scanner: &rack::Scanner,
        rack_info: &rack::PluginInfo,
    ) -> PluginResult<Self> {
        let plugin = {
            let _guard = vst3_ffi_lock();
            scanner
                .load(rack_info)
                .map_err(|e: RackError| PluginError::LoadError(e.to_string()))?
        };

        let info = convert_plugin_info(rack_info);

        Ok(Self {
            plugin: Some(plugin),
            info,
            sample_rate: 44100.0,
            max_block_size: 512,
            initialized: false,
            actual_input_channels: None,
        })
    }

    /// Leak the plugin instance to prevent cleanup crash
    /// Call this before dropping to avoid double-free bugs in VST3 SDK
    pub fn leak(mut self) {
        if let Some(plugin) = self.plugin.take() {
            std::mem::forget(plugin);
        }
    }

    /// Get a reference to the plugin, panics if leaked
    fn plugin(&self) -> &rack::Plugin {
        self.plugin.as_ref().expect("Plugin was leaked")
    }

    /// Get a mutable reference to the plugin, panics if leaked
    fn plugin_mut(&mut self) -> &mut rack::Plugin {
        self.plugin.as_mut().expect("Plugin was leaked")
    }

    /// Initialize the plugin with sample rate and block size
    pub fn initialize(&mut self, sample_rate: f32, max_block_size: usize) -> PluginResult<()> {
        {
            let _guard = vst3_ffi_lock();
            self.plugin_mut()
                .initialize(sample_rate as f64, max_block_size)
                .map_err(|e| PluginError::InitError(e.to_string()))?;
        }

        self.sample_rate = sample_rate;
        self.max_block_size = max_block_size;
        self.initialized = true;
        Ok(())
    }

    /// Check if plugin is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get plugin info
    pub fn info(&self) -> &PhononPluginInfo {
        &self.info
    }

    /// Process audio through the plugin (for effects)
    pub fn process(
        &mut self,
        inputs: &[&[f32]],
        outputs: &mut [&mut [f32]],
        num_samples: usize,
    ) -> PluginResult<()> {
        if !self.initialized {
            return Err(PluginError::ProcessError(
                "Plugin not initialized".to_string(),
            ));
        }

        let _guard = vst3_ffi_lock();
        self.plugin_mut()
            .process(inputs, outputs, num_samples)
            .map_err(|e| PluginError::ProcessError(e.to_string()))
    }

    /// Process with MIDI input (for instruments)
    pub fn process_with_midi(
        &mut self,
        midi_events: &[PhononMidiEvent],
        outputs: &mut [&mut [f32]],
        num_samples: usize,
    ) -> PluginResult<()> {
        if !self.initialized {
            return Err(PluginError::ProcessError(
                "Plugin not initialized".to_string(),
            ));
        }

        // Serialize the whole send_midi + process sequence against all other
        // VST3 FFI activity so it stays atomic and can't race another plugin.
        let _guard = vst3_ffi_lock();

        // Convert Phonon MIDI events to rack MIDI events
        let rack_events: Vec<rack::MidiEvent> = midi_events
            .iter()
            .map(convert_midi_event)
            .collect();

        // Send MIDI events to the plugin
        if !rack_events.is_empty() {
            self.plugin_mut()
                .send_midi(&rack_events)
                .map_err(|e| PluginError::ProcessError(format!("MIDI error: {}", e)))?;
        }

        // Determine actual input channel count (cached after first successful call)
        // Some instruments (like Surge XT) want 2 inputs despite being instrument type
        // Others (like Odin2) want exactly 0 inputs
        let use_stereo_inputs = match self.actual_input_channels {
            Some(count) => count > 0,
            None => {
                // First call - try stereo first (most plugins accept this)
                let input_left = vec![0.0f32; num_samples];
                let input_right = vec![0.0f32; num_samples];
                let inputs: Vec<&[f32]> = vec![&input_left, &input_right];
                let result = self.plugin_mut().process(&inputs, outputs, num_samples);

                match &result {
                    Ok(()) => {
                        // Stereo inputs work - cache this
                        self.actual_input_channels = Some(2);
                        return Ok(());
                    }
                    Err(e) if e.to_string().contains("expects 0") => {
                        // Plugin wants 0 inputs - cache this and try empty
                        self.actual_input_channels = Some(0);
                        false
                    }
                    Err(e) => {
                        // Some other error - propagate it
                        return Err(PluginError::ProcessError(e.to_string()));
                    }
                }
            }
        };

        if use_stereo_inputs {
            // Effect or instrument with inputs: pass stereo silent inputs
            let input_left = vec![0.0f32; num_samples];
            let input_right = vec![0.0f32; num_samples];
            let inputs: Vec<&[f32]> = vec![&input_left, &input_right];
            self.plugin_mut()
                .process(&inputs, outputs, num_samples)
                .map_err(|e| PluginError::ProcessError(e.to_string()))
        } else {
            // Pure instrument: pass empty inputs
            let inputs: Vec<&[f32]> = vec![];
            self.plugin_mut()
                .process(&inputs, outputs, num_samples)
                .map_err(|e| PluginError::ProcessError(e.to_string()))
        }
    }

    /// Get parameter value by index
    pub fn get_parameter(&self, index: usize) -> PluginResult<f32> {
        let _guard = vst3_ffi_lock();
        self.plugin()
            .get_parameter(index)
            .map_err(|e| PluginError::ParameterError(e.to_string()))
    }

    /// Set parameter value by index (normalized 0.0 - 1.0)
    pub fn set_parameter(&mut self, index: usize, value: f32) -> PluginResult<()> {
        let _guard = vst3_ffi_lock();
        self.plugin_mut()
            .set_parameter(index, value)
            .map_err(|e| PluginError::ParameterError(e.to_string()))
    }

    /// Get number of parameters
    pub fn parameter_count(&self) -> usize {
        let _guard = vst3_ffi_lock();
        self.plugin().parameter_count()
    }

    /// Get parameter info by index
    pub fn parameter_info(&self, index: usize) -> PluginResult<PhononParameterInfo> {
        let rack_param = {
            let _guard = vst3_ffi_lock();
            self.plugin()
                .parameter_info(index)
                .map_err(|e| PluginError::ParameterError(e.to_string()))?
        };

        Ok(PhononParameterInfo {
            index: rack_param.index,
            name: rack_param.name.clone(),
            short_name: rack_param.name.clone(), // rack doesn't have short_name
            default_value: rack_param.default,
            min_value: rack_param.min,
            max_value: rack_param.max,
            unit: rack_param.unit.clone(),
            step_count: 0,
            automatable: true,
        })
    }

    /// Find parameter name by VST3 parameter ID
    /// VST3 param IDs can be hashed values (like Surge XT uses)
    /// Returns (index, name) if found
    pub fn find_parameter_by_id(&self, param_id: u32) -> Option<(usize, String)> {
        let count = self.parameter_count();
        for idx in 0..count {
            if let Ok(info) = self.parameter_info(idx) {
                // The parameter index in rack corresponds to VST3's param_id for most plugins
                // But some plugins (Surge XT) use hashed IDs
                // Check if the index matches the param_id
                if info.index as u32 == param_id {
                    return Some((idx, info.name));
                }
            }
        }
        // If no match by index, the param_id might be a hash
        // In this case, we can't resolve the name, return None
        None
    }

    /// Set parameter by VST3 parameter ID (not index)
    /// Searches for the parameter index that matches the ID
    pub fn set_parameter_by_id(&mut self, param_id: u32, value: f32) -> PluginResult<()> {
        // For most VST3 plugins, param_id == index
        // Try direct index first
        let count = self.parameter_count();
        if (param_id as usize) < count {
            return self.set_parameter(param_id as usize, value);
        }
        // If param_id is larger than count, it might be a hashed ID
        // We'd need to search through parameters, but rack doesn't expose the ID
        // Fall back to treating param_id as index (clamped)
        Err(PluginError::ParameterError(format!(
            "Parameter ID {} out of range (count: {})",
            param_id, count
        )))
    }

    /// Get plugin state as bytes
    pub fn get_state(&self) -> PluginResult<Vec<u8>> {
        let _guard = vst3_ffi_lock();
        self.plugin()
            .get_state()
            .map_err(|e| PluginError::PresetError(e.to_string()))
    }

    /// Set plugin state from bytes
    pub fn set_state(&mut self, data: &[u8]) -> PluginResult<()> {
        let _guard = vst3_ffi_lock();
        self.plugin_mut()
            .set_state(data)
            .map_err(|e| PluginError::PresetError(e.to_string()))
    }

    /// Reset the plugin state
    pub fn reset(&mut self) -> PluginResult<()> {
        let _guard = vst3_ffi_lock();
        self.plugin_mut()
            .reset()
            .map_err(|e| PluginError::ProcessError(e.to_string()))
    }

    /// Get parameter changes from plugin GUI since last call
    ///
    /// Returns a list of (param_id, value) tuples for parameters that were
    /// changed by the user in the plugin GUI.
    #[cfg(target_os = "linux")]
    pub fn get_param_changes(&mut self) -> PluginResult<Vec<(u32, f64)>> {
        let _guard = vst3_ffi_lock();
        self.plugin_mut()
            .get_param_changes()
            .map_err(|e| PluginError::ParameterError(e.to_string()))
    }

    /// Get number of factory presets
    pub fn preset_count(&self) -> PluginResult<usize> {
        let _guard = vst3_ffi_lock();
        self.plugin()
            .preset_count()
            .map_err(|e| PluginError::PresetError(e.to_string()))
    }

    /// Load factory preset by index
    pub fn load_preset(&mut self, preset_number: i32) -> PluginResult<()> {
        let _guard = vst3_ffi_lock();
        self.plugin_mut()
            .load_preset(preset_number)
            .map_err(|e| PluginError::PresetError(e.to_string()))
    }

    /// Create a GUI window for this plugin
    #[cfg(target_os = "linux")]
    pub fn create_gui(&mut self) -> PluginResult<rack::Vst3Gui> {
        let _guard = vst3_ffi_lock();
        rack::Vst3Gui::create(self.plugin_mut())
            .map_err(|e| PluginError::ProcessError(format!("GUI error: {}", e)))
    }

    /// Get plugin name
    pub fn name(&self) -> &str {
        &self.info.id.name
    }
}

/// Convert rack PluginInfo to Phonon PluginInfo
#[cfg(feature = "vst3")]
pub fn convert_plugin_info(rack_info: &rack::PluginInfo) -> PhononPluginInfo {
    let category = match rack_info.plugin_type {
        rack::PluginType::Instrument => PluginCategory::Instrument,
        rack::PluginType::Effect => PluginCategory::Effect,
        rack::PluginType::Mixer => PluginCategory::Effect,
        rack::PluginType::Analyzer => PluginCategory::Analyzer,
        _ => PluginCategory::Unknown,
    };

    let (num_inputs, num_outputs) = match rack_info.plugin_type {
        rack::PluginType::Instrument => (0, 2),
        rack::PluginType::Effect => (2, 2),
        _ => (2, 2),
    };

    PhononPluginInfo {
        id: PluginId {
            format: PluginFormat::Vst3,
            identifier: rack_info.unique_id.clone(),
            name: rack_info.name.clone(),
        },
        vendor: rack_info.manufacturer.clone(),
        version: format!("{}", rack_info.version),
        category,
        num_inputs,
        num_outputs,
        parameters: vec![], // Will be populated when plugin is loaded
        factory_presets: vec![],
        has_gui: true, // Assume GUI support
        path: rack_info.path.to_string_lossy().to_string(),
    }
}

/// Convert Phonon MidiEvent to rack MidiEvent
#[cfg(feature = "vst3")]
pub fn convert_midi_event(event: &PhononMidiEvent) -> rack::MidiEvent {
    let sample_offset = event.sample_offset as u32;

    if event.is_note_on() {
        rack::MidiEvent::note_on(
            event.data1, // note
            event.data2, // velocity
            event.channel(),
            sample_offset,
        )
    } else if event.is_note_off() {
        rack::MidiEvent::note_off(
            event.data1, // note
            0,           // release velocity
            event.channel(),
            sample_offset,
        )
    } else {
        // Control change or other
        let status_type = event.status & 0xF0;
        match status_type {
            0xB0 => rack::MidiEvent::control_change(
                event.data1,
                event.data2,
                event.channel(),
                sample_offset,
            ),
            0xC0 => rack::MidiEvent::program_change(event.data1, event.channel(), sample_offset),
            0xE0 => {
                // Pitch bend - combine data1 and data2 into 14-bit value
                let value = (event.data2 as u16) << 7 | (event.data1 as u16);
                rack::MidiEvent::pitch_bend(value, event.channel(), sample_offset)
            }
            _ => {
                // Default to note on with the raw data
                rack::MidiEvent::note_on(event.data1, event.data2, event.channel(), sample_offset)
            }
        }
    }
}

/// Create a RealPluginInstance from a plugin path
#[cfg(feature = "vst3")]
pub fn create_real_plugin_from_path(path: &std::path::Path) -> PluginResult<RealPluginInstance> {
    // Refuse to load if the host environment can't safely run JUCE VST3 plugins
    // (e.g. no reachable X display). Loading — even just scanning — would spawn
    // plugin threads that segfault the process. See `vst3_runtime_available`.
    if !vst3_runtime_available() {
        return Err(PluginError::NotSupported(
            "VST3 host environment unavailable (no reachable display); \
             refusing to load plugins. Set PHONON_VST3_FORCE_LOAD=1 to override."
                .to_string(),
        ));
    }

    // Create a fresh scanner for each load - avoids shared state issues
    let scanner = {
        let _guard = vst3_ffi_lock();
        rack::Scanner::new().map_err(|e: RackError| PluginError::ScanError(e.to_string()))?
    };

    // Scan the specific path to find the plugin
    let plugins = {
        let _guard = vst3_ffi_lock();
        scanner
            .scan_path(path)
            .map_err(|e: RackError| PluginError::ScanError(e.to_string()))?
    };

    if plugins.is_empty() {
        return Err(PluginError::NotFound(format!("No plugin found at: {}", path.display())));
    }

    RealPluginInstance::from_rack_info(&scanner, &plugins[0])
}

/// Create a RealPluginInstance by name (scans system paths)
#[cfg(feature = "vst3")]
pub fn create_real_plugin_by_name(name: &str) -> PluginResult<RealPluginInstance> {
    // Refuse to load if the host environment can't safely run JUCE VST3 plugins
    // (e.g. no reachable X display). Scanning by name `dlopen`s + initialises
    // EVERY system plugin, each spawning threads that segfault the process in a
    // headless environment. See `vst3_runtime_available`.
    if !vst3_runtime_available() {
        return Err(PluginError::NotSupported(
            "VST3 host environment unavailable (no reachable display); \
             refusing to load plugins. Set PHONON_VST3_FORCE_LOAD=1 to override."
                .to_string(),
        ));
    }

    // Create a fresh scanner for each load - avoids shared state issues
    let scanner = {
        let _guard = vst3_ffi_lock();
        rack::Scanner::new().map_err(|e: RackError| PluginError::ScanError(e.to_string()))?
    };

    // Scan for all plugins
    let plugins = {
        let _guard = vst3_ffi_lock();
        scanner
            .scan()
            .map_err(|e: RackError| PluginError::ScanError(e.to_string()))?
    };

    // Find the plugin by name (case-insensitive, prefer exact match)
    let name_lower = name.to_lowercase();
    let exact_match = plugins.iter().find(|p| p.name.to_lowercase() == name_lower);
    let prefix_match = plugins.iter().find(|p| p.name.to_lowercase().starts_with(&name_lower));

    let matching = exact_match.or(prefix_match);

    match matching {
        Some(info) => {
            tracing::info!("Loading plugin: {} from {}", info.name, info.path.display());
            RealPluginInstance::from_rack_info(&scanner, info)
        }
        None => Err(PluginError::NotFound(format!("Plugin not found: {}", name))),
    }
}

/// Plugin scanner using rack
#[cfg(feature = "vst3")]
pub struct RealPluginScanner {
    scanner: rack::Scanner,
}

#[cfg(feature = "vst3")]
impl RealPluginScanner {
    /// Create a new scanner
    pub fn new() -> PluginResult<Self> {
        let _guard = vst3_ffi_lock();
        let scanner = rack::Scanner::new()
            .map_err(|e: RackError| PluginError::ScanError(e.to_string()))?;
        Ok(Self { scanner })
    }

    /// Scan for plugins in system paths
    ///
    /// Scanning `dlopen`s + initialises every installed plugin module, which is
    /// unsafe on a host that can't run them (see `vst3_runtime_available`), so
    /// this refuses to scan there rather than crash the process.
    pub fn scan(&self) -> PluginResult<Vec<rack::PluginInfo>> {
        if !vst3_runtime_available() {
            return Err(PluginError::NotSupported(
                "VST3 host environment unavailable (no reachable display); \
                 refusing to scan plugins. Set PHONON_VST3_FORCE_LOAD=1 to override."
                    .to_string(),
            ));
        }
        let _guard = vst3_ffi_lock();
        self.scanner
            .scan()
            .map_err(|e: RackError| PluginError::ScanError(e.to_string()))
    }

    /// Scan a specific path
    pub fn scan_path(&self, path: &std::path::Path) -> PluginResult<Vec<rack::PluginInfo>> {
        if !vst3_runtime_available() {
            return Err(PluginError::NotSupported(
                "VST3 host environment unavailable (no reachable display); \
                 refusing to scan plugins. Set PHONON_VST3_FORCE_LOAD=1 to override."
                    .to_string(),
            ));
        }
        let _guard = vst3_ffi_lock();
        self.scanner
            .scan_path(path)
            .map_err(|e: RackError| PluginError::ScanError(e.to_string()))
    }

    /// Load a plugin from PluginInfo
    pub fn load(&self, info: &rack::PluginInfo) -> PluginResult<RealPluginInstance> {
        RealPluginInstance::from_rack_info(&self.scanner, info)
    }

    /// Get the underlying rack scanner
    pub fn inner(&self) -> &rack::Scanner {
        &self.scanner
    }
}

#[cfg(feature = "vst3")]
impl Default for RealPluginScanner {
    fn default() -> Self {
        Self::new().expect("Failed to create plugin scanner")
    }
}

// Stub implementations when VST3 feature is not enabled
#[cfg(not(feature = "vst3"))]
pub struct RealPluginInstance {
    _private: (),
}

#[cfg(not(feature = "vst3"))]
impl RealPluginInstance {
    pub fn initialize(&mut self, _sample_rate: f32, _max_block_size: usize) -> PluginResult<()> {
        Err(PluginError::NotSupported(
            "VST3 support not available (feature not enabled)".to_string(),
        ))
    }

    pub fn is_initialized(&self) -> bool {
        false
    }

    pub fn info(&self) -> &PhononPluginInfo {
        unimplemented!("VST3 not available")
    }

    pub fn process(
        &mut self,
        _inputs: &[&[f32]],
        _outputs: &mut [&mut [f32]],
        _num_samples: usize,
    ) -> PluginResult<()> {
        Err(PluginError::NotSupported(
            "VST3 support not available".to_string(),
        ))
    }

    pub fn process_with_midi(
        &mut self,
        _midi_events: &[PhononMidiEvent],
        _outputs: &mut [&mut [f32]],
        _num_samples: usize,
    ) -> PluginResult<()> {
        Err(PluginError::NotSupported(
            "VST3 support not available".to_string(),
        ))
    }
}

#[cfg(not(feature = "vst3"))]
pub struct RealPluginScanner {
    _private: (),
}

#[cfg(not(feature = "vst3"))]
impl RealPluginScanner {
    pub fn new() -> PluginResult<Self> {
        Err(PluginError::NotSupported(
            "VST3 support not available (feature not enabled)".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "vst3")]
    fn test_scanner_creation() {
        // This test only runs when VST3 feature is enabled
        let result = RealPluginScanner::new();
        assert!(result.is_ok(), "Should be able to create scanner");
    }

    #[test]
    fn test_midi_event_conversion() {
        // Test note on conversion
        let note_on = PhononMidiEvent::note_on(0, 0, 60, 100);
        assert!(note_on.is_note_on());

        // Test note off conversion
        let note_off = PhononMidiEvent::note_off(100, 0, 60);
        assert!(note_off.is_note_off());
    }
}
