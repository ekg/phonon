# Test if pattern gating works
gate = "1 0 1 0"
tone = sine 440 * gate
out tone * 0.3
