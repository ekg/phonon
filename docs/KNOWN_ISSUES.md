# Known Issues in Phonon

## Transform Chaining Order Sensitivity

**Status**: Known limitation (requires parser refactoring to fix)

**Description**: The order of chained pattern transforms affects behavior. 

**Examples**:
```phonon
# Works correctly:
d1: s "bd sn" $ fast 2 $ rev  # ✅ Applies fast, then rev

# Works incorrectly:
d1: s "bd sn" $ rev $ fast 2  # ❌ Only applies rev

# Workaround: Always put transforms in left-to-right order
d1: s "bd sn" $ fast 2 $ rev  # Instead of rev $ fast 2
```

**Root Cause**: The parser creates different AST structures:
- `fast 2 $ rev` → nested `Expr::Transform` nodes (handled correctly by compiler)
- `rev $ fast 2` → `Expr::Transform { transform: Rev, expr: Call("fast") }` (not handled)

**Fix Required**: Parser needs to handle `Transform $ Transform` as a special case and create nested `Expr::Transform` nodes.

**Workaround**: Use transforms in left-to-right order (which matches Tidal Cycles convention anyway).

## Onset Detection Threshold for Short Samples

**Description**: Very short samples (e.g., hi-hats in Euclidean patterns) may not be detected by onset detection algorithms.

**Impact**: Affects automated testing, not user-facing functionality.

**Workaround**: Use lower detection thresholds or rely on RMS/spectral analysis.
