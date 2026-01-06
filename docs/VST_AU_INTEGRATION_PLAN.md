# VST/AU Plugin Integration Plan

## Vision

Enable Phonon to host external audio plugins (VST3, AU, CLAP, LV2), bringing professional synthesizers and effects into the live-coding workflow with pattern-controlled parameters.

```phonon
-- Load a Virus emulation, pattern-control the filter
~lfo $ sine 0.25
~virus $ vst "Osirus" # cutoff (~lfo * 0.5 + 0.5) # note "c4 e4 g4"
out $ ~virus * 0.7
```

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│  Phonon DSL                                                  │
│  ~synth $ vst "PluginName" # param value # note "pattern"   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Compiler (compositional_compiler.rs)                        │
│  - Parse vst/au/clap function calls                         │
│  - Resolve plugin by name from registry                     │
│  - Compile parameter patterns to Signals                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  SignalNode::PluginInstance                                  │
│  - plugin_id: PluginId (format + path/name)                 │
│  - params: HashMap<String, Signal>                          │
│  - midi_input: Option<Signal>                               │
│  - preset: Option<PresetState>                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Plugin Host Layer (src/plugin_host/)                        │
│  - PluginRegistry: scan, cache, lookup                      │
│  - PluginInstance: load, process, params, state             │
│  - Uses: Rack library for cross-platform hosting            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  External Plugins (VST3, AU, CLAP, LV2)                     │
│  - Gearmulator (Virus, microQ, Nord, etc.)                  │
│  - Commercial plugins (Diva, Serum, etc.)                   │
│  - Free plugins (Vital, Surge, etc.)                        │
└─────────────────────────────────────────────────────────────┘
```

## Dependency DAG

```
                    ┌─────────────────┐
                    │  Phase 0: Core  │
                    │  Dependencies   │
                    └────────┬────────┘
                             │
            ┌────────────────┼────────────────┐
            ▼                ▼                ▼
   ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐
   │ 1a: Plugin  │  │ 1b: Plugin  │  │ 1c: MIDI Event  │
   │   Types &   │  │  Registry   │  │    Pattern      │
   │   Traits    │  │  & Scanner  │  │   Integration   │
   └──────┬──────┘  └──────┬──────┘  └────────┬────────┘
          │                │                   │
          └────────────────┼───────────────────┘
                           ▼
                  ┌─────────────────┐
                  │ 2: PluginInstance│
                  │   SignalNode    │
                  └────────┬────────┘
                           │
            ┌──────────────┼──────────────┐
            ▼              ▼              ▼
   ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
   │ 3a: DSL     │ │ 3b: Buffer  │ │ 3c: Param   │
   │  Compiler   │ │ Processing  │ │  Automation │
   │ Integration │ │ Integration │ │  & Mapping  │
   └──────┬──────┘ └──────┬──────┘ └──────┬──────┘
          │               │               │
          └───────────────┼───────────────┘
                          ▼
                 ┌─────────────────┐
                 │  4: Preset &    │
                 │  State System   │
                 └────────┬────────┘
                          │
            ┌─────────────┼─────────────┐
            ▼             ▼             ▼
   ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
   │ 5a: GUI     │ │ 5b: Plugin  │ │ 5c: Tab     │
   │  Hosting    │ │  Discovery  │ │ Completion  │
   │ (Optional)  │ │    CLI      │ │  & REPL     │
   └─────────────┘ └─────────────┘ └─────────────┘
```

## Implementation Phases

### Phase 0: Core Dependencies
**No internal dependencies - can start immediately**

- [ ] Add Rack library dependency (or vendor if needed)
- [ ] Verify Rack builds on target platforms (Linux, macOS)
- [ ] Create `src/plugin_host/` module structure

### Phase 1: Foundation Layer (Parallel)

#### 1a: Plugin Types & Traits
**Depends on: Phase 0**

```rust
// src/plugin_host/types.rs

/// Supported plugin formats
#[derive(Clone, Debug, PartialEq)]
pub enum PluginFormat {
    Vst3,
    AudioUnit,
    Clap,
    Lv2,
}

/// Unique plugin identifier
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct PluginId {
    pub format: PluginFormat,
    pub identifier: String,  // Bundle ID (AU) or path (VST3)
    pub name: String,        // Human-readable name
}

