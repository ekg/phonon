-- Send/Return Reverb Example
-- Classic mixing technique: multiple sources share one reverb
-- Like an aux send on a mixing console

tempo: 2.0

-- Two dry sources
~dry1: sine 440 * 0.3
~dry2: saw 220 * 0.3

-- Send both to shared reverb (mix them first)
~send: (~dry1 + ~dry2) * 0.5
~return: ~send # reverb 0.9 0.5 0.9

-- Mix dry signals + reverb return
out: ~dry1 * 0.4 + ~dry2 * 0.4 + ~return * 0.3

-- Try adjusting:
-- - reverb room size (0.9) for different spaces
-- - return level (0.3) for more/less ambience
-- - add more sources to the send bus
