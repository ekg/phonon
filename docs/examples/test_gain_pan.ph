tempo: 2.0
-- Test gain and pan DSP parameters

-- Test simple gain (reduce volume)
~quiet: s "bd sn hh cp" # gain 0.3

-- Test pattern-controlled gain
~dynamic: s "bd*4" # gain "0.2 0.5 0.8 1.0"

-- Test LFO-controlled gain (tremolo effect)
~lfo: sine
~tremolo: s "cp*8" # gain (range 0.3 1.0 ~lfo)

-- Combine all
out: (~quiet + ~dynamic + ~tremolo) * 0.5
