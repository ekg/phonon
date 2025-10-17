# Phonon Modular Synthesis DSL - Developer Guide

## Architecture Overview

The modular synthesis DSL is implemented in Rust as part of the Fermion engine. It provides a graph-based signal processing system with bidirectional communication between the pattern engine (Strudel) and the audio synthesis engine.

## Core Components

### 1. Signal Graph (`signal_graph.rs`)

The heart of the system - manages nodes, connections, and signal routing.

```rust
pub struct SignalGraph {
    pub nodes: HashMap<NodeId, Node>,
    pub buses: HashMap<BusId, f32>,
    pub connections: Vec<Connection>,
    pub execution_order: Option<Vec<NodeId>>,
    pub sample_rate: f32,
}
```

**Key Features:**
- Topological sorting for correct execution order
- Cycle detection to prevent feedback loops
- Bus system for named signal routes
- Connection management with signal taps

### 2. Enhanced Parser (`enhanced_parser.rs`)

Parses the DSL text into a signal graph.

```rust
pub enum Expression {
    Number(f64),
    BusRef(String),
    Add(Box<Expression>, Box<Expression>),
    Chain(Box<Expression>, Box<Expression>),
    FunctionCall(String, Vec<Expression>),
    // ...
}
```

**Parsing Pipeline:**
1. Tokenization (lexical analysis)
2. Expression parsing with precedence
3. Bus definition parsing
4. Graph construction
5. Validation

### 3. Signal Executor (`signal_executor.rs`)

Processes the signal graph to generate audio.

```rust
pub struct SignalExecutor {
    graph: SignalGraph,
    processors: HashMap<NodeId, Box<dyn AudioUnit>>,
    buffers: HashMap<NodeId, AudioBuffer>,
    // ...
}
```

**Processing Steps:**
1. Initialize FunDSP processors
2. Process nodes in topological order
3. Apply connections and modulation
4. Mix to output buffer

### 4. Pattern Bridge (`pattern_bridge.rs`)

Enables cross-modulation between patterns and synthesis.

```rust
pub struct PatternBridge {
    signal_graph: Arc<RwLock<SignalGraph>>,
    pattern_signals: HashMap<String, PatternSignal>,
    audio_features: HashMap<String, f32>,
    // ...
}
```

**Features:**
- Pattern event to signal conversion
- Note to frequency mapping
- Gate and trigger generation
- Audio feature extraction

### 5. Audio Analysis (`audio_analysis.rs`)

Real-time feature extraction for modulation.

```rust
pub struct AudioAnalyzer {
    pitch_detector: PitchDetector,
    transient_detector: TransientDetector,
    spectral_centroid: SpectralCentroid,
    // ...
}
```

**Analysis Types:**
- RMS level tracking
- Pitch detection (autocorrelation)
- Transient detection (energy flux)
- Spectral centroid (brightness)

### 6. Modulation Router (`modulation_router.rs`)

Advanced routing of modulation signals.

```rust
pub struct ModulationRouter {
    routes: Vec<ModulationRoute>,
    signal_graph: Arc<RwLock<SignalGraph>>,
    modulation_cache: HashMap<String, f32>,
    base_values: HashMap<String, f32>,
}
```

**Modulation Modes:**
- Add: Offset the base value
- Multiply: Scale the base value
- Replace: Override the base value
- Bipolar: -1 to +1 modulation

## Implementation Details

### Node Types

```rust
pub enum Node {
    Source { id: NodeId, source_type: SourceType },
    Bus { id: BusId, value: f32 },
    Processor { id: NodeId, processor_type: ProcessorType },
    Analysis { id: NodeId, analysis_type: AnalysisType },
    Pattern { id: NodeId, pattern: String },
    Output { id: NodeId },
}
```

### Source Types

```rust
pub enum SourceType {
    Sine { freq: f64 },
    Saw { freq: f64 },
    Square { freq: f64 },
    Triangle { freq: f64 },
    Noise,
    Sample { name: String },
}
```

### Processor Types

```rust
pub enum ProcessorType {
    LowPass { cutoff: f64, q: f64 },
    HighPass { cutoff: f64, q: f64 },
    BandPass { center: f64, q: f64 },
    Delay { time: f64, feedback: f64 },
    Reverb { mix: f64 },
    Distortion { amount: f64 },
    Compressor { threshold: f64, ratio: f64 },
    Gain { amount: f32 },
}
```

## DSL Grammar (Simplified BNF)

```bnf
program := statement*
statement := bus_definition | route_statement | output_statement

bus_definition := "~" identifier ":" expression
route_statement := "route" expression "->" route_targets
output_statement := "out" ":" expression

expression := term (("+"|"-") term)*
term := factor (("*"|"/") factor)*
factor := primary (">>" primary)*
primary := number | bus_ref | function_call | string | "(" expression ")"

function_call := identifier "(" arguments ")"
arguments := expression ("," expression)*
bus_ref := "~" identifier
```

## Integration Points

### With Strudel (JavaScript)

Communication via OSC messages:
```javascript
// Pattern event sent to Fermion
{
  pattern: "bass",
  event: {
    value: "c3",
    time: 0.0,
    duration: 0.5,
    velocity: 0.8
  }
}
```

