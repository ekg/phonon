The bug is clear now:

**Root Cause:**
All Sample nodes (o1, o2, o3) share a single VoiceManager via RefCell.
When multiple Sample nodes trigger in the same sample period, they interfere.

**Evidence:**
- Sine waves: Perfect (no voice manager) ✅
- Single sample channel: Peak 0.975 ✅  
- Three sample channels (same sample): Peak 4.23 ❌ (should be ~2.92)

**The interference pattern:**
3 channels × 1 sample = Peak 4.23 (instead of expected ~2.92)
This suggests voices are being retriggered or allocated incorrectly.

**Solution Options:**
1. Each output channel gets its own VoiceManager (cleanest)
2. Deduplicate triggers within a single sample period
3. Voice manager tracks which sample/time already triggered

Will implement option that makes most architectural sense.

