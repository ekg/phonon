# VST3 GUI Support for Phonon

## Overview

Add native VST3 plugin GUI support on Linux (X11) to enable visual patch editing.

## Current State

- `rack` crate v0.4.8 provides VST3 audio processing
- GUI support exists for AU (macOS) but NOT for VST3
- `RackVST3Gui` struct is defined but has no implementation

## Architecture Decision

**Approach: Build extension module alongside rack**

Rather than forking rack, we'll create `phonon-vst3-gui` that:
1. Uses rack for plugin loading/processing
2. Adds C++ code for IPlugView/X11 integration
3. Can be contributed upstream later

## VST3 GUI Technical Requirements

### Linux/X11 Integration

From [Steinberg VST3 Documentation](https://steinbergmedia.github.io/vst3_doc/base/classSteinberg_1_1IPlugView.html):

1. **Get IEditController** from plugin component
2. **Call `createView(ViewType::kEditor)`** to get IPlugView
3. **Create X11 window** with XEmbed support
4. **Call `plugView->attached(window_id, "X11EmbedWindowID")`**
5. **Implement IPlugFrame** for resize callbacks

### Required X11 Libraries

```
libx11-xcb-dev
libxcb-util-dev
libxcb-cursor-dev
libxcb-xkb-dev
libxkbcommon-dev
libxkbcommon-x11-dev
libcairo2-dev
libpango1.0-dev
```

## Implementation Plan

### Phase 1: C++ VST3 GUI Wrapper

Create `phonon-vst3-gui-sys/src/vst3_gui.cpp`:

```cpp
#include "pluginterfaces/gui/iplugview.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include <X11/Xlib.h>

struct PhononVST3Gui {
    Steinberg::IPlugView* view;
    Display* display;
    Window window;
    // ... resize handling
};

// FFI exports
extern "C" {
    PhononVST3Gui* phonon_vst3_gui_create(RackVST3Plugin* plugin);
    void phonon_vst3_gui_show(PhononVST3Gui* gui);
    void phonon_vst3_gui_hide(PhononVST3Gui* gui);
    void phonon_vst3_gui_pump_events(PhononVST3Gui* gui);
    void phonon_vst3_gui_free(PhononVST3Gui* gui);
}
```

### Phase 2: Rust Bindings

Create `phonon-vst3-gui/src/lib.rs`:

```rust
pub struct Vst3Gui {
    handle: *mut ffi::PhononVST3Gui,
}

impl Vst3Gui {
    pub fn create(plugin: &mut Vst3Plugin) -> Result<Self>;
    pub fn show(&mut self);
    pub fn hide(&mut self);
    pub fn pump_events(&mut self);
}
```

### Phase 3: Integration with phonon-edit

```rust
// In modal editor
impl PluginBrowser {
    fn open_plugin_gui(&mut self, plugin_id: &str) {
        if let Some(plugin) = self.get_plugin(plugin_id) {
            let gui = Vst3Gui::create(plugin)?;
            gui.show();
            self.open_guis.insert(plugin_id.to_string(), gui);
        }
    }
}
```

## File Structure

```
phonon/
├── phonon-vst3-gui-sys/    # C++ wrapper + FFI
│   ├── Cargo.toml
│   ├── build.rs
│   └── src/
│       ├── vst3_gui.cpp
│       ├── vst3_gui.h
│       └── lib.rs (ffi bindings)
│
├── phonon-vst3-gui/        # Safe Rust wrapper
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
```

## Dependencies

- VST3 SDK (already bundled with rack)
- X11/XCB libraries
- raw-window-handle (for window integration)

## Event Loop Integration

Options:
1. **Separate thread** - GUI runs in its own X11 event loop
2. **Main thread polling** - `pump_events()` called periodically
3. **async/await** - Integrate with tokio/async-std

Recommendation: Start with option 2 (polling), can upgrade later.

## References

- [yabridge](https://github.com/robbert-vdh/yabridge) - Windows VST on Linux with GUI
- [Steinberg IPlugView](https://steinbergmedia.github.io/vst3_doc/base/classSteinberg_1_1IPlugView.html)
- [VST3 SDK editorhost example](https://github.com/steinbergmedia/vst3sdk)
