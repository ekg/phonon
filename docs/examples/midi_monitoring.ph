-- MIDI Real-Time Monitoring Example
-- Shows how to use ~midi buses for real-time MIDI playthrough
--
-- Requirements:
-- 1. Connect MIDI device (Alt+M in phonon edit)
-- 2. Play notes on your MIDI keyboard
-- 3. Hear immediate audio response (<10ms latency)

tempo: 0.5

-- Basic MIDI monitoring: All channels mixed together
~piano: saw ~midi

-- Add ADSR envelope for more natural sound
~env_piano: ~piano # adsr 0.01 0.1 0.7 0.2

-- Multi-channel setup: Different synths per MIDI channel
~ch1: saw ~midi1       -- Channel 1: Saw wave
~ch2: square ~midi2    -- Channel 2: Square wave
~ch3: triangle ~midi3  -- Channel 3: Triangle wave

-- Process channels with effects
~ch1_fx: ~ch1 # lpf 2000 0.8 # reverb 0.5 0.8
~ch2_fx: ~ch2 # lpf 1000 0.6
~ch3_fx: ~ch3 # hpf 200 0.5

-- Mix channels
~mix: ~ch1_fx * 0.4 + ~ch2_fx * 0.3 + ~ch3_fx * 0.3

-- Output
out: ~env_piano * 0.7 + ~mix * 0.3

-- WORKFLOW:
-- 1. Launch editor: cargo run --release --bin phonon -- edit
-- 2. Connect MIDI: Alt+M (cycles through available devices)
-- 3. Play your keyboard → hear real-time audio
-- 4. Record pattern: Alt+R (start), play notes, Alt+R (stop)
-- 5. Smart paste: Alt+Shift+I → creates ~rec1: slow N $ n "..." # gain "..."
-- 6. Separate paste: Alt+I (notes), Alt+N (offsets), Alt+V (velocities)
