## Summary of Bug Found

**The Problem:**
- voice_manager and sample_bank are shared via RefCell across the entire graph
- When rendering o1 + o2 + o3 together, all channels share the same voice manager
- This causes voice stealing, double-triggering, or other interference

**Test Results:**
- Individual channels: o2 alone works (peak 0.75)
- Combined channels: all together is SILENT (peak 0.0)
- Manual sum vs. rendered together: 99.5% of samples don't match\!
- Max difference: 3.4 (should be ~0.0)

This confirms your suspicion - samples are "switching" because voice allocations are interfering between channels when they share state.

**The Fix:**
Need to ensure each output channel evaluation doesn't interfere with others. Options:
1. Clear voice manager state between channel evaluations
2. Give each output channel its own voice manager
3. Ensure sample triggering only happens once per sample, not per channel

