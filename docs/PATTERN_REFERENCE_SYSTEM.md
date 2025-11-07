# Pattern Reference System & Auto-Completion Strategy

## Overview

Phonon has multiple ways to reference sound sources in patterns, and the auto-completion system must understand and support all of them intelligently.

## Current Pattern Reference Types

### 1. Sample Names (Filesystem-Based)

```phonon
s "bd sn hh cp"
```

**Source**: Directories in `~/dirt-samples/`
- Each subdirectory is a sample bank
- Examples: `bd/`, `sn/`, `hh/`, `cp/`, `bass/`, `808/`, etc.

**Discovery**: Scan filesystem at editor startup
```rust
~/dirt-samples/
  bd/
    BD0000.wav
    BD0001.wav
  bass/
    BASS0.wav
    BASS1.wav
  808/
    808BD.wav
    808SD.wav
```

Sample names = directory names: `bd`, `bass`, `808`

### 2. Bus References (Code-Defined)

```phonon
~bass: saw 55 # lpf 800 0.8
~drums: s "bd sn hh cp"
~lfo: sine 0.25

-- Reference buses in patterns
s "~bass ~drums"
out: sine "~lfo"
```

**Source**: Current editor session
- Any line matching `~identifier:`
- Pattern can include `~name` to reference the bus signal

**Discovery**: Parse current editor content in real-time
- Extract all `~name:` definitions
- Update on every content change (fast enough - just regex scan)

### 3. Direct Pattern Control

```phonon
out: sine "440 550 660"  -- Pattern controls frequency
out: saw "55 82.5 110"   -- Pattern modulates synth parameter
```

**Source**: Pattern string contains values directly
- Not auto-completable (user enters numbers)
- No completion needed here

## Auto-Completion Strategy: Context-Aware with Labels

### User Confirmation

> "I think for... we should show them mixed together. All completions, and say what they are. Context-aware is excellent. Of course, it should always be context-aware."

**Strategy**: Hybrid approach
- **Filter** by context (prefix-based: `b` vs `~b`)
- **Label** results to show type (sample/bus/function)
- **Sort** by relevance within each context

### Visual Design

#### Context: Function Names (outside strings)

```phonon
User types: fa<TAB>

┌─ Completions ─────────────┐
│ fast        [function]    │
│ fade        [function]    │
│ fadeIn      [function]    │
└───────────────────────────┘
```

#### Context: Sample Names (inside `s "..."`)

```phonon
User types: s "b<TAB>

┌─ Completions ─────────────┐
│ bd          [sample]      │
│ bass        [sample]      │
│ bend        [sample]      │
│ ~bass       [bus]         │
└───────────────────────────┘
```

**Note**: Shows both samples AND buses, but buses are prefixed with `~` so user can see the distinction.

#### Context: Bus References (inside `s "~..."`)

```phonon
User types: s "~d<TAB>

┌─ Completions ─────────────┐
│ ~drums      [bus]         │
│ ~delay      [bus]         │
└───────────────────────────┘
```

**Note**: Only shows buses since user explicitly typed `~`

### Implementation Details

```rust
#[derive(Debug, PartialEq)]
pub enum CompletionType {
    Function,
    Sample,
    Bus,
}

#[derive(Debug)]
pub struct Completion {
    pub text: String,
    pub completion_type: CompletionType,
}

impl Completion {
    fn label(&self) -> &str {
        match self.completion_type {
            CompletionType::Function => "[function]",
            CompletionType::Sample => "[sample]",
            CompletionType::Bus => "[bus]",
        }
    }
}

pub fn get_completions_with_context(
    line: &str,
    cursor: usize,
    sample_names: &[String],
    bus_names: &[String],
) -> Vec<Completion> {
    let context = get_completion_context(line, cursor);
    let token = get_token_at_cursor(line, cursor);
    let partial = token.map(|t| t.text).unwrap_or_default();

    match context {
        CompletionContext::Function => {
            // Only show functions
            FUNCTIONS.iter()
                .filter(|f| f.starts_with(&partial))
                .map(|f| Completion {
                    text: f.to_string(),
                    completion_type: CompletionType::Function,
                })
                .collect()
        }

        CompletionContext::Sample => {
            let mut completions = Vec::new();

            // If partial starts with ~, only show buses
            if partial.starts_with('~') {
                let partial_no_tilde = partial.trim_start_matches('~');
                for bus in bus_names {
                    if bus.starts_with(partial_no_tilde) {
                        completions.push(Completion {
                            text: format!("~{}", bus),
                            completion_type: CompletionType::Bus,
                        });
                    }
                }
            } else {
                // Show both samples and buses (with ~ prefix)

                // Add matching samples
                for sample in sample_names {
                    if sample.starts_with(&partial) {
                        completions.push(Completion {
                            text: sample.clone(),
                            completion_type: CompletionType::Sample,
                        });
                    }
                }

                // Add matching buses with ~ prefix
                for bus in bus_names {
                    if bus.starts_with(&partial) {
                        completions.push(Completion {
                            text: format!("~{}", bus),
                            completion_type: CompletionType::Bus,
                        });
                    }
                }
            }

            completions
        }

        CompletionContext::Bus => {
            // User explicitly typed ~, only show buses
            let partial_no_tilde = partial.trim_start_matches('~');
            bus_names.iter()
                .filter(|b| b.starts_with(partial_no_tilde))
                .map(|b| Completion {
                    text: format!("~{}", b),
                    completion_type: CompletionType::Bus,
                })
                .collect()
        }

        CompletionContext::None => Vec::new(),
    }
}
```

