# Pattern Parameters Audit

**Goal**: Make EVERY parameter accept patterns, not just bare numbers.

**Status**: In Progress
**Last updated**: 2025-11-10

## The Problem

Currently, most parameters only accept constants:
```phonon
s "bd" $ fast 2              -- ❌ Only accepts number
s "bd" # lpf 1000 0.8        -- ❌ Only accepts numbers
s "bd" # delay 0.334 0.3     -- ❌ Only accepts numbers
```

Should accept patterns:
```phonon
s "bd" $ fast "2 3 4"        -- ✅ Pattern of speeds
s "bd" # lpf "500 2000" 0.8  -- ✅ Pattern of cutoff frequencies
s "bd" # delay "0.25 0.5" 0.3 -- ✅ Pattern of delay times
```

## Strategy

### For Transforms (e.g., fast, slow, etc.)

**Current**:
```rust
Transform::Fast(speed_expr) => {
    let speed = extract_number(&speed_expr)?;  // ❌ Only gets single number
    Ok(pattern.fast(speed))
}
```

**Should be**:
```rust
Transform::Fast(speed_expr) => {
    match speed_expr.as_ref() {
        Expr::Number(n) => {
            // Constant: simple case
            Ok(pattern.fast(*n))
        }
        Expr::String(s) => {
            // Pattern: parse mini-notation and create pattern-controlled transform
            let speed_pattern = parse_mini_notation(s);
            Ok(pattern.fast_pattern(speed_pattern))
        }
        _ => Err("fast requires number or pattern string")
    }
}
```

### For Effects (e.g., lpf, delay, reverb)

**Current**:
```rust
// Likely compiles to constant Signal::Value(1000.0)
lpf(cutoff, resonance)
```

**Should be**:
```rust
// Compile to Signal::Pattern that queries at sample time
lpf(cutoff_signal, resonance_signal)
where cutoff_signal can be:
  - Signal::Value(1000.0) for constants
  - Signal::Pattern { pattern, last_value } for patterns
```

## Areas to Audit

### 1. Transforms (src/compositional_compiler.rs)

Lines ~4260-4800 in `apply_transform_to_pattern()`:

- [ ] `Fast(speed)` - line ~4260
- [ ] `Slow(speed)` - line ~4265
- [ ] `Squeeze(factor)` - line ~4269
- [ ] `Every { n, transform }` - line ~4273
- [ ] `Stutter(n)` - line ~4279
- [ ] `Shuffle(amount)` - line ~4282
- [ ] `Chop(n)` / `Striate(n)` - line ~4288
- [ ] `Slice { n, indices }` - indices already pattern! ✅
- [ ] `Struct(pattern)` - already pattern! ✅
- [ ] `Scramble(n)` - line ~4313
- [ ] `Swing(amount)` - line ~4317
- [ ] `Legato(factor)` - line ~4321
- [ ] `Staccato(factor)` - line ~4325
- [ ] `Echo { times, time, feedback }` - line ~4329
- [ ] `Segment(n)` - line ~4337
- [ ] `Zoom { begin, end }` - line ~4340
- [ ] `Compress { begin, end }` - line ~4347
- [ ] `Spin(n)` - line ~4354
- [ ] `Gap(n)` - line ~4357
- [ ] `Late(amount)` - line ~4360
- [ ] `Early(amount)` - line ~4363
- [ ] `Dup(n)` - line ~4366
- [ ] `RotL(amount)` / `RotR(amount)` - line ~4369
- [ ] `Iter(n)` / `IterBack(n)` - line ~4378
- [ ] `Ply(n)` - line ~4391
- [ ] `Linger(amount)` - line ~4394
- [ ] `Offset(amount)` - line ~4397
- [ ] `Loop(n)` / `LoopAt(n)` - loopAt already pattern! ✅
- [ ] `Chew(n)` - line ~4451
- [ ] `FastGap(factor)` - line ~4444
- [ ] `Discretise(n)` - line ~4448
- [ ] `CompressGap { begin, end }` - line ~4452
- [ ] `Reset(n)` / `Restart(n)` - line ~4458
- [ ] `Quantize(n)` - line ~4472
- [ ] `Focus { begin, end, transform }` - line ~4475
- [ ] `Smooth(amount)` - line ~4489
- [ ] `Trim { begin, end }` - line ~4492
- [ ] `Exp(factor)` / `Log(factor)` - line ~4499
- [ ] `Walk(steps)` - line ~4510
- [ ] `Inside { n, transform }` - line ~4513
- [ ] `Outside { n, transform }` - line ~4527
- [ ] `Superimpose(transform)` - transform, not number
- [ ] `Chunk { n, transform }` - line ~4554
- [ ] `Sometimes(transform)` / `Often(transform)` / `Rarely(transform)` - transforms
- [ ] `SometimesBy { prob, transform }` - line ~4597
- [ ] `AlmostAlways(transform)` / `AlmostNever(transform)` - transforms
- [ ] `Whenmod { n, m, transform }` - line ~4633
- [ ] `Wait(amount)` - line ~4648
- [ ] `Weave(amount)` - line ~4655
- [ ] `DegradeSeed(seed)` - line ~4668
- [ ] `Accelerate(amount)` - line ~4672
- [ ] `Humanize { amount, prob }` - line ~4675
- [ ] `Within { begin, end, transform }` - line ~4684
- [ ] `Euclid { pulses, steps }` - line ~4703

