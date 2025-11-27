-- Convolution Reverb Demo
-- Demonstrates realistic room acoustics using impulse response convolution
--
-- Convolution reverb captures the acoustic signature of real spaces:
-- - Concert halls
-- - Churches
-- - Recording studios
-- - Small rooms
--
-- This implementation uses a built-in IR with early reflections
-- and exponential decay tail (RT60 ≈ 0.3s)

tempo: 0.5

-- Example 1: Dry percussion vs convolution reverb
-- Listen to the difference - convolution adds space and depth

~kick: saw "55 ~ 82.5 ~" * 0.6
~dry: ~kick

-- Apply convolution reverb to add realistic room acoustics
~wet: convolve ~kick * 0.5

-- Mix dry and wet for comparison (try commenting one out)
-- out: ~dry          -- Dry: tight, up-front percussion
-- out: ~wet          -- Wet: spacious, reverberant percussion
out: ~dry * 0.3 + ~wet * 0.7    -- Blend: punchy with space

-- Example 2: Convolution on chord progressions
-- Reverb adds lushness and depth to harmonic content

-- ~chord: saw "110 165 220" $ slow 2
-- ~reverb_chord: convolve ~chord * 0.3
-- out: ~reverb_chord

-- Example 3: Percussive sequences
-- Early reflections and decay tail create realistic room feel

-- ~drums: saw "220 ~ 330 ~ 440 ~ 550 ~"
-- ~room: convolve ~drums * 0.4
-- out: ~room

-- Example 4: Comparison - Algorithmic vs Convolution
-- Both create reverb, but with different character:
-- - reverb: Freeverb-style algorithmic (adjustable parameters)
-- - convolve: Impulse response-based (fixed room signature)

-- ~bass: saw 55
-- ~algorithmic: reverb ~bass 0.8 0.5 0.3
-- ~convolution: convolve ~bass * 0.3
-- out: ~convolution

-- Technical Details:
-- - Built-in IR: 500ms length
-- - Early reflections: 21ms, 43ms, 67ms, 89ms, 121ms, 156ms
-- - Decay: Exponential tail (RT60 ≈ 0.3 seconds)
-- - Algorithm: Direct time-domain convolution
--
-- Future enhancements:
-- - User-loadable impulse responses (.wav files)
-- - FFT-based fast convolution (for longer IRs)
-- - Partitioned convolution (for real-time with long IRs)
-- - Wet/dry mix parameter

-- Sound Design Tips:
-- 1. Convolution works great on:
--    - Percussion (adds realistic space)
--    - Pads and chords (adds depth and lushness)
--    - Bass (subtle use adds warmth)
--
-- 2. Comparison with algorithmic reverb:
--    - Convolution: realistic, captures actual spaces
--    - Algorithmic: flexible, adjustable parameters
--
-- 3. Mixing:
--    - Start with ~30% wet for subtle space
--    - Go 70%+ wet for ambient/experimental sounds
--    - Layer dry + wet for punchy-yet-spacious mixes
--
-- 4. Creative uses:
--    - Short IRs: create unique timbral coloration
--    - Long IRs: massive ambient washes
--    - Reverse IRs: swell/build-up effects (future)