/// Plugin metadata from scanning
#[derive(Clone, Debug)]
pub struct PluginInfo {
    pub id: PluginId,
    pub vendor: String,
    pub version: String,
    pub num_inputs: usize,
    pub num_outputs: usize,
    pub parameters: Vec<ParameterInfo>,
    pub has_gui: bool,
}

/// Parameter metadata
#[derive(Clone, Debug)]
pub struct ParameterInfo {
    pub index: usize,
    pub name: String,
    pub short_name: String,
    pub default_value: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub unit: String,
}

/// Preset state (opaque blob)
#[derive(Clone, Debug)]
pub struct PresetState {
    pub name: String,
    pub data: Vec<u8>,
}
```

#### 1b: Plugin Registry & Scanner
**Depends on: Phase 0, 1a**

```rust
// src/plugin_host/registry.rs

pub struct PluginRegistry {
    plugins: HashMap<String, PluginInfo>,  // name -> info
    scanner: rack::Scanner,
    cache_path: PathBuf,
}

impl PluginRegistry {
    /// Scan system plugin paths
    pub fn scan(&mut self) -> Result<Vec<PluginInfo>, Error>;

    /// Load cached scan results
    pub fn load_cache(&mut self) -> Result<(), Error>;

    /// Save scan results to cache
    pub fn save_cache(&self) -> Result<(), Error>;

    /// Lookup plugin by name (fuzzy match)
    pub fn find(&self, name: &str) -> Option<&PluginInfo>;

    /// List all plugins matching pattern
    pub fn search(&self, pattern: &str) -> Vec<&PluginInfo>;
}
```

#### 1c: MIDI Event Pattern Integration
**Depends on: Phase 0**

```rust
// Extend existing pattern system for MIDI events

/// MIDI event for plugin input
#[derive(Clone, Debug)]
pub struct MidiEvent {
    pub sample_offset: usize,
    pub status: u8,
    pub data1: u8,
    pub data2: u8,
}

/// Convert note patterns to MIDI events
pub fn pattern_to_midi_events(
    pattern: &Pattern<Note>,
    start_sample: usize,
    num_samples: usize,
    sample_rate: f32,
) -> Vec<MidiEvent>;
```

### Phase 2: Plugin Instance Node
**Depends on: 1a, 1b, 1c**

```rust
// src/unified_graph.rs - new SignalNode variant

SignalNode::PluginInstance {
    /// Plugin identifier
    plugin_id: PluginId,

    /// Audio inputs (for effects)
    audio_inputs: Vec<Signal>,

    /// MIDI input (for instruments)
    midi_input: Option<Signal>,

    /// Parameter automation
    params: HashMap<String, Signal>,

    /// Initial preset state
    preset: Option<PresetState>,

    /// Runtime handle (populated during graph setup)
    #[serde(skip)]
    instance: Option<Arc<Mutex<PluginInstanceHandle>>>,
}
```

### Phase 3: Integration Layer (Parallel)

#### 3a: DSL Compiler Integration
**Depends on: Phase 2**

```rust
// src/compositional_compiler.rs

/// Compile vst/au/clap function calls
fn compile_vst(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Parse: vst "PluginName" [preset] [params...]
    let plugin_name = extract_string(&args[0])?;
    let plugin_info = ctx.plugin_registry.find(&plugin_name)
        .ok_or_else(|| format!("Plugin not found: {}", plugin_name))?;

    // Handle chain operator for params
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    // Compile parameter patterns
    let mut param_signals = HashMap::new();
    for (name, expr) in extract_named_params(&params)? {
        let signal = compile_expr(ctx, expr)?;
        param_signals.insert(name, Signal::Node(signal));
    }

    // Create node
    let node = SignalNode::PluginInstance {
        plugin_id: plugin_info.id.clone(),
        audio_inputs: vec![input_signal],
        midi_input: None,
        params: param_signals,
        preset: None,
        instance: None,
    };

    Ok(ctx.graph.add_node(node))
}
```

#### 3b: Buffer Processing Integration
**Depends on: Phase 2**

```rust
// src/unified_graph.rs - eval_buffer implementation

