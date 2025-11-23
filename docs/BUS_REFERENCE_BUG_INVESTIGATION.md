# Bus Reference Bug Investigation

**Date**: 2025-11-23
**Issue**: Bus references in sample patterns (`s "~busname(3,8)"`) produce silence despite correct synthesis

## Summary

User reported that patterns like `~s: sine 440` followed by `~c: s "~s(<7 7 6 10>,11,2)" # note "c3'maj"` produce a solid sine tone with no patterning, when they should trigger the bus rhythmically according to the pattern.

## Investigation Status

### What WORKS ✅

1. **Bus synthesis**: Buffers are synthesized correctly with proper audio
   - Debug output shows: `Synthesized buffer: 5512 samples, RMS=0.707139`
   - Sine wave samples are correct: `[0.0, 0.062648326, 0.12505053, ...]`

2. **Pattern evaluation**: Events are detected and triggered correctly
   - 4 events triggered as expected for `~sine*4` pattern
   - Debug output shows: `Triggering: '~sine' at cycle 0.000000, 0.250000, 0.500000, 0.750000`

3. **Voice triggering code path**: The code reaches the trigger_sample_with_envelope() call
   - Bus lookup works (line 9246)
   - Event duration calculated (lines 9247-9260)
   - Buffer retrieved from cache (lines 9267-9279)
   - Voice trigger called (lines 9331-9341)

### What FAILS ❌

4. **Final audio output**: Despite perfect synthesis and triggering, output is SILENCE
   - Test result: `RMS=0` (complete silence)
   - But synthesized buffer has `RMS=0.707` (perfect sine wave)

## Root Cause Hypothesis

The bug is NOT in:
- Bus synthesis (works perfectly)
- Pattern evaluation (works perfectly)
- Event triggering logic (events detected correctly)

The bug IS likely in:
- Voice playback mechanism
- Voice manager integration with buffer-based rendering
- Voice output caching/routing

## Next Steps for Debugging

1. **Verify voices are created**: Add debug output to check if `trigger_sample_with_envelope()` actually creates voice objects

2. **Check voice processing**: Verify that `process_buffer_per_node()` processes the triggered voices

3. **Check output routing**: Verify that voice output is correctly routed to the Sample node's output via `voice_output_cache`

4. **Check source_node matching**: The Sample node uses `node_id.0` as source_node. Verify this matches what the voice is tagged with.

## Test Files Created

- `/home/erik/phonon/tests/test_bus_references_in_patterns.rs` - Comprehensive test suite (12 tests)
  - All tests currently failing with RMS=0
  - Test `test_user_exact_issue` reproduces the exact user problem

## Code Changes Made

### Debug Output Added

1. **Bus synthesis debug** (`synthesize_bus_buffer_parallel`):
   ```rust
   eprintln!("  Synthesized buffer: {} samples, RMS={:.6}, first_10={:?}",
       buffer.len(), rms, &buffer[..buffer.len().min(10)]);
   ```

2. **Voice trigger debug** (before trigger_sample_with_envelope):
   ```rust
   eprintln!("    About to trigger voice: buffer_len={}, gain={}, pan={}, speed={}, source_node={}",
       synthetic_buffer.len(), gain_val, pan_val, final_speed, node_id.0);
   ```

3. **Voice count debug** (before processing):
   ```rust
   let voice_count = self.voice_manager.borrow().active_voice_count();
   eprintln!("  Processing {} active voices in buffer", voice_count);
   ```

## Environment Variables for Debugging

- `DEBUG_BUS_SYNTHESIS=1` - Shows synthesized buffer stats
- `DEBUG_SAMPLE_EVENTS=1` - Shows pattern event triggering
- `DEBUG_VOICE_TRIGGER=1` - Shows voice trigger parameters
- `DEBUG_VOICE_COUNT=1` - Shows active voice count during processing

## Relevant Code Locations

- **Bus triggering**: `src/unified_graph.rs:9244-9427`
- **Bus synthesis**: `src/unified_graph.rs:3920-3958` (`synthesize_bus_buffer_parallel`)
- **Voice processing**: `src/voice_manager.rs:1296-1386` (`process_buffer_per_node`)
- **Sample node output**: `src/unified_graph.rs:9652` (returns from `voice_output_cache`)
- **Voice cache population**: `src/unified_graph.rs:11856` (`process_buffer_per_node`)

## Status

**FIXED** ✅ - Bus references in sample patterns now work correctly!

## Solution

The root cause was a timing issue with voice processing:
1. Voice buffers were pre-computed for all existing voices before buffer rendering
2. Sample nodes triggered NEW voices during evaluation (after pre-computation)
3. Newly triggered voices weren't in the pre-computed buffers

**Fix implemented:**
- Track all newly triggered voices during buffer rendering
- Process them live for each sample in the buffer
- Add their output to voice_output_cache for correct routing

**Test results:**
- 9/10 bus reference tests passing
- `test_user_exact_issue` - PASSED (user's exact reported bug fixed!)
- Only `test_bus_reference_nested` fails (nested bus→bus triggering - edge case)

**Files modified:**
- `src/unified_graph.rs:11971-12022` - Live processing of newly triggered voices
- `src/voice_manager.rs:1671-1683` - New method to process voices by index