### 2. DSP Parameters (src/unified_graph.rs)

Sample parameters that should be pattern-controlled:
- [ ] `gain` - currently Signal, check if pattern works
- [ ] `pan` - currently Signal, check if pattern works
- [ ] `speed` - currently Signal, check if pattern works
- [ ] `cut_group` - currently Signal
- [ ] `attack` - currently Signal
- [ ] `release` - currently Signal
- [ ] `begin` / `end` - currently Signal

Effect parameters:
- [ ] `lpf(cutoff, resonance)` - needs pattern support
- [ ] `hpf(cutoff, resonance)` - needs pattern support
- [ ] `bpf(frequency, q)` - needs pattern support
- [ ] `delay(time, feedback)` - needs pattern support
- [ ] `reverb(size, damping)` - needs pattern support
- [ ] `distortion(amount)` - needs pattern support
- [ ] `bitcrush(bits)` - needs pattern support
- [ ] `chorus(rate, depth)` - needs pattern support
- [ ] `compressor(threshold, ratio)` - needs pattern support

### 3. Pattern Operations (src/pattern_ops_extended.rs)

Methods that take `f64` and should take `Pattern<f64>`:
- [ ] `fast(speed: f64)` - line ~50
- [ ] `slow(speed: f64)` - line ~59
- [ ] `fast_gap(factor: f64)` - ?
- [ ] `loop_at(cycles: f64)` - has `loop_at_pattern` ✅
- [ ] `swing(amount: f64)` - ?
- [ ] `legato_with_duration(factor: f64)` - ?
- [ ] `late(amount: f64)` - ?
- [ ] `early(amount: f64)` - ?

## Implementation Phases

### Phase 1: Core Transforms (HIGHEST IMPACT)
Start with most-used transforms from livecode:
1. ✅ `fast` / `slow` - DONE! (very common)
2. `every` (n parameter) - TODO
3. `swing` (common for groove) - TODO
4. `late` / `early` (timing adjustments) - TODO

### Phase 2: Effects Parameters (HIGH IMPACT)
FX parameters used in every pattern:
1. ✅ `lpf` / `hpf` / `bpf` (cutoff frequency) - ALREADY WORKING! Signal::Node handles patterns
2. ✅ `delay` (time, feedback) - ALREADY WORKING! Signal::Node handles patterns
3. ✅ `reverb` (size, damping) - ALREADY WORKING! Signal::Node handles patterns
4. ✅ `gain` (amplitude control) - ALREADY WORKING! Signal::Node handles patterns

**NOTE**: Effect parameters already support patterns via the Signal/Node architecture!

### Phase 3: Structural Transforms (MEDIUM IMPACT)
Transforms affecting pattern structure:
1. `chop` / `striate` (n slices)
2. `scramble` / `shuffle`
3. `compress` / `zoom`
4. `ply` / `dup`

### Phase 4: Everything Else (COMPLETENESS)
All remaining transforms and parameters.

## Pattern-Controlled Transform Pattern

For any transform `foo` that currently takes `f64`:

1. Keep existing method for constant case:
   ```rust
   pub fn foo(self, value: f64) -> Self { ... }
   ```

2. Add pattern-controlled version:
   ```rust
   pub fn foo_pattern(self, value_pattern: Pattern<String>) -> Self {
       Pattern::new(move |state| {
           // Query value pattern to get current value
           let values = value_pattern.query(state);
           let value = if let Some(hap) = values.first() {
               hap.value.parse::<f64>().unwrap_or(default_value)
           } else {
               default_value
           };

           // Apply transform with queried value
           self.clone().foo(value).query(state)
       })
   }
   ```

3. Update compiler to detect string vs number:
   ```rust
   Transform::Foo(expr) => {
       match expr.as_ref() {
           Expr::Number(n) => Ok(pattern.foo(*n)),
           Expr::String(s) => {
               let pat = parse_mini_notation(s);
               Ok(pattern.foo_pattern(pat))
           }
           _ => Err("foo requires number or pattern")
       }
   }
   ```

## Testing Strategy

For each pattern-controlled parameter:
1. Test constant still works: `fast 2`
2. Test simple pattern: `fast "2 3 4"`
3. Test complex pattern: `fast "<2 3 [4 5]>"`
4. Test pattern with alternation: `fast "<2 4>"`
5. Audio verification: onset detection confirms timing changes

## Notes

- This is a MASSIVE undertaking - affects 50+ functions
- Start with high-impact, commonly-used parameters
- Keep backward compatibility (constants still work)
- Pattern queries may be expensive - profile performance
- Some transforms may not make sense with patterns (e.g., structural ones)

## See Also

- `docs/CRITICAL_BUGS.md` - P0.0 issue
- `src/compositional_compiler.rs` - Transform compilation
- `src/pattern_ops_extended.rs` - Pattern operations
- `src/unified_graph.rs` - DSP parameters
