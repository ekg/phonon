# Phonon Working Syntax Reference

## ✅ CONFIRMED WORKING EXAMPLES

All examples tested and producing audio.

### Basic Sample Playback

```phonon
tempo: 0.5
out: s("bd sn hh cp")
```

### Euclidean Rhythms

```phonon
tempo: 0.5
out: s("bd(3,8)")
```

### Alternating Euclidean Pulses

```phonon
tempo: 0.5
out: s("bd(<3 4 5>,8)")
```

**IMPORTANT:** Use SPACES not commas inside alternation: `<3 4 5>` ✅ NOT `<3,4,5>` ❌

### Multiple Patterns

```phonon
tempo: 0.5
out: s("bd sn hh cp cp")
```

### Pattern Repeats

```phonon
tempo: 0.5
out: s("bd*4")
```

### Rests

```phonon
tempo: 0.5
out: s("bd ~ sn ~")
```

### Simple Alternation

```phonon
tempo: 0.5
out: s("<bd sn cp>")
```

## Key Syntax Rules

1. **`out:` with colon** - NOT `out =` or `out`
2. **`s("pattern")`** - Parentheses ARE required
3. **Quotes around pattern** - `s("bd sn")` NOT `s(bd sn)`
4. **Spaces in alternation** - `<3 4 5>` NOT `<3,4,5>`
5. **No comments with #** - Comments break the parser
6. **Tempo with colon** - `tempo: 0.5`

## Commands

```bash
# Render to WAV file (8 cycles at tempo 0.5 = 16 seconds)
cargo run --release --bin phonon -- render COMPLETE_EXAMPLE.phonon output.wav --cycles 8

# Live mode (different parser, different syntax - USE SPACES)
# NOT RECOMMENDED - use render mode instead
```

## What DOESN'T Work

- `out = s("bd")` ❌ (wrong assignment)
- `out s("bd")` ❌ (missing colon)
- `s "bd"` ❌ (missing parentheses)
- `s(bd)` ❌ (missing quotes)
- `bd(<3,4>,8)` ❌ (commas in alternation)
- `# comments` ❌ (breaks parser)

## Testing

To verify a pattern works:

```bash
cargo run --release --bin phonon -- render yourfile.phonon test.wav --cycles 4
```

If you see `RMS level: 0.000` then it didn't work.
If you see `RMS level: 0.XXX` (non-zero) then it worked!
