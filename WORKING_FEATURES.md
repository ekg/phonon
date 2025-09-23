# Phonon Working Features

## ✅ Working

### DSP Synthesis
- `out: sin 440` - sine wave
- `out: saw 220` - saw wave  
- `out: square 110` - square wave
- `out: noise` - white noise
- `out: sin 440 * 0.5` - amplitude scaling
- `out: sin 440 + sin 880` - mixing

### Pattern Playback (using `s` node)
- `out: s "bd sn hh cp"` - basic pattern
- `out: s "[bd sn]*2"` - fast/condensed pattern 
- `out: s "bd*4"` - repetition
- `out: s "bd(3,8)"` - euclidean rhythms
- `out: s "<bd sn cp>"` - alternation per cycle

### Live Mode
- `phonon live file.phonon` - watches file and reloads on save
- Plays 4 seconds of audio per render by default
- Use `--duration N` to change render duration

## ⚠️ Limitations

### Audio Generation
- Patterns generate sine waves, not actual drum sounds
- No real sample loading yet
- Simple envelope (attack/decay only)

### Pattern Syntax  
- No support for `|>` pipe operator yet (e.g. `"bd sn" |> fast 2`)
- Limited to what fits in a string after `s`
- No pattern variables or functions

### DSP Features
- Filters (lpf, hpf) are very basic
- No real reverb/delay implementation
- No modulation routing

## 🔧 Next Steps

1. **Drum Synthesis**: Implement proper kick, snare, hihat sounds
2. **Pattern Operators**: Add `fast`, `slow`, `rev`, `every` as functions
3. **Better Live Mode**: Seamless audio transitions, no clicks
4. **Sample Loading**: Load actual WAV files from disk