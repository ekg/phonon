# Tab Completion System Improvement Plan

## Executive Summary

The current tab completion system has **three major sources of truth** that drift out of sync:
1. `FUNCTIONS` list in `highlighting.rs` (100+ strings)
2. `FUNCTION_METADATA` in `function_metadata.rs` (150+ hand-written entries)
3. `GENERATED_STUBS` in `generated_metadata_stubs.rs` (100+ auto-generated)

Plus the actual implementation in `compositional_compiler.rs`, `pattern_ops.rs`, etc.

This plan proposes consolidating to a **single source of truth** with complete discoverability.

---

## Current Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    CURRENT STATE (FRAGMENTED)                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  highlighting.rs          function_metadata.rs                   │
│  ┌──────────────┐        ┌──────────────────────┐               │
│  │ FUNCTIONS[]  │        │ FUNCTION_METADATA    │               │
│  │ 100+ strings │◄──────►│ 150+ hand-written    │               │
│  │ (just names) │ drift! │ (descriptions,       │               │
│  └──────────────┘        │  params, examples)   │               │
│         ▲                └──────────────────────┘               │
│         │                         ▲                              │
│         │ drift!                  │ drift!                       │
│         │                         │                              │
│         ▼                         ▼                              │
│  ┌──────────────────────────────────────────────┐               │
│  │            ACTUAL IMPLEMENTATIONS             │               │
│  │                                              │               │
│  │  compositional_compiler.rs (transforms)      │               │
│  │  pattern_ops.rs (pattern methods)            │               │
│  │  unified_graph.rs (SignalNode enum)          │               │
│  │  nodes/*.rs (effect implementations)         │               │
│  └──────────────────────────────────────────────┘               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Problems

1. **Drift**: New functions added to compiler but not to completion
2. **Inconsistency**: Parameters/defaults differ between metadata and implementation
3. **Duplication**: Same info maintained in 3+ places
4. **No docstring search**: Can't find functions by description
5. **Basic fuzzy**: Current matching is simple, not fzf-quality

---

## Proposed Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    PROPOSED STATE (UNIFIED)                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌───────────────────────────────────────────────────────┐      │
│  │           SOURCE CODE (Single Source of Truth)         │      │
│  │                                                        │      │
│  │  /// Low-pass filter with resonance                    │      │
│  │  /// @param cutoff Filter cutoff in Hz (default: 1000) │      │
│  │  /// @param q Resonance 0.1-10 (default: 1.0)          │      │
│  │  /// @example saw 110 # lpf 800 0.8                    │      │
│  │  /// @category Filters                                 │      │
│  │  pub fn lpf(cutoff: Pattern<f64>, q: Pattern<f64>)     │      │
│  │                                                        │      │
│  └───────────────────────────────────────────────────────┘      │
│                          │                                       │
│                          ▼ (build.rs extracts)                   │
│                                                                  │
│  ┌───────────────────────────────────────────────────────┐      │
│  │         generated_completions.rs (Auto-Generated)      │      │
│  │                                                        │      │
│  │  • All function names                                  │      │
│  │  • All parameters with types & defaults                │      │
│  │  • All descriptions (for docstring search)             │      │
│  │  • All examples                                        │      │
│  │  • Full-text search index                              │      │
│  └───────────────────────────────────────────────────────┘      │
│                          │                                       │
│                          ▼                                       │
│                                                                  │
│  ┌───────────────────────────────────────────────────────┐      │
│  │              Editor Tab Completion                     │      │
│  │                                                        │      │
│  │  • fzf-style fuzzy matching (Smith-Waterman)           │      │
│  │  • Search by name OR description                       │      │
│  │  • Live documentation preview                          │      │
│  │  • Default value insertion                             │      │
│  └───────────────────────────────────────────────────────┘      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Phase 1: Unified Doc Comment Format

Define a standard doc comment format that can be parsed by build.rs:

```rust
/// Short description (first line is always the summary)
///
/// Longer description with details about behavior, use cases, etc.
///
/// # Parameters
/// - `cutoff`: Filter cutoff frequency in Hz (default: 1000.0)
/// - `q`: Resonance/Q factor from 0.1 to 10.0 (default: 1.0)
///
/// # Example
/// ```phonon
/// ~bass $ saw 55 # lpf 800 1.5
/// ```
///
/// # Category
/// Filters
pub fn lpf(self, cutoff: Pattern<f64>, q: Pattern<f64>) -> Self
```

Alternative: Use custom attributes (cleaner parsing):
```rust
#[phonon_function(
    category = "Filters",
    example = "saw 55 # lpf 800 1.5"
)]
/// Low-pass filter - removes frequencies above cutoff
pub fn lpf(
    #[param(default = 1000.0, range = "20-20000", unit = "Hz")]
    cutoff: Pattern<f64>,
    #[param(default = 1.0, range = "0.1-10")]
    q: Pattern<f64>,
) -> Self
```

### Phase 2: Enhanced build.rs

Expand `build.rs` to extract from ALL sources:

```rust
// Sources to parse:
const SOURCES: &[&str] = &[
    "src/pattern.rs",           // Core pattern methods
    "src/pattern_ops.rs",       // Pattern transforms
    "src/pattern_ops_extended.rs",
    "src/pattern_structure.rs",
    "src/compositional_compiler.rs", // Transform::* enum
    "src/unified_graph.rs",     // SignalNode::* enum
    "src/nodes/*.rs",           // Effect nodes
];

// Generate:
// 1. COMPLETIONS: Vec<CompletionEntry> - all functions with full metadata
// 2. SEARCH_INDEX: HashMap<String, Vec<usize>> - word -> completion indices
// 3. CATEGORIES: HashMap<String, Vec<usize>> - category -> completion indices
```

### Phase 3: fzf-Style Fuzzy Matching

Replace current simple fuzzy with proper fzf algorithm:

```rust
/// Smith-Waterman-like fuzzy matching with:
/// - Consecutive match bonus
/// - Word boundary bonus
/// - Camel case boundary bonus
/// - First character bonus
/// - Gap penalty
fn fzf_score(query: &str, candidate: &str) -> Option<(i32, Vec<usize>)> {
    // Returns (score, matched_indices) for highlighting
}
```

Features:
- **Highlight matched characters** in the UI
- **Score consecutive matches** higher (typing "lpf" should rank "lpf" above "loopfilter")
- **Word boundaries** score higher (typing "bp" matches "bandPass" better than "substring")

### Phase 4: Docstring Search

Enable searching through descriptions:

```
User types: "filter"
Shows:
  lpf     Low-pass filter - removes frequencies above cutoff
  hpf     High-pass filter - removes frequencies below cutoff
  bpf     Band-pass filter - isolates frequency band
  ...

User types: "time"
Shows:
  fast    Speed up pattern by factor
  slow    Slow down pattern by factor
  late    Shift pattern forward in time
  early   Shift pattern backward in time
  ...
```

Implementation:
```rust
fn search_completions(query: &str) -> Vec<(Completion, i32)> {
    let mut results = Vec::new();

    for completion in COMPLETIONS.iter() {
        // Score against name
        let name_score = fzf_score(query, &completion.name);

        // Score against description (lower weight)
        let desc_score = fzf_score(query, &completion.description)
            .map(|s| s / 2);

        // Score against example
        let example_score = fzf_score(query, &completion.example)
            .map(|s| s / 3);

        // Take best score
        let best = [name_score, desc_score, example_score]
            .into_iter()
            .flatten()
            .max();

        if let Some(score) = best {
            results.push((completion.clone(), score));
        }
    }

    results.sort_by_key(|(_, score)| -score);
    results
}
```

### Phase 5: Documentation Preview Panel

Add a preview panel that shows full documentation:

```
┌────────────────────────────────────────────────────────────┐
│ ~bass $ saw 55 # lp█                                       │
├────────────────────────────────────────────────────────────┤
│ > lpf     Low-pass filter                      [Filters]   │
│   hpf     High-pass filter                     [Filters]   │
│   loopAt  Loop sample at cycle                [Transforms] │
├────────────────────────────────────────────────────────────┤
│ lpf - Low-pass filter                                      │
│                                                            │
│ Removes frequencies above the cutoff frequency.            │
│ Use Q > 1 for resonance at the cutoff point.              │
│                                                            │
│ Parameters:                                                │
│   cutoff  Hz      Cutoff frequency (default: 1000)        │
│   q       float   Resonance 0.1-10 (default: 1.0)         │
│                                                            │
│ Example:                                                   │
│   ~bass $ saw 55 # lpf 800 1.5                            │
└────────────────────────────────────────────────────────────┘
```

Key bindings:
- **Tab**: Cycle through completions
- **Enter/Tab (on selection)**: Insert completion
- **Ctrl+Space**: Insert with all default parameters
- **?** or **F1**: Toggle documentation panel
- **Ctrl+D**: Search by description

### Phase 6: Default Value Insertion

When accepting a completion, offer to insert with defaults:

```
Typing: lpf<Tab>
  Option 1: "lpf" (just the function name)
  Option 2: "lpf 1000 1.0" (with default positional args)
  Option 3: "lpf :cutoff 1000 :q 1.0" (with named args)

Ctrl+Space on "lpf":
  Inserts: "lpf :cutoff 1000 :q 1.0"
```

---

## File Changes Summary

| File | Change |
|------|--------|
| `build.rs` | Expand to parse all source files, generate unified completion data |
| `src/modal_editor/completion/mod.rs` | Simplify to use generated data only |
| `src/modal_editor/completion/generated_completions.rs` | New: auto-generated from build.rs |
| `src/modal_editor/completion/function_metadata.rs` | DELETE: replaced by generated |
| `src/modal_editor/completion/generated_metadata_stubs.rs` | DELETE: replaced by generated |
| `src/modal_editor/completion/matching.rs` | Replace with fzf algorithm |
| `src/modal_editor/highlighting.rs` | FUNCTIONS list auto-generated |
| `src/modal_editor/mod.rs` | Add documentation preview panel |
| Source files (pattern_ops.rs, etc.) | Add standardized doc comments |

---

## Migration Path

1. **Phase 1** (2-3 hours): Define doc comment format, update a few key functions
2. **Phase 2** (4-6 hours): Enhance build.rs to extract from all sources
3. **Phase 3** (2-3 hours): Implement fzf-style matching
4. **Phase 4** (1-2 hours): Add docstring search
5. **Phase 5** (3-4 hours): Add documentation preview panel
6. **Phase 6** (2-3 hours): Implement default value insertion

Total: ~15-20 hours of focused work

---

## Test Plan

1. **Regression tests**: Existing completion tests should pass (after updating expectations)
2. **Fuzz testing**: Random input shouldn't crash completion
3. **Coverage**: Every function in FUNCTIONS list has metadata
4. **Sync verification**: build.rs test ensures no drift between source and generated

```rust
#[test]
fn test_no_missing_completions() {
    // Every function in FUNCTIONS must have completion metadata
    for func in FUNCTIONS.iter() {
        assert!(
            COMPLETIONS.iter().any(|c| c.name == *func),
            "Missing completion for: {}",
            func
        );
    }
}

#[test]
fn test_no_orphan_completions() {
    // Every completion must map to a real function
    for completion in COMPLETIONS.iter() {
        assert!(
            FUNCTIONS.contains(&completion.name.as_str())
            || is_sample_function(&completion.name),
            "Orphan completion (no implementation): {}",
            completion.name
        );
    }
}
```

---

## Questions for Discussion

1. **Attribute macros vs doc comments?**
   - Attributes: Cleaner parsing, compile-time validation
   - Doc comments: Standard Rust, works with rustdoc
   - Recommendation: Doc comments with structured format

2. **How much documentation is enough?**
   - Every function needs: short description, params with defaults
   - Nice to have: long description, examples, see-also

3. **Real-time vs build-time generation?**
   - Build-time: Faster runtime, checked into git
   - Real-time: Always current, but slower startup
   - Recommendation: Build-time (like now)

4. **Keyboard shortcuts?**
   - Suggest: Tab=cycle, Enter=accept, Ctrl+Space=with defaults, ?=docs

---

## Success Criteria

- [ ] Zero manual metadata maintenance needed for new functions
- [ ] Fuzzy search finds functions by partial name OR description
- [ ] Documentation visible without leaving editor
- [ ] Default values insertable with single keystroke
- [ ] All tests pass
- [ ] No performance regression (completion < 10ms)
