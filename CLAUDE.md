- you must use test-driven design. any feature implemented must be tested. audio features should be tested by examining output audio files using appropriate signal analysis techniques. intermediate representations (text) can be used too for testing that the pattern and signal generation system is working correctly.
- any time you finish and say "next steps" you should actually automatically begin working on the next steps and not bother me lol.

## Current Status (Updated)

### Modular Synthesis DSL Implementation
- ✅ Complete signal graph system with topological sorting
- ✅ Enhanced parser supporting arithmetic operations and bus references
- ✅ Pattern bridge for cross-modulation between patterns and audio
- ✅ Audio analysis module (pitch, transients, spectral centroid, RMS)
- ✅ Modulation routing system with multiple destinations
- ✅ Integration tests covering all features
- ✅ Comprehensive documentation (user guide and developer guide)
- ✅ All compiler warnings cleaned up
- ✅ 69 tests total, 66 passing

### Key Files
- `src/signal_graph.rs` - Core signal routing infrastructure
- `src/enhanced_parser.rs` - DSL parser with full expression support
- `src/pattern_bridge.rs` - Pattern-to-audio cross-modulation
- `src/audio_analysis.rs` - Real-time audio feature extraction
- `src/modulation_router.rs` - Advanced modulation routing
- `docs/modular-synthesis-dsl-design.md` - Original design document
- `docs/modular-synthesis-user-guide.md` - User documentation
- `docs/modular-synthesis-developer-guide.md` - Developer documentation

### DSL Features
The DSL now supports:
- Signal buses with `~` prefix notation
- Arithmetic operations with proper precedence
- Signal chains with `>>` operator
- Pattern strings embedded in synthesis
- Audio analysis (RMS, pitch, transients, centroid)
- Cross-modulation between patterns and audio
- Modulation routing to multiple targets
- Synthdef definitions
- Conditional processing with `when`

### Example DSL Code
```phonon
~lfo: sine(0.5) * 0.5 + 0.5
~bass: saw(55) >> lpf(~lfo * 2000 + 500, 0.8)
~bass_rms: ~bass >> rms(0.05)
~hats: "hh*16" >> hpf(~bass_rms * 5000 + 2000, 0.8)
out: ~bass * 0.4 + ~hats * 0.2
```