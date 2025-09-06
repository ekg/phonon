# Bug Report: Failing Tests in DSP Audio Module

## Date: 2025-09-05

## Current Status (Updated)

### Progress: 9 of 15 tests passing

#### Fixed Issues:
1. ✅ Added missing tokens: impulse, pink, brown, clip  
2. ✅ Fixed peek_token vs current_token bug in parameter parsing
3. ✅ Added Env and Clip node parsing

#### Remaining Issues:
1. ❌ Arithmetic operations (+, -, *, /) not implemented for signal mixing
2. ❌ Parentheses in parameters not supported (e.g., `sin (440 + ~mod)`)
3. ❌ Bus references as delay parameters not working

## Summary
13 out of 15 tests in `tests/dsp_audio_tests.rs` were initially failing. After fixes, 9 tests now pass.

## Root Cause Analysis

### Test: `test_low_pass_filter`
**Code:** `"out: saw 110 >> lpf 1000 0.8"`
**Issue:** Parser fails on "out:" pattern

### Investigation Results:
1. The parser's `parse_line` function expects identifiers to be tokenized as `Token::Symbol(String)`
2. The tokenizer correctly identifies "out" as a symbol
3. However, when parsing chains with specific DSP nodes like `saw`, the parser has special handling for when these appear as the first token without a name (lines 235-241)
4. The issue is that when "out:" is followed by a DSP node like "saw", the parser flow gets confused

### Pattern in Failures:
All failing tests share these characteristics:
- Use "out:" as the output identifier 
- Contain DSP-specific tokens (saw, sin, noise, etc.)
- Have multi-line definitions with bus references using `~`

## Specific Test Failures:

### 1. test_low_pass_filter
- Input: `"out: saw 110 >> lpf 1000 0.8"`
- Expected: Should parse as output chain with saw oscillator and low-pass filter
- Actual: Parser error "Expected identifier"

### 2. test_high_pass_filter  
- Input: `"out: noise >> hpf 2000 0.9"`
- Similar parsing issue with noise generator

### 3. test_additive_synthesis
- Multi-line input with bus references
- Uses arithmetic operators (+) that may not be properly supported

### 4. test_lfo_modulation
- Uses bus references as parameters to DSP nodes
- Parser may not support `~lfo` references as filter parameters

### 5. test_envelope
- Input: `"out: sin 440 >> env 0.01 0.1 0.7 0.2"`
- Envelope node parsing with 4 parameters

### 6. test_delay_effect
- Multi-line with bus references
- Arithmetic operations between buses

### 7. test_reverb_effect
- Input: `"out: impulse 1 >> reverb 0.9 0.5"`
- "impulse" is not a recognized token

### 8. test_complex_patch
- Complex multi-line with multiple buses
- Arithmetic operations (* and +) between signals

### 9. test_fm_synthesis
- Uses arithmetic in frequency parameter: `sin (440 + ~mod)`
- Parser doesn't support parentheses in parameters

### 10. test_ring_modulation
- Uses multiplication operator between signals

### 11. test_noise_generators
- References to "pink" and "brown" noise types not in token list

### 12. test_distortion
- Uses "clip" node which is not in token list

### 13. test_chorus_effect
- Uses bus reference as delay time parameter

## Root Issues Identified:

1. **Missing tokens**: impulse, pink, brown, clip
2. **Arithmetic operations**: Parser doesn't support +, -, *, / between signals
3. **Parentheses in parameters**: Not supported (e.g., `sin (440 + ~mod)`)
4. **Bus references as parameters**: Not fully supported
5. **Multi-line parsing**: Issues with complex multi-line definitions

## Fix Strategy:

1. Add missing tokens to the lexer
2. Implement arithmetic operation support in the parser
3. Add support for parentheses in parameter expressions
4. Enhance bus reference handling in parameters
5. Fix multi-line parsing logic

## Priority:
High - These are core DSP functionality tests that should pass for the system to work correctly.