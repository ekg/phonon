-- VST3 Plugin Demo for Phonon
-- ============================
--
-- This example demonstrates VST3 effects and instruments with parameter modulation.
--
-- Available VST3 plugins on this system:
--   Effects: Surge XT Effects
--   Instruments: Surge XT, Osirus, Vavra, Xenia, JE8086, NodalRed2x, OsTIrus
--
-- Syntax:
--   Instrument: vst "PluginName" # note "c4 e4 g4"
--   Effect: audio_source # vst "PluginName"
--   Parameter: vst "Plugin" # param_name value
--   LFO Modulation: vst "Plugin" # param_name (~lfo * 0.5)

cps: 1.0

-- ============================================
-- EXAMPLE 1: VST Instrument with note pattern
-- ============================================

-- Surge XT synth playing a simple arpeggio
~synth $ vst "Surge XT" # note "c4 e4 g4 c5"

-- ============================================
-- EXAMPLE 2: Parameter modulation with LFO
-- ============================================

-- LFO for filter modulation (0.5 Hz)
~lfo $ sine 0.5

-- Synth with LFO-modulated filter
~modulated $ vst "Surge XT" # note "c3 g3" # osc1_level (~lfo * 0.3 + 0.5)

-- ============================================
-- EXAMPLE 3: Effect chain
-- ============================================

-- Create a bass tone with built-in oscillator
~bass $ saw 55

-- Process bass through Surge XT Effects
~bass_fx $ ~bass # vst "Surge XT Effects"

-- ============================================
-- EXAMPLE 4: Rhythmic elements
-- ============================================

-- Kick and snare pattern using samples
~drums $ s "bd ~ sd ~"

-- ============================================
-- MIX OUTPUT
-- ============================================

out $ ~synth * 0.2 + ~modulated * 0.2 + ~bass_fx * 0.2 + ~drums * 0.5
