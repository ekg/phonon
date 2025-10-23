-- Test pattern-driven synthesis through main parser
-- Pattern controls oscillator frequency
melody = saw "220 330 440 330"
out melody * 0.2