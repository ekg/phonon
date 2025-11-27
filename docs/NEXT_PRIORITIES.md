# Next Priorities After Operator Syntax Completion

## High Priority Features (Ready to Implement)

### 1. Pattern DSP Parameters (gain, pan, speed, cut)
**Status:** Infrastructure exists, needs implementation
**Estimated Time:** 2-3 days
**Value:** High - enables per-pattern volume/panning/speed control
**Example:**
```phonon
out: s("bd sn") $ gain "0.8 1.0" $ pan "-0.5 0.5"
```

### 2. Additional Audio Effects
**Status:** Some implemented (reverb, distortion, chorus, bitcrush), needs testing
**Estimated Time:** 2-3 days
**Value:** High - essential for music production
**Needed:**
- Verify existing effects work correctly
- Add delay (if not working)
- Add compression
- Document all effect parameters

### 3. Fix Parser Bug (Bus Transform Assignment)
**Status:** Root cause identified, workaround documented
**Estimated Time:** 4-6 hours
**Value:** Medium - improves ergonomics
**Issue:** `~fast: ~drums $ fast 2` doesn't parse correctly
**Workaround:** Use `out: ~drums $ fast 2` instead

## Medium Priority

### 4. Comprehensive Test Audit
**Status:** Many tests exist, unknown how many pass
**Estimated Time:** 4-6 hours
**Value:** High - ensures system stability
**Action:** Run all 116+ test files, categorize results

### 5. Documentation Polish
**Status:** Multiple docs exist, need consolidation
**Estimated Time:** 3-4 hours
**Value:** High - enables user onboarding
**Needed:**
- Consolidate grammar docs
- Write beginner tutorial
- Document all syntax features
- Create example library

### 6. Live Coding Improvements
**Status:** Basic live coding works
**Value:** Medium
**Ideas:**
- Better error messages
- File watch with debouncing
- Multiple file support
- REPL mode

## Low Priority / Nice to Have

### 7. Pattern Transform Enhancements
- More transforms (jux, chunk, stripe, etc.)
- Nested higher-order transforms
- Pattern algebra operators

### 8. MIDI Integration
- MIDI output for patterns
- MIDI input for live control

### 9. Performance Optimization
- Profile hot paths
- Optimize sample loading
- Reduce memory allocations

### 10. Advanced Features
- Custom synth definitions
- Plugin system
- Save/load projects
- Export stems

## Known Issues to Track

1. **Parser Bug:** Bus assignments with transforms (`~x: ~y $ f`)
2. **test_effects_comprehensive:** Uses old API, needs update or ignore
3. **Operator Semantics:** `$` is reversed from Tidal (document clearly)
4. **DSL Syntax:** Keywords require colons (`tempo: 0.5` not `tempo 2.0`)

## Success Criteria for "1.0 Release"

- [ ] All core features work reliably
- [ ] Comprehensive test coverage (>90%)
- [ ] User documentation complete
- [ ] Example library (10+ examples)
- [ ] Grammar formally documented
- [ ] No critical bugs
- [ ] Performance acceptable (realtime audio)