SignalNode::PluginInstance { plugin_id, audio_inputs, params, instance, .. } => {
    // Get or create plugin instance
    let inst = instance.get_or_insert_with(|| {
        self.plugin_registry.load(&plugin_id).expect("Plugin load failed")
    });

    // Prepare input buffers
    let mut input_bufs: Vec<Vec<f32>> = audio_inputs
        .iter()
        .map(|sig| {
            let mut buf = vec![0.0; buffer_size];
            self.eval_buffer_signal(sig, &mut buf);
            buf
        })
        .collect();

    // Apply parameter automation
    for (name, signal) in params {
        let value = self.eval_signal(signal);
        inst.set_parameter_by_name(name, value)?;
    }

    // Process through plugin
    let input_refs: Vec<&[f32]> = input_bufs.iter().map(|b| b.as_slice()).collect();
    let mut output_refs: Vec<&mut [f32]> = vec![output];

    inst.process(&input_refs, &mut output_refs, buffer_size)?;
}
```

#### 3c: Parameter Automation & Mapping
**Depends on: Phase 2, 1a**

```rust
// src/plugin_host/automation.rs

/// Maps parameter names to indices with caching
pub struct ParameterMapper {
    name_to_index: HashMap<String, usize>,
    short_name_to_index: HashMap<String, usize>,
}

impl ParameterMapper {
    pub fn from_plugin_info(info: &PluginInfo) -> Self;

    /// Resolve parameter name (exact, short, or fuzzy)
    pub fn resolve(&self, name: &str) -> Option<usize>;
}

/// Sample-accurate parameter automation
pub struct ParameterAutomation {
    values: Vec<(usize, f32)>,  // (sample_offset, value)
}

impl ParameterAutomation {
    /// Build automation from pattern over buffer
    pub fn from_pattern(
        pattern: &Pattern<f64>,
        buffer_size: usize,
        sample_rate: f32,
        cycle_position: f64,
    ) -> Self;
}
```

### Phase 4: Preset & State System
**Depends on: Phase 3**

```rust
// src/plugin_host/preset.rs

/// Phonon preset file format (.ph)
/// Human-readable, version-controllable
#[derive(Serialize, Deserialize)]
pub struct PhononPreset {
    pub plugin_name: String,
    pub plugin_version: String,
    pub parameters: HashMap<String, f64>,
    pub binary_state: Option<String>,  // Base64-encoded opaque state
}

impl PhononPreset {
    /// Load from .ph file
    pub fn load(path: &Path) -> Result<Self, Error>;

    /// Save to .ph file
    pub fn save(&self, path: &Path) -> Result<(), Error>;

    /// Import from FXP/FXB (VST preset format)
    pub fn from_fxp(data: &[u8]) -> Result<Self, Error>;

    /// Export to FXP
    pub fn to_fxp(&self) -> Result<Vec<u8>, Error>;
}
```

**Preset file format:**
```phonon
-- presets/virus_bass.ph
vst_preset "Osirus" {
    -- Human-readable parameters
    cutoff: 0.35
    resonance: 0.72
    filter_env_amount: 0.5
    osc1_shape: 0.0
    osc2_detune: 0.12

    -- Plugin's opaque state (for params we can't map)
    _binary: "base64encodedstate..."
}
```

### Phase 5: User Experience (Parallel, Optional)

#### 5a: GUI Hosting
**Depends on: Phase 4**

```rust
// src/plugin_host/gui.rs

pub struct PluginGuiHost {
    window: Option<WindowHandle>,
    plugin_instance: Arc<Mutex<PluginInstanceHandle>>,
}

impl PluginGuiHost {
    /// Open plugin GUI in new window
    pub fn open(&mut self) -> Result<(), Error>;

    /// Close GUI window
    pub fn close(&mut self);

    /// Check if GUI is open
    pub fn is_open(&self) -> bool;

    /// Get current state after user editing
    pub fn capture_state(&self) -> Result<PresetState, Error>;
}
```

#### 5b: Plugin Discovery CLI
**Depends on: Phase 1b**

```bash
# Scan and list plugins
phonon plugins scan
phonon plugins list
phonon plugins search "virus"
phonon plugins info "Osirus"

# Show parameter list
phonon plugins params "Osirus"
```

#### 5c: Tab Completion & REPL
**Depends on: Phase 1b, 5b**

```
phonon> vst "Os<TAB>
Osirus    OsTIrus

phonon> ~virus $ vst "Osirus" # cu<TAB>
cutoff    cutoff_keytrack    cutoff_velocity

phonon> :params ~virus
  0: cutoff (0.0-1.0) = 0.35
  1: resonance (0.0-1.0) = 0.72
  ...

