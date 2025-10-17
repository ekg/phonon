# Phonon Grammar & Operator Reference

**Status**: This is what's **actually implemented** vs. what's documented

---

## History & Influences

### Operator Origins

- `#` - **From Glicol** (Rust live coding language, signal chaining)
- `$` - **Documented but NOT implemented** (F#/Elixir pipe operator)
- `$` - **From Tidal/Haskell** (function application, NOT implemented)
- `#` - **From Tidal** (parameter application, NOT implemented)
- `~` - **From Tidal** (bus/reference prefix)
- `:` vs `=` - **Confusion** (docs say `:`, code uses `=`)

---

## Current Grammar (ACTUAL - What Works)

### What Actually Parses

```ebnf
(* Top-level structure *)
program = { statement } ;

statement =
    | tempo_def
    | bus_def
    | output_def
    | standalone_command
    | comment ;

tempo_def = ("tempo" | "cps") number ;
bus_def = "~" identifier "=" expression ;    (* = NOT : ! *)
output_def = "out" expression ;              (* NO = here! *)
standalone_command = "hush" | "panic" ;
comment = "#" text ;

(* Expressions - Signal Graph Only *)
expression = chain_expr ;

chain_expr = additive_expr { ">>" additive_expr } ;    (* # works *)

additive_expr = mult_expr { ("+" | "-") mult_expr } ;

mult_expr = primary_expr { "*" primary_expr } ;        (* / not implemented *)

primary_expr =
    | number
    | pattern_string                    (* "bd sn hh cp" *)
    | bus_ref                          (* ~lfo *)
    | function_call                    (* sine(440) *)
    | "(" expression ")" ;

pattern_string = '"' mini_notation '"' ;

bus_ref = "~" identifier ;

function_call =
    | "sine" "(" expr ")"
    | "saw" "(" expr ")"
    | "square" "(" expr ")"
    | "noise"
    | "s" "(" pattern_string ")"
    | "lpf" "(" expr "," expr ")"
    | "hpf" "(" expr "," expr ")" ;
```

### What Does NOT Parse

```ebnf
(* DOCUMENTED but NOT IMPLEMENTED *)
pattern_transform = expression "|>" transform ;     (* $ doesn't work! *)
function_application = transform "$" expression ;    (* $ doesn't work! *)
parameter_application = expression "#" param ;       (* # doesn't work! *)
output_with_equals = "out" "=" expression ;          (* BROKEN! *)
```

---

## Operator Status

### ✅ Working Operators

| Operator | Purpose | Example | Status |
|----------|---------|---------|--------|
| `#` | Signal chain | `saw(55) # lpf(1000, 0.8)` | ✅ Works |
| `+` | Add signals | `~a + ~b` | ✅ Works |
| `-` | Subtract signals | `~a - ~b` | ✅ Works |
| `*` | Multiply signals | `~a * 0.5` | ✅ Works |
| `~` | Bus reference | `~lfo` | ✅ Works |
| `=` | Bus assignment | `~lfo = sine(0.5)` | ✅ Works |

### ❌ Broken Operators

| Operator | Purpose | Example | Status |
|----------|---------|---------|--------|
| `=` | Output assignment | `out = sine(440)` | ❌ Produces silence! |
| `/` | Divide signals | `~a / 2` | ❌ Not implemented |

### 📝 Documented But Not Implemented

| Operator | Purpose | Tidal/Strudel Syntax | Status |
|----------|---------|---------------------|--------|
| `\|>` | Pattern transform | `"bd sn" \|> fast 2` | 📝 In docs only |
| `$` | Function application | `every 4 rev $ s "bd"` | 📝 Not implemented |
| `#` | Parameter application | `s "bd" # gain 0.8` | 📝 Not implemented |

---

## What Works Right Now

### ✅ This Works

```phonon
# Tempo
tempo 2.0

# Bus assignment (with =)
~lfo = sine(0.25)
~bass = saw(55) # lpf(~lfo * 2000 + 500, 0.8)

# Output (NO = sign!)
out ~bass * 0.3

# Samples
out s("bd sn hh*4 cp")

# Samples through effects
out s("bd sn") # lpf(2000, 0.8)

# Pattern-controlled synthesis
out sine("110 220 440") * 0.2

# Signal math
~mix = ~a + ~b * 0.5

# Hush/panic
hush
```

### ❌ This DOESN'T Work

```phonon
# Assignment with = for output - BROKEN!
out = sine(440)         # Produces silence!

# Pattern transformations - NOT IMPLEMENTED!
out s("bd sn") $ fast 2

# Tidal-style - NOT IMPLEMENTED!
out every 4 (fast 2) $ s "bd"

# Parameters - NOT IMPLEMENTED!
out s "bd" # gain 0.8 # pan 0.5

# Division - NOT IMPLEMENTED!
~half = ~signal / 2
```

---

## Correct Syntax (What Actually Works)

### Basic Pattern
```phonon
tempo 2.0
out s("bd sn hh*4 cp")
```

### With Filter
```phonon
tempo 2.0
out s("bd sn hh*4 cp") # lpf(2000, 0.8)
```

### With Bus (NO = on output!)
```phonon
tempo 2.0
~drums = s("bd sn hh*4 cp")
out ~drums # lpf(2000, 0.8)
```

### Pattern Modulation
```phonon
tempo 2.0
~lfo = sine(0.25)
~bass = saw("55 82.5") # lpf(~lfo * 2000 + 500, 0.8)
out ~bass * 0.3
```

---

## What We NEED to Implement

### Priority 1: Fix Broken Stuff

1. **Fix `out =` assignment** (HIGH)
   ```phonon
   out = sine(440)   # Should work, currently broken
   ```

2. **Unify `:` vs `=` confusion** (HIGH)
   - Docs say `:`, code uses `=`
   - Pick one!

### Priority 2: Pattern Transformations

Choose one approach:

#### Option A: Just `$` (Tidal-style)
```phonon
out every 4 (fast 2) $ s "bd"
out sometimes rev $ s "bd sn"
```

**Pros**: Tidal-compatible
**Cons**: Right-to-left reading

#### Option B: Just `$` (Pipe-style)
```phonon
out s "bd" $ fast 2 $ every 4 rev
out s "bd sn" $ sometimes rev
```

**Pros**: Left-to-right, easier to read
**Cons**: Not Tidal-compatible

#### Option C: BOTH (Recommended)
```phonon
# Tidal style
out every 4 (fast 2) $ s "bd"

# Pipe style
out s "bd" $ fast 2 $ every 4 rev

# Both work!
```

**Pros**: Maximum flexibility
**Cons**: Two ways to do same thing

### Priority 3: Parameter Application

```phonon
# Using # operator
out s "bd" # gain 0.8 # pan "0 1 0.5"

# Or keyword arguments?
out s("bd", gain: 0.8, pan: "0 1 0.5")
```

---

## Proposed Complete Grammar

### If We Add `$` and `$`

```ebnf
(* Expressions with all operators *)
expression = pipe_expr ;

pipe_expr = dollar_expr { "|>" transform } ;

dollar_expr = chain_expr [ "$" dollar_expr ] ;

chain_expr = additive_expr { ">>" additive_expr } ;

additive_expr = mult_expr { ("+" | "-") mult_expr } ;

mult_expr = primary_expr { ("*" | "/") primary_expr } ;

(* Pattern transformations *)
transform =
    | identifier                              (* rev, brak *)
    | identifier number                       (* fast 2 *)
    | identifier "(" args ")"                (* every(4, rev) *)
    | identifier transform                    (* every 4 rev *)
    | "(" transform ")" ;

args = arg { "," arg } ;

arg = number | pattern_string | transform ;
```

### Operator Precedence (Proposed)

1. Function calls: `sine(440)`, `s("bd")`
2. `*` `/` (multiplication, division)
3. `+` `-` (addition, subtraction)
4. `#` (signal/DSP chaining)
5. `$` (pattern transformations, left-to-right)
6. `$` (function application, right-to-left)
7. `=` (assignment)

---

## Example: Full Tidal Translation

### Tidal
```haskell
d1 $ sound "bd sn" # gain 0.8
    # lpf (range 500 2000 $ sine 0.25)
    $ every 4 (fast 2)
```

### Phonon (Proposed)
```phonon
tempo 1.0

# With $ operator
~lfo = sine 0.25
~cutoff = ~lfo * 1500 + 750
~drums = s "bd sn" # gain 0.8
~filtered = ~drums # lpf ~cutoff 0.8
out every 4 (fast 2) $ ~filtered

# Or with $ operator
out ~drums # lpf ~cutoff 0.8 $ every 4 (fast 2)

# Or mixed
out every 4 (fast 2) $ ~drums # lpf ~cutoff 0.8
```

---

## Recommendations

1. **Fix `out =` bug IMMEDIATELY** - this is breaking basic usage

2. **Pick assignment operator** - `:` or `=`, not both
   - Recommendation: Use `=` (already works for buses, just fix output)

3. **Implement BOTH `$` and `$`**
   - Let users choose Tidal-style or pipe-style
   - F# and Elixir prove both can coexist

4. **Keep `#` for signal chaining** - already works, Glicol heritage

5. **Add `#` for parameters later** - not critical, can wait

6. **Write pattern transform parser** - functions exist, just wire them up!

---

## Next Steps

1. Fix `out =` (one line change in parser)
2. Implement `$` operator (Tidal compat)
3. Implement `$` operator (pipe style)
4. Wire up existing pattern functions: `fast`, `slow`, `rev`, `every`
5. Update docs to match reality
6. Profit! 🎵

---

## References

- **Glicol**: https://glicol.org/ (source of `#` operator)
- **Tidal**: https://tidalcycles.org/ (source of `$`, `#`, pattern transforms)
- **Strudel**: https://strudel.cc/ (JavaScript port of Tidal)
- **F# pipe**: https://fsharpforfunandprofit.com/posts/function-composition/ (inspiration for `$`)
