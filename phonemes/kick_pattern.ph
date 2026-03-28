-- Four-on-the-floor kick pattern
-- Each "100" is a kick, "~" is silence
out noise # lpf("100 ~ ~ ~ 100 ~ ~ ~ 100 ~ ~ ~ 100 ~ ~ ~", 20) * 0.5
