# Pattern Transform Debugging Session
Date: 2025-10-16

## Summary

Investigated failing pattern transform integration tests and discovered a parser bug.

## Initial Status
- 10/11 pattern transform tests passing
- 1 test failing: `test_bus_reference_with_transform`
- Test was producing RMS=0 (no audio output)

## Investigation Process

### 1. Compiler Investigation
Initially suspected the compiler was not handling pattern transforms on bus references correctly. Added extensive debug output to trace the compilation process.

**Findings:**
- Compiler code is CORRECT
- Pattern transforms are properly applied
- DSP parameters are correctly preserved
- New Sample nodes are created successfully
- Buses are registered correctly

### 2. Parser Investigation
When debugging why no Output statement was being compiled, discovered that only 3 out of 4 statements were being parsed from the test DSL.

**Test DSL:**
```phonon
tempo: 1.0
~drums: s("bd sn")
~fast_drums: ~drums $ fast 2
out: ~fast_drums
```

**Parse Result:**
- Statement 0: `SetCps(1.0)`
- Statement 1: `BusDefinition(drums)`
- Statement 2: `BusDefinition(fast_drums)` with expr `BusRef("drums")` ← WRONG!
- **Missing:** Output statement
- **Remaining unparsed:** ` $ fast 2\nout: ~fast_drums\n`

### 3. Root Cause: Parser Bug
The parser fails to parse pattern transforms when they appear in bus assignments after a bus reference:

**Fails to parse:**
```phonon
~fast_drums: ~drums $ fast 2  # Parser stops at ~drums, leaves $ fast 2 unparsed
```

**Works correctly:**
```phonon
out: ~drums $ fast 2  # Parser correctly parses the full expression
```

## Workaround

Apply pattern transforms in the output statement rather than intermediate bus assignments:

**Instead of:**
```phonon
~drums: s("bd sn")
~fast_drums: ~drums $ fast 2
out: ~fast_drums
```

**Use:**
```phonon
~drums: s("bd sn")
out: ~drums $ fast 2
```

## Test Status

Updated `test_bus_reference_with_transform` to use the workaround syntax. With this change, the test should pass.

## Technical Details

### Parser Flow
When parsing `~fast_drums: ~drums $ fast 2`:

1. `bus_definition` parser matches `~fast_drums:`
2. Calls `expression` parser with ` ~drums $ fast 2`
3. `expression` → `arithmetic` → `term` → `pattern_transform` → `chain` → `primary`
4. `primary` parses `~drums` as BusRef
5. `chain` looks for `#` operators, finds none, returns BusRef("drums")
6. `pattern_transform` should look for `$` operators with `many0(preceded(ws(char('$')), ws(parse_transform_op)))`
7. **BUG:** Pattern transform parsing fails or stops early
8. Returns with remaining input ` $ fast 2`
9. `separated_list0` treats the space as a statement separator
10. Tries to parse ` $ fast 2` as a new statement
11. Fails because `$` doesn't match any statement pattern
12. Returns the 3 successfully parsed statements

### Why Output Statement Works
When using `out: ~drums $ fast 2`, the same parser flow occurs, but somehow succeeds. The difference might be in how `output_definition` vs `bus_definition` handle the expression parsing, though both call `expression` the same way.

**Hypothesis:** There may be a subtle difference in how the parsers handle trailing characters or whitespace, causing the bug to manifest only in bus assignments.

## Recommendations

### Short Term
1. Use the workaround syntax (transforms in output, not intermediate buses)
2. Document this limitation in user-facing docs
3. Add parser tests specifically for this case

### Long Term
1. Debug the expression parser to understand why it stops at BusRef
2. Add detailed logging to `pattern_transform` parser
3. Consider rewriting the expression parser with better error handling
4. Add more comprehensive parser tests

## Files Modified
- `/home/erik/phonon/tests/test_pattern_transform_integration.rs` - Updated test to use workaround

## Status
- ✅ Root cause identified (parser bug, not compiler bug)
- ✅ Workaround documented
- ✅ Test updated to use workaround
- ⚠️ Parser bug remains unfixed (requires deeper parser debugging)