### With Audio Engine

FunDSP integration:
```rust
use fundsp::hacker::*;

let sine = sine_hz(440.0);
let filtered = sine # lowpass_hz(1000.0, 1.0);
```

## Adding New Features

### Adding a New Oscillator

1. Add to `SourceType` enum:
```rust
pub enum SourceType {
    // ...
    PulseWave { freq: f64, width: f64 },
}
```

2. Update parser to recognize it:
```rust
"pulse" => {
    let freq = self.parse_number()?;
    let width = self.parse_number()?;
    SourceType::PulseWave { freq, width }
}
```

3. Implement in signal executor:
```rust
SourceType::PulseWave { freq, width } => {
    Box::new(pulse() * constant(width) # sine_hz(freq))
}
```

### Adding a New Effect

1. Add to `ProcessorType` enum
2. Update parser
3. Implement processor creation
4. Add tests

### Adding a New Analysis Type

1. Add to `AnalysisType` enum
2. Implement analyzer in `audio_analysis.rs`
3. Update signal executor
4. Add to pattern bridge

## Testing

### Unit Tests

Each module has comprehensive tests:
```bash
cargo test audio_analysis::tests
cargo test pattern_bridge::tests
cargo test modulation_router::tests
```

### Integration Tests

Test complete signal flow:
```bash
cargo test phonon_integration_tests
```

### Test Pattern Files

Located in `/test-patterns/`:
- `test_modular_synth.phonon` - Basic functionality
- `test_cross_modulation.phonon` - Pattern/audio interaction
- `test_complex_routing.phonon` - Advanced routing

## Performance Optimization

### Current Optimizations

1. **Topological Sorting**: Nodes processed in optimal order
2. **Buffer Reuse**: Audio buffers recycled between frames
3. **Lazy Evaluation**: Only compute used signals
4. **Lock-Free Audio**: RwLock for thread-safe access

### Future Optimizations

1. **SIMD Processing**: Use vectorized operations
2. **Graph Compilation**: Pre-compile static graphs
3. **Parallel Processing**: Multi-threaded node execution
4. **JIT Compilation**: Compile DSL to native code

## Debugging

### Enable Debug Output

```rust
use tracing::{debug, info};

debug!("Processing node: {:?}", node_id);
info!("Graph execution order: {:?}", order);
```

### Visualize Signal Graph

```rust
impl SignalGraph {
    pub fn to_dot(&self) -> String {
        // Generate Graphviz DOT format
    }
}
```

### Monitor Signals

```phonon
// Add visualization helpers
~viz_level: ~master # rms(0.01)
~viz_pitch: ~input # pitch
```

## Common Issues and Solutions

### Issue: Feedback Loop Detected
**Solution**: Use delayed feedback with attenuation:
```phonon
~feedback: ~delay_out * 0.7  // Attenuate feedback
~delay_out: (~input + ~feedback) # delay(0.1)
```

### Issue: No Audio Output
**Debug Steps:**
1. Check graph has output node
2. Verify execution order computed
3. Check bus values updating
4. Monitor processor outputs

### Issue: High CPU Usage
**Optimizations:**
1. Reduce analysis window sizes
2. Increase buffer size
3. Simplify signal chains
4. Use fewer parallel paths

## API Reference

### SignalGraph Methods

```rust
impl SignalGraph {
    pub fn new(sample_rate: f32) -> Self
    pub fn add_node(&mut self, node: Node) -> NodeId
    pub fn add_bus(&mut self, name: String, value: f32) -> BusId
    pub fn connect(&mut self, from: NodeId, to: NodeId, amount: f32)
    pub fn compute_execution_order(&mut self) -> Result<(), String>
}
```

### PatternBridge Methods

```rust
impl PatternBridge {
    pub fn register_pattern(&mut self, name: String) -> BusId
    pub fn process_pattern_event(&mut self, pattern: &str, event: PatternEvent)
    pub fn update_audio_features(&mut self, executor: &SignalExecutor)
    pub fn apply_pattern_modulation(&self, pattern: &str, base: f32, amount: f32) -> f32
}
```

### ModulationRouter Methods

```rust
impl ModulationRouter {
    pub fn add_route(&mut self, route: ModulationRoute)
    pub fn process(&mut self)
    pub fn parse_route(&mut self, route_str: &str) -> Result<ModulationRoute, String>
}
```

## Future Roadmap

### Phase 1: Core Stability ✅
- Signal graph implementation
- Basic DSL parser
- Pattern integration
- Audio analysis

### Phase 2: Advanced Features (Current)
- Modulation routing
- Parallel processing
- Hot-swapping support
- Complex synthdefs

### Phase 3: Performance
- SIMD optimization
- Graph compilation
- Multi-threading
- Memory pooling

### Phase 4: Extended Features
- Visual programming bridge
- MIDI integration
- Plugin hosting
- Network collaboration

## Contributing

### Code Style
- Use `rustfmt` for formatting
- Follow Rust naming conventions
- Add doc comments for public APIs
- Write tests for new features

### Pull Request Process
1. Create feature branch
2. Implement with tests
3. Update documentation
4. Run `cargo test`
5. Submit PR with description

## License

Part of the Phonon project - see main LICENSE file.