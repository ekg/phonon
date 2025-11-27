-- DYNAMIC BUS SYSTEM DEMONSTRATION
-- Phase 4: Dynamic Everything - Conditional Routing & Signal Selection
--
-- This example demonstrates:
-- 1. Conditional (if-then-else) routing
-- 2. Select/multiplex between multiple signals
-- 3. Pattern-controlled signal selection
-- 4. LFO-based dynamic effect switching

tempo: 0.5

-- ============================================================================
-- PART 1: Conditional Routing (Dynamic Effect Application)
-- ============================================================================

-- Create a bass line
~bass_freq: "55 82.5 110 73.33"
~bass: saw ~bass_freq

-- Create an envelope to control effect routing
~env: adsr 0.01 0.1 0.5 0.2

-- Dry signal (no effect)
~dry_bass: ~bass

-- Wet signal (with heavy filtering)
~wet_bass: ~bass # lpf 300 0.9

-- Use conditional to route between dry and wet based on envelope
-- When envelope > 0.5, use wet (filtered) bass
-- When envelope <= 0.5, use dry bass
~conditional_bass: if ~env ~wet_bass ~dry_bass

-- ============================================================================
-- PART 2: Select - Pattern-Based Signal Selection
-- ============================================================================

-- Create four different tones
~tone_a: sine 220   -- A3
~tone_b: sine 330   -- E4
~tone_c: sine 440   -- A4
~tone_d: sine 550   -- C#5

-- Pattern cycles through indices 0, 1, 2, 3
-- Each index selects a different tone
~selector: "0 1 2 3"
~melody: select ~selector ~tone_a ~tone_b ~tone_c ~tone_d

-- Add envelope to melody
~melody_shaped: ~melody * adsr 0.01 0.05 0.0 0.1

-- ============================================================================
-- PART 3: LFO-Based Dynamic Routing
-- ============================================================================

-- Create a pad sound
~pad: saw 110

-- Two different effect chains
~reverb_pad: ~pad # reverb 0.8 0.5 0.5
~delay_pad: ~pad # delay 0.25 0.6 0.4

-- LFO slowly crossfades between the two effects
~lfo: sine 0.25
~dynamic_pad: if ~lfo ~reverb_pad ~delay_pad

-- ============================================================================
-- PART 4: Multi-Way Selection with Continuous Index
-- ============================================================================

-- Create four harmonic frequencies
~harm1: sine 110    -- Fundamental
~harm2: sine 220    -- 2nd harmonic
~harm3: sine 330    -- 3rd harmonic
~harm4: sine 440    -- 4th harmonic

-- Use LFO to continuously sweep through harmonics
-- LFO output (-1 to 1) is scaled to index range (0 to 3)
~harm_selector: (~lfo + 1.0) * 1.5
~harmonic_sweep: select ~harm_selector ~harm1 ~harm2 ~harm3 ~harm4

-- ============================================================================
-- OUTPUT MIX
-- ============================================================================

-- Mix all parts together
~mix: ~conditional_bass * 0.3 +
      ~melody_shaped * 0.2 +
      ~dynamic_pad * 0.15 +
      ~harmonic_sweep * 0.1

out: ~mix
