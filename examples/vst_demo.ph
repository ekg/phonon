-- VST3 Plugin Demo for Phonon
-- ============================
--
-- This example demonstrates VST3 effects and instruments.
--
-- Available VST3 plugins on this system:
--   Effects: Surge XT Effects
--   Instruments: Surge XT, Osirus, Vavra, Xenia, JE8086, NodalRed2x, OsTIrus
--
-- Syntax:
--   Instrument: vst "PluginName" # note "c4 e4 g4"
--   Effect: audio_source # vst "PluginName"

cps: 1.0

-- ============================================
-- EXAMPLE 1: VST Instrument with note pattern
-- ============================================

-- Surge XT synth playing a simple arpeggio
~synth $ vst "Surge XT" # note "c4 e4 g4 c5"

-- ============================================
-- EXAMPLE 2: Effect chain
-- ============================================

-- Create a bass tone with built-in oscillator
~bass $ saw 55

-- Process bass through Surge XT Effects
~bass_fx $ ~bass # vst "Surge XT Effects"

-- ============================================
-- EXAMPLE 3: Rhythmic elements
-- ============================================

-- Kick and snare pattern using samples
~drums $ s "bd ~ sd ~"

-- ============================================
-- MIX OUTPUT
-- ============================================

out $ ~synth * 0.3 + ~bass_fx * 0.3 + ~drums * 0.5
