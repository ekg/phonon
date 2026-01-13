# Handoff: VST3 GUI Parameter Synchronization

## Summary

When a user adjusts parameters in a VST3 plugin's GUI (e.g., turning a knob in Surge XT), those changes are now automatically reflected in the phonon source code in real-time. This enables a workflow where you can:

1. Open a plugin GUI (Alt+G in phonon)
2. Tweak knobs/sliders in the GUI
3. See the parameter values update live in your phonon code

## Implementation Overview

### Changes Made to `rack` (https://github.com/ekg/rack)

1. **Native Linux VST3 Parameter Change Notifications**

   Added `IComponentHandler` implementation in `rack-sys/src/vst3_instance.cpp`:
   - `ComponentHandler` class captures GUI parameter changes via `performEdit()`
   - Uses circular buffer for lock-free single-producer queue
   - Handler is registered via `setComponentHandler()` on the IEditController

2. **C API Addition** (`rack-sys/include/rack_vst3.h`):
   ```c
   typedef struct {
       uint32_t param_id;      // Parameter ID
       double value;           // Normalized value (0.0 to 1.0)
   } RackVST3ParamChange;

   int rack_vst3_plugin_get_param_changes(
       RackVST3Plugin* plugin,
       RackVST3ParamChange* changes,
       uint32_t max_changes
   );
   ```

3. **Rust FFI Bindings** (`src/vst3/ffi.rs`):
   ```rust
   #[repr(C)]
   pub struct RackVST3ParamChange {
       pub param_id: u32,
       pub value: f64,
   }

   pub fn rack_vst3_plugin_get_param_changes(...) -> c_int;
   ```

4. **Safe Rust Wrapper** (`src/vst3/instance.rs`):
   ```rust
   pub fn get_param_changes(&mut self) -> Result<Vec<(u32, f64)>>
   ```

### Changes Made to `phonon`

1. **RealPluginInstance Wrapper** (`src/plugin_host/real_plugin.rs`):
   ```rust
   #[cfg(target_os = "linux")]
   pub fn get_param_changes(&mut self) -> PluginResult<Vec<(u32, f64)>>
   ```

2. **Modal Editor Integration** (`src/modal_editor/mod.rs`):

   - `poll_vst3_param_changes()` - Called in main loop after GUI event pumping
     - Collects parameter changes from all open GUIs
     - Maps param_id back to parameter names
     - Updates console with change notifications
     - Updates phonon source code

   - `update_plugin_param_in_content()` - Finds plugin definition line and updates it

   - `update_param_in_line()` - Handles the text manipulation:
     - If parameter exists: updates the value
     - If parameter doesn't exist: appends `# param_name value`

## How It Works

1. **In the Main Event Loop** (every ~100ms):
   ```
   pump_vst3_gui_events()  ->  poll_vst3_param_changes()
   ```

2. **Parameter Change Flow**:
   ```
   Plugin GUI knob turn
       -> IComponentHandler::performEdit() called by plugin
       -> Stored in circular buffer
       -> poll_vst3_param_changes() retrieves changes
       -> Mapped to parameter name via parameter_info()
       -> Console shows: "🎛️ ~surge:1 # cutoff 0.750"
       -> Content updated: ~surge:1 $ vst "Surge" # cutoff 0.750
   ```

3. **Line Update Algorithm**:
   - Find line starting with `~{instance_name}` that contains `$` or `:`
   - Look for existing `# param_name ` pattern
   - If found: replace the value (up to next `#` or EOL)
   - If not found: append `# param_name value` to end

## Usage in Phonon

1. Write phonon code with a VST plugin:
   ```phonon
   ~synth $ vst "Surge XT" # note "c4 e4 g4"
   out $ ~synth
   ```

2. Evaluate the code (Ctrl+X)

3. Open the plugin GUI (Alt+G)

4. Turn knobs in the GUI - watch the code update:
   ```phonon
   ~synth $ vst "Surge XT" # note "c4 e4 g4" # A Osc 1 Pitch 0.523
   out $ ~synth
   ```

## Key Files

### In rack repo:
- `rack-sys/src/vst3_instance.cpp` - ComponentHandler implementation
- `rack-sys/include/rack_vst3.h` - C API (RackVST3ParamChange, get_param_changes)
- `src/vst3/ffi.rs` - Rust FFI bindings
- `src/vst3/instance.rs` - Safe Rust wrapper

### In phonon repo:
- `src/plugin_host/real_plugin.rs` - RealPluginInstance::get_param_changes()
- `src/modal_editor/mod.rs`:
  - Line ~800: poll_vst3_param_changes() call in main loop
  - Line ~2659: poll_vst3_param_changes() method
  - Line ~2715: update_plugin_param_in_content() method
  - Line ~2742: update_param_in_line() method

## Wine Host Support

The Wine host (`rack-wine-host/`) also has parameter change support:
- `src/main.cpp` - HostComponentHandler class
- `include/protocol.h` - CMD_GET_PARAM_CHANGES protocol
- Rust bindings in `src/wine_host/`

This enables the same parameter sync workflow for Windows VST3 plugins running via Wine.

## Testing

1. Load phonon with a simple VST code
2. Open GUI (Alt+G)
3. Move a parameter
4. Verify console shows the change
5. Verify the source code line is updated

## Future Improvements

1. **Undo Support**: Consider pushing undo state before param updates
2. **Throttling**: May want to throttle updates if too frequent
3. **Multi-line Definitions**: Handle plugins defined across multiple lines
4. **Expression Preservation**: Avoid replacing LFO expressions like `(~lfo * 0.5)`
