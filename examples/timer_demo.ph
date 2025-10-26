-- Timer Demonstration
-- Timer measures elapsed time since last trigger reset
-- Resets to 0 on rising edge, counts up in seconds
-- Perfect for time-based modulation, envelopes, and sequencing

tempo: 2.0

-- Example 1: Time-based filter sweep
~trig1: impulse 2.0
~time1: ~trig1 # timer
~carrier1: saw 110
~cutoff1: ~time1 * 4000.0 + 200.0
~swept1: ~carrier1 # lpf ~cutoff1 0.8
~out1: ~swept1 * 0.3

-- Example 2: Time-based pitch glide
~trig2: impulse 3.0
~time2: ~trig2 # timer
~freq2: 220.0 + ~time2 * 100.0
~glide2: saw ~freq2
~out2: ~glide2 * 0.2

-- Example 3: Fast sweep with rapid triggers
~fast_trig3: impulse 8.0
~time3: ~fast_trig3 # timer
~fast_sweep3: sine (110.0 + ~time3 * 440.0)
~out3: ~fast_sweep3 * 0.2

-- Example 4: Time-based resonance sweep
~trig4: impulse 2.0
~time4: ~trig4 # timer
~osc4: saw 82.5
~res4: ~time4 * 5.0 + 0.5
~filtered4: ~osc4 # lpf 800.0 ~res4
~out4: ~filtered4 * 0.25

-- Example 5: Slow continuous sweep (no trigger)
~no_trig5: 0.0
~time5: ~no_trig5 # timer
~continuous5: sine (55.0 + ~time5 * 10.0)
~out5: ~continuous5 * 0.15

-- Example 6: Multiple timers at different rates
~trig6a: impulse 1.0
~trig6b: impulse 2.0
~time6a: ~trig6a # timer
~time6b: ~trig6b # timer
~freq6a: 330.0 + ~time6a * 50.0
~freq6b: 440.0 + ~time6b * 75.0
~tone6a: sine ~freq6a
~tone6b: saw ~freq6b
~out6: ~tone6a * 0.15 + ~tone6b * 0.15

-- Example 7: Timer modulating amplitude
~trig7: impulse 1.5
~time7: ~trig7 # timer
~amp7: ~time7 * 0.5
~tone7: sine 550
~modulated7: ~tone7 * ~amp7
~out7: ~modulated7 * 0.3

-- Example 8: Timer controlling filter frequency
~trig8: impulse 2.5
~time8: ~trig8 # timer
~carrier8: saw 165
~cutoff8: 300.0 + ~time8 * 3000.0
~swept8: ~carrier8 # lpf ~cutoff8 1.2
~out8: ~swept8 * 0.25

-- Example 9: Dual parallel timers
~trig9: impulse 4.0
~time9a: ~trig9 # timer
~time9b: ~trig9 # timer
~mod9: ~time9a * ~time9b * 200.0 + 220.0
~tone9: saw ~mod9
~out9: ~tone9 * 0.2

-- Example 10: Timer with rhythmic triggers
~rhythm10: impulse 6.0
~time10: ~rhythm10 # timer
~pitch10: 110.0 + ~time10 * 220.0
~rhythmic10: sine ~pitch10
~out10: ~rhythmic10 * 0.25

-- Mix all examples
out: ~out1 * 0.5 + ~out2 * 0.5 + ~out3 * 0.4 + ~out4 * 0.5 + ~out5 * 0.3 + ~out6 * 0.6 + ~out7 * 0.5 + ~out8 * 0.5 + ~out9 * 0.4 + ~out10 * 0.5