## Sorting Strategy

Within each context, sort completions by:

1. **Exact prefix match first**
2. **Alphabetical within type**
3. **Type order**: Samples, then Buses

Example for `s "b<TAB>`:
```
bd          [sample]    <- Exact prefix, sample
bass        [sample]    <- Exact prefix, sample
bend        [sample]    <- Exact prefix, sample
~bass       [bus]       <- Exact prefix, bus (after samples)
```

## Bus Name Extraction

### Real-Time Updates

```rust
impl ModalEditor {
    /// Called after any content change
    fn update_bus_names(&mut self) {
        self.bus_names = extract_bus_names(&self.content);
    }

    /// Extract bus definitions from content
    fn extract_bus_names(content: &str) -> Vec<String> {
        let mut buses = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(name) = parse_bus_definition(trimmed) {
                buses.push(name);
            }
        }

        buses.sort();
        buses.dedup();
        buses
    }

    fn parse_bus_definition(line: &str) -> Option<String> {
        // Match: ~name:
        if line.starts_with('~') {
            if let Some(colon_pos) = line.find(':') {
                let name = &line[1..colon_pos];
                if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    return Some(name.to_string());
                }
            }
        }
        None
    }
}
```

### Performance Considerations

- Bus extraction is O(n) where n = number of lines
- Typically fast: 100 lines × 1µs = 100µs
- Only runs on content changes, not on cursor movement
- Cached until next edit

## Sample Discovery

### One-Time Scan at Startup

```rust
impl ModalEditor {
    pub fn new() -> Self {
        let sample_names = discover_samples();

        Self {
            sample_names,
            bus_names: Vec::new(),
            // ... other fields
        }
    }
}

fn discover_samples() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let dirt_samples = PathBuf::from(home).join("dirt-samples");

    if !dirt_samples.exists() {
        eprintln!("Warning: ~/dirt-samples not found");
        return Vec::new();
    }

    let mut samples = Vec::new();

    if let Ok(entries) = fs::read_dir(&dirt_samples) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    samples.push(name.to_string());
                }
            }
        }
    }

    samples.sort();
    samples
}
```

### Performance Considerations

- One-time cost at startup: ~10-50ms for typical dirt-samples
- Could be lazy-loaded on first Tab press
- Could be cached to disk for instant startup

### Future: Sample Bank Numbers

```phonon
s "bd:0 bd:1 bd:2"  -- Select specific sample from bank
```

**Not implementing yet**, but consider in design:
- Would need to scan files within each directory
- Format: `bd:0` through `bd:N` where N = number of .wav files
- More expensive discovery (need to count files, not just dirs)

## Edge Cases & Validation

### Empty Sample Directory

```rust
if sample_names.is_empty() {
    // Show warning in editor status line
    status = "⚠️  No samples found in ~/dirt-samples";
}
```

### Undefined Bus Reference

```phonon
s "~drums"  -- But no ~drums: definition exists
```

**Behavior**:
- Auto-completion won't suggest it (since bus_names is empty)
- Runtime: Will be silent (no audio output)
- Could add linting: highlight undefined `~` references in red

### Circular Bus References

```phonon
~a: ~b
~b: ~a
```

**Out of scope** for auto-completion
- Handled by compiler/runtime
- Won't prevent completion from working

