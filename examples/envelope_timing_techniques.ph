-- Envelope Timing & Triggering Techniques in Phonon
-- A comprehensive guide to sequencing envelopes in time

tempo: 0.5

-- ============================================================
-- TECHNIQUE 1: Continuous Envelopes (One-Shot)
-- ============================================================
-- Line, Curve, Segments start immediately and run once
-- They progress through their duration, then hold final value

-- Simple fade-in over 2 seconds
~fade_in: line 0.0 1.0 2.0
~tone1: sine 440 * ~fade_in
~out1: ~tone1 * 0.2

-- Exponential pitch sweep (starts immediately)
~pitch_sweep: xline 110.0 880.0 3.0
~sweep_osc: saw ~pitch_sweep
~out2: ~sweep_osc * 0.1

-- ============================================================
-- TECHNIQUE 2: Gate-Triggered Envelopes (Re-triggerable)
-- ============================================================
-- ADSR, ASR respond to gate signals (0 or 1)
-- Gate HIGH (1) = attack/sustain, Gate LOW (0) = release

-- Use impulse to create a gate pattern (triggers at 4 Hz)
~gate2: impulse 4.0

-- ADSR triggered by the gate
~env2: ~gate2 # adsr 0.01 0.1 0.7 0.2
~synth2: saw 220 * ~env2
~out3: ~synth2 * 0.2

-- ============================================================
-- TECHNIQUE 3: Pattern-Triggered Envelopes
-- ============================================================
-- env_trig uses mini-notation patterns for discrete events
-- Each "x" in the pattern triggers a new envelope

-- Euclidean rhythm triggers (5 hits in 8 steps)
~kick_env: env_trig "x(5,8)" 0.01 0.3 0.0 0.1
~kick: sine 60 * ~kick_env
~out4: ~kick * 0.4

-- ============================================================
-- TECHNIQUE 4: LFO as Gate (Slow Modulation)
-- ============================================================
-- Use Schmidt trigger to convert LFO to gate
-- This creates rhythmic envelope retriggering

-- LFO that oscillates between 0 and 1
~lfo4: sine 2.0 * 0.5 + 0.5

-- Convert to gate using Schmidt trigger (hysteresis)
~gate4: ~lfo4 # schmidt 0.7 0.3

-- Envelope triggered by gate transitions
~env4: ~gate4 # asr 0.05 1.0 0.1
~bass4: saw 55 * ~env4
~out5: ~bass4 * 0.2

-- ============================================================
-- TECHNIQUE 5: Manual Gate Patterns with Segments
-- ============================================================
-- Create custom gate timing with segments
-- Allows precise control over when envelopes trigger

-- Gate pattern: on for 0.2s, off for 0.3s, on for 0.1s, off for 0.4s
~manual_gate: segments "0 1 1 0 0 1 1 0 0" "0.1 0.2 0.0 0.3 0.1 0.1 0.0 0.4"
~env5: ~manual_gate # asr 0.02 1.0 0.05
~pluck5: saw 330 * ~env5
~out6: ~pluck5 * 0.15

-- ============================================================
-- TECHNIQUE 6: Staggered Envelopes (Polyrhythmic)
-- ============================================================
-- Multiple envelopes with different trigger rates
-- Creates complex, evolving textures

-- Three voices with different trigger rates
~gate_a: impulse 3.0
~gate_b: impulse 4.0
~gate_c: impulse 5.0

~env_a: ~gate_a # adsr 0.05 0.2 0.5 0.3
~env_b: ~gate_b # adsr 0.02 0.15 0.6 0.2
~env_c: ~gate_c # adsr 0.01 0.1 0.7 0.25

~voice_a: sine 220 * ~env_a
~voice_b: sine 330 * ~env_b
~voice_c: sine 440 * ~env_c

~out7: (~voice_a + ~voice_b + ~voice_c) * 0.1

-- ============================================================
-- TECHNIQUE 7: Envelope Chaining (Sequential)
-- ============================================================
-- Use one envelope to modulate another's gate
-- Creates sequential, conditional triggering

-- First envelope creates a slow gate
~slow_gate: impulse 0.5
~slow_env: ~slow_gate # adsr 0.1 0.5 0.8 0.5

-- Use slow envelope to gate fast pulses (acts as amplitude envelope)
~fast_gate: impulse 8.0
~fast_env: ~fast_gate # adsr 0.01 0.05 0.0 0.05

-- Multiply to create bursts
~burst_env: ~slow_env * ~fast_env
~burst_osc: saw 165 * ~burst_env
~out8: ~burst_osc * 0.2

-- ============================================================
-- KEY CONCEPTS:
-- ============================================================
-- 1. CONTINUOUS: Line, Curve, Segments start immediately, run once
-- 2. GATE-TRIGGERED: ADSR, ASR respond to 0→1 and 1→0 transitions
-- 3. PATTERN-TRIGGERED: env_trig uses mini-notation for discrete events
-- 4. LFO→GATE: Use schmidt trigger to convert oscillations to gates
-- 5. CUSTOM TIMING: Use segments to create arbitrary gate patterns
-- 6. POLYRHYTHMS: Multiple trigger rates create complex interactions
-- 7. CHAINING: Multiply envelopes for conditional/sequential behavior

-- Mix all examples (adjust levels to taste)
out: ~out1 * 0.3 + ~out2 * 0.2 + ~out3 * 0.3 + ~out4 * 0.4 + ~out5 * 0.25 + ~out6 * 0.2 + ~out7 * 0.25 + ~out8 * 0.3
