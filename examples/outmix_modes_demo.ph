-- Output Mixing Mode Demo
--
-- When you have multiple output channels (o1, o2, o3, etc.),
-- they get summed to mono. The outmix setting controls how.

tempo: 2.0

-- Five mixing modes available:
--
-- outmix: none  -- Direct sum (DEFAULT) - like a hardware mixer
--                   Each channel is independent. Set your own levels!
--                   Can clip if too loud - turn down individual channels.
--
-- outmix: tanh  -- Soft saturation - warm analog-style limiting
--                   Prevents clipping with gentle distortion
--
-- outmix: hard  -- Hard limiter - brick-wall at ±1.0
--                   Prevents clipping with hard ceiling
--
-- outmix: gain  -- Automatic gain compensation (divide by num channels)
--                   WARNING: Channels affect each other! Adding o3 makes o1 quieter!
--
-- outmix: sqrt  -- RMS-based mixing (divide by sqrt of num channels)
--                   WARNING: Channels affect each other! Less extreme than gain.

-- DEFAULT: outmix: none (commented out to show default behavior)
-- outmix: none

-- Three output channels with appropriate levels set manually
o1: s "bd ~ bd ~" * 0.3
o2: s "~ sn ~ sn" * 0.3
o3: s "hh hh hh hh" * 0.2

-- Try this:
-- 1. Render with defaults (none mode)
-- 2. Comment out o3 - levels of o1 and o2 stay the same! ✅
-- 3. Uncomment outmix: gain above
-- 4. Comment out o3 again - o1 and o2 get LOUDER (channels interdependent) ⚠️
--
-- Recommendation: Use default (none) and set appropriate levels yourself.
-- Use tanh/hard only when you specifically want limiting.