phonon> :gui ~virus
[Opens plugin GUI window]

phonon> :save ~virus presets/my_bass.ph
Saved preset to presets/my_bass.ph
```

## DSL Syntax Specification

### Basic Plugin Loading
```phonon
-- Load instrument plugin
~synth $ vst "PluginName"

-- Load effect plugin (with audio input)
~processed $ ~input # vst "EffectName"

-- Specify format explicitly
~synth $ au "com.access.virus"
~synth $ clap "surge-xt"
```

### Preset Loading
```phonon
-- Factory preset by name
~synth $ vst "Osirus" "Trance Lead 1"

-- External preset file
~synth $ vst "Osirus" @presets/my_bass.ph
~synth $ vst "Osirus" @presets/factory.fxp

-- Inline preset definition
~synth $ vst "Osirus" {
    cutoff: 0.5
    resonance: 0.8
}
```

### Parameter Control
```phonon
-- Static parameter
~synth $ vst "Osirus" # cutoff 0.5

-- Pattern-controlled (Phonon's superpower!)
~lfo $ sine 0.25
~synth $ vst "Osirus" # cutoff (~lfo * 0.5 + 0.5)

-- Multiple parameters
~synth $ vst "Osirus" # cutoff 0.5 # resonance 0.8 # filter_env 0.3

-- Parameter by index (for unnamed params)
~synth $ vst "Osirus" # p0 0.5 # p1 0.8
```

### MIDI/Note Input
```phonon
-- Note pattern to instrument
~synth $ vst "Osirus" # note "c4 e4 g4 c5"

-- With velocity
~synth $ vst "Osirus" # note "c4 e4 g4" # velocity "0.8 0.6 1.0"

-- Chord patterns
~synth $ vst "Osirus" # chord "Cm7 Dm7 G7"
```

## File Structure

```
src/
├── plugin_host/
│   ├── mod.rs           # Module exports
│   ├── types.rs         # PluginId, PluginInfo, etc.
│   ├── registry.rs      # PluginRegistry, Scanner
│   ├── instance.rs      # PluginInstanceHandle
│   ├── automation.rs    # ParameterMapper, Automation
│   ├── preset.rs        # PhononPreset, FXP import/export
│   ├── midi.rs          # MIDI event generation
│   └── gui.rs           # GUI hosting (optional)
├── unified_graph.rs     # Add PluginInstance node
└── compositional_compiler.rs  # Add vst/au/clap compilers
```

## Testing Strategy

### Unit Tests
- Plugin type serialization
- Parameter mapping
- Preset file parsing
- MIDI event generation

### Integration Tests (require plugins)
- Plugin scanning
- Load/unload lifecycle
- Audio processing
- Parameter automation
- State save/restore

### Mock Plugin for CI
```rust
// tests/mock_plugin.rs
// Sine wave generator with controllable freq/amp
// Used for testing without real plugins
```

## Platform Considerations

| Platform | VST3 | AU | CLAP | LV2 |
|----------|------|-----|------|-----|
| Linux    | ✓    | -   | ✓    | ✓   |
| macOS    | ✓    | ✓   | ✓    | ✓   |
| Windows  | ✓    | -   | ✓    | -   |

## Timeline Estimate

| Phase | Tasks | Parallelizable |
|-------|-------|----------------|
| 0     | Dependencies | No |
| 1a-c  | Foundation | Yes (3 parallel) |
| 2     | Core Node | No |
| 3a-c  | Integration | Yes (3 parallel) |
| 4     | Presets | No |
| 5a-c  | UX Polish | Yes (3 parallel) |

## Open Questions

1. **Thread model**: Plugin on audio thread or dedicated thread with message passing?
2. **Multiple instances**: Same plugin loaded multiple times - share state?
3. **Hot reload**: Reload plugin without stopping audio?
4. **Latency compensation**: Report and compensate for plugin latency?
5. **Sandbox**: Run plugins in separate process for stability?

## References

- [Rack - Rust plugin hosting](https://github.com/sinkingsugar/rack)
- [Gearmulator - Hardware synth emulation](https://github.com/dsp56300/gearmulator)
- [VST3 SDK](https://github.com/steinbergmedia/vst3sdk)
- [CLAP - CLever Audio Plugin](https://github.com/free-audio/clap)
