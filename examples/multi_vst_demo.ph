-- Multi-VST Demo for Phonon (Surge XT Edition)
-- =============================================
--
-- This demo uses Surge XT for instruments and effects
-- Syntax: vst "PluginName" # note "c4 e4 g4"

cps: 0.5

-- ============================================
-- BASS: Surge XT playing low notes
-- ============================================
~bass $ vst "Surge XT" # note "c2 ~ c2 eb2"

-- ============================================
-- PAD: Surge XT with chord pattern
-- ============================================
~pad $ vst "Surge XT" # note "c4 eb4 g4 bb4"

-- ============================================
-- LEAD: Surge XT with melody
-- ============================================
~lead $ vst "Surge XT" # note "c5 eb5 g5 c6"

-- ============================================
-- DRUMS: Sample-based rhythm
-- ============================================
~drums $ s "bd ~ sd ~"

-- ============================================
-- EFFECT: Route pad through Surge XT Effects
-- ============================================
~pad_fx $ ~pad # vst "Surge XT Effects"

-- ============================================
-- MIX OUTPUT
-- ============================================
out $ ~bass * 0.3 + ~pad_fx * 0.2 + ~lead * 0.2 + ~drums * 0.4
