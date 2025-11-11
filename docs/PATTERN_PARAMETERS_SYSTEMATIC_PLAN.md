# Systematic Pattern Parameters Implementation

**CRITICAL ARCHITECTURAL RULE**: Every parameter, everywhere, must be a Pattern by default.

## The Problem

We're currently fixing transforms piecemeal (fast, slow, late, early). This is:
- ❌ Fragile - easy to miss transforms
- ❌ Incomplete - ~30+ transforms remain
- ❌ Not enforced - new code could break the pattern

## The Solution: Systematic Batch Updates

### Phase 1: Time Manipulation (Priority 1)
**Status**: In progress
- [x] fast, slow - DONE
- [x] late, early - DONE (fixing call sites)
- [ ] offset - TODO (wraps late)
- [ ] zoom, compress, compress_to, trim
- [ ] focus

### Phase 2: Probability & Randomness (Priority 2)
- [ ] degrade_by(probability: f64)
- [ ] humanize(time_var, velocity_var)

### Phase 3: Groove & Timing (Priority 3)
- [ ] swing(amount: f64) - **USER REPORTED AS HIGH PRIORITY**
- [ ] shuffle(amount: f64)
- [ ] legato(factor: f64)
- [ ] staccato(factor: f64)

### Phase 4: Value Manipulation (Priority 4)
- [ ] range(min, max)
- [ ] quantize(steps)
- [ ] smooth(amount)
- [ ] exp(base), log(base)
- [ ] walk(step_size)

### Phase 5: DSP Parameters (Priority 5)
These set control values:
- [ ] gain(amount)
- [ ] pan(position)
- [ ] speed(rate)
- [ ] accelerate(rate)
- [ ] cutoff(freq)
- [ ] resonance(amount)
- [ ] delay(amount)
- [ ] room(amount)
- [ ] distort(amount)

### Phase 6: Advanced (Priority 6)
- [ ] jux_by_ctx(amount, transform)
- [ ] echo(times, time, feedback) - only time/feedback need patterns
- [ ] splice(at, other) - 'at' needs pattern

## Implementation Strategy

### For Each Transform:

1. **Update method signature**:
   ```rust
   // OLD:
   pub fn swing(self, amount: f64) -> Self

   // NEW:
   pub fn swing(self, amount: Pattern<f64>) -> Self
   where T: Clone + Send + Sync + 'static
   ```

2. **Query pattern at appropriate time**:
   ```rust
   Pattern::new(move |state| {
       let cycle_start = state.span.begin.to_float().floor();
       let amount_state = State {
           span: TimeSpan::new(
               Fraction::from_float(cycle_start),
               Fraction::from_float(cycle_start + 0.001),
           ),
           controls: state.controls.clone(),
       };

       let amount_haps = amount.query(&amount_state);
       let value = amount_haps.first().map(|h| h.value).unwrap_or(default);

       // ... apply transform with value
   })
   ```

3. **Fix call sites** (automated with grep/sed):
   ```bash
   # Find all call sites
   grep -r "\.swing(" src/

   # Replace with Pattern::pure wrapper
   sed -i 's/\.swing(\([0-9.]*\))/\.swing(Pattern::pure(\1))/g' src/*.rs
   ```

4. **Update compiler** (if needed):
   ```rust
   Transform::Swing(amount_expr) => {
       let amount_pattern = match amount_expr.as_ref() {
           Expr::String(s) => {
               let string_pattern = parse_mini_notation(s);
               string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(0.5))
           }
           _ => {
               let amount = extract_number(&amount_expr)?;
               Pattern::pure(amount)
           }
       };
       Ok(pattern.swing(amount_pattern))
   }
   ```

## Enforcement Going Forward

### 1. Documentation (CLAUDE.md)
Add to development principles:
```markdown
**CRITICAL RULE**: ALL parameters must be Pattern<T>, not bare types.

❌ WRONG:
pub fn foo(self, amount: f64) -> Self

✅ CORRECT:
pub fn foo(self, amount: Pattern<f64>) -> Self
```

### 2. Code Review Checklist
When adding new transforms/UGens:
- [ ] All numeric parameters are `Pattern<T>`
- [ ] Compiler wraps constants with `Pattern::pure()`
- [ ] Tests verify both constant and pattern inputs

### 3. UGen Template (docs/UGEN_IMPLEMENTATION_GUIDE.md)
Update template to enforce pattern parameters:
```rust
SignalNode::NewUGen {
    input: Signal,
    param1: Signal,  // Signal can hold patterns!
    param2: Signal,  // All parameters as Signal
    state: State,
}
```

## Benefits of This Approach

1. **Systematic**: Every transform handled uniformly
2. **Enforceable**: Type system prevents bare f64
3. **Future-proof**: New code follows the pattern
4. **Powerful**: Users can modulate ANY parameter

## Current Status

- ✅ fast, slow (Phase 1) - DONE
- ✅ Effects (lpf, delay, etc.) - Already work via Signal
- ⚠️ late, early (Phase 1) - Method done, call sites remain
- ❌ ~30 transforms (Phases 2-6) - TODO

## Next Steps

1. Finish late/early call sites (mechanical)
2. Batch process Phase 2-6 transforms
3. Update documentation with enforcement rules
4. Verify with comprehensive tests
