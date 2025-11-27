# SINGLE-EVENT PATTERN BUG
**Date**: 2025-10-18
**Status**: 🔴 CONFIRMED BUG - Single-event patterns with long cycles produce silence

## Bug Isolation Complete

| Pattern | Events | Tempo | Cycle Dur | Result |
|---------|--------|-------|-----------|--------|
| `s "bd"` | 1 | 0.5 | 2.0s | ❌ **SILENT** (BUG!) |
| `s "bd bd"` | 2 | 0.5 | 2.0s | ✅ Peak 0.012 |
| `s "bd"` | 1 | 2.0 | 0.5s | ✅ Peak 0.012 |
| `s "bd*16"` | 16 | 2.0 | 0.5s | ✅ Peak 0.012 |

## Root Cause

**CRITICAL**: Bug occurs ONLY when:
1. Pattern has exactly **1 event per cycle**
2. Cycle duration is **≥ 2 seconds** (tempo ≤ 0.5 cps)

This is a **boundary condition bug** in pattern query logic!

## Evidence

### Failing Case (Single event, slow tempo)
```
DSL: s "bd", tempo: 0.5
Result: Peak 0.000 ❌ COMPLETELY SILENT
```

### Working Cases
```
DSL: s "bd bd", tempo: 0.5
Result: Peak 0.012 ✅ WORKS (2 events)

DSL: s "bd", tempo: 0.5
Result: Peak 0.012 ✅ WORKS (fast tempo)

Direct graph: pattern = parse_mini_notation("bd")
Result: Peak 0.014 ✅ WORKS (not DSL-related)
```

## Hypothesis

Pattern query window or event detection fails when:
- Query window is large (≥2 seconds)
- Only 1 event exists in that window
- Possible off-by-one error or floating-point precision issue

## Next Steps

1. Examine pattern query logic in unified_graph.rs (Sample node evaluation)
2. Check query window calculations for long cycles
3. Look for boundary conditions in event detection
4. Test fix with minimal reproduction case