### Case Sensitivity

```phonon
s "BD vs bd"
```

**Decision**: Case-sensitive matching (Unix filesystem convention)
- `BD` and `bd` are different samples
- Completion respects case exactly

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_completion_context_sample_with_tilde() {
    let line = "s \"~d";
    let completions = get_completions_with_context(
        line, 5,
        &["drums".to_string()],
        &["drums".to_string(), "delay".to_string()],
    );

    // Should only show buses when ~ prefix
    assert_eq!(completions.len(), 2);
    assert!(completions.iter().all(|c| c.text.starts_with('~')));
}

#[test]
fn test_completion_mixed_samples_and_buses() {
    let line = "s \"b";
    let completions = get_completions_with_context(
        line, 4,
        &["bd".to_string(), "bass".to_string()],
        &["bass".to_string()],
    );

    // Should show: bd [sample], bass [sample], ~bass [bus]
    assert_eq!(completions.len(), 3);

    let bd = completions.iter().find(|c| c.text == "bd").unwrap();
    assert_eq!(bd.completion_type, CompletionType::Sample);

    let bass_sample = completions.iter()
        .find(|c| c.text == "bass" && c.completion_type == CompletionType::Sample)
        .unwrap();

    let bass_bus = completions.iter()
        .find(|c| c.text == "~bass")
        .unwrap();
    assert_eq!(bass_bus.completion_type, CompletionType::Bus);
}
```

### Integration Tests

```rust
#[test]
fn test_bus_names_update_on_content_change() {
    let mut editor = ModalEditor::new_for_testing();

    editor.set_content("~drums: s \"bd\"\n~bass: saw 55");
    editor.update_bus_names();

    assert_eq!(editor.bus_names, vec!["bass", "drums"]);

    // Add new bus
    editor.set_content("~drums: s \"bd\"\n~bass: saw 55\n~lfo: sine 0.25");
    editor.update_bus_names();

    assert_eq!(editor.bus_names, vec!["bass", "drums", "lfo"]);
}
```

## Visual Rendering

### Completion Popup

```
Current line: s "b|
                  ^ cursor

┌─ 4 matches ──────────────┐
│ > bd          [sample]   │  <- Selected (arrow)
│   bass        [sample]   │
│   bend        [sample]   │
│   ~bass       [bus]      │
└──────────────────────────┘

Keys:
- Tab: Show/update completions
- ↑/↓: Navigate
- Enter: Accept
- Esc: Dismiss
```

### Color Coding (Optional Enhancement)

```
│ > bd          [sample]   │  <- Blue label
│   ~bass       [bus]      │  <- Magenta label (matches bus highlighting)
```

## Performance Targets

### Completion Generation
- **Target**: < 1ms for typical case
- **Worst case**: < 10ms with 1000 samples + 100 buses
- **Implementation**: Simple linear scan (fast enough)

### Sample Discovery
- **Target**: < 100ms at startup
- **Typical**: ~10-20ms for standard dirt-samples (~200 directories)
- **Optimization**: Could cache to disk if needed

### Bus Extraction
- **Target**: < 1ms per edit
- **Typical**: ~100µs for 100 lines
- **Optimization**: Only scan changed regions if needed (premature)

## Future Enhancements

### 1. Fuzzy Matching
```
s "bss<TAB> → Suggests "bass" even though 'a' is missing
```

### 2. Frequency-Based Ranking
```
Track which samples are used most often, rank them higher
```

### 3. Sample Preview
```
Hovering over completion plays a short preview of the sample
```

### 4. Sample Bank Variants
```
s "bd:<TAB> → Shows bd:0, bd:1, bd:2, ...
```

### 5. Function Argument Hints
```
lpf <TAB> → Shows "lpf input cutoff q" parameter hints
```

## Conclusion

**The system is well-designed and ready for implementation:**

✅ **Clear separation**: Samples (filesystem) vs Buses (code) vs Functions
✅ **Context-aware**: Filters based on cursor position and prefix
✅ **Labeled output**: User sees what type each completion is
✅ **Real-time updates**: Bus names update as user types
✅ **Performant**: All operations fast enough for interactive use
✅ **Testable**: Pure functions with comprehensive test coverage

**No architectural changes needed** - the current design handles all cases correctly. Implementation can proceed directly to the strategy outlined in `MODAL_EDITOR_TESTING_STRATEGY.md`.
