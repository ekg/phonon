# Performance Fixes - December 2025

## Changes Made This Session

### 1. Voice Cleanup Fix (voice_manager.rs:548-550)
When a sample finishes playing, ADSR envelopes now properly trigger `release()` instead of staying in Sustain forever. This prevents voice accumulation.

### 2. Buffer Sizes Increased (main.rs)
- Synthesis buffer: 512 → 4096 samples (line 1891)
- Ring buffer: 1s → 2s (line 1840)

### 3. Voice Buffer Architecture Refactor (voice_manager.rs:1556-1625)
Changed `process_buffer_per_node` return type from:
- OLD: `Vec<HashMap<usize, f32>>` - 4096 HashMaps, one per sample
- NEW: `HashMap<usize, Vec<f32>>` - one HashMap, buffer per source_node

### 4. Profiling Instrumentation (main.rs:1902-1919)
Added "🐢 Slow synthesis" logging when synthesis exceeds audio budget.

### 5. Vec-based Voice Buffer Optimization (DONE)

Replaced HashMap with Vec<Vec<f32>> for O(1) lookup in hot loop:

**Implementation:**
```rust
// NEW: VoiceBuffers struct (voice_manager.rs:106-195)
pub struct VoiceBuffers {
    pub buffers: Vec<Vec<f32>>,  // Indexed by node_id, each is buffer_size samples
    pub buffer_size: usize,
    pub max_active_node: usize,
}

// NEW: process_buffer_vec() method (voice_manager.rs:1718-1780)
pub fn process_buffer_vec(&mut self, buffer_size: usize, max_node_id: usize) -> VoiceBuffers

// Hot loop optimization (unified_graph.rs:13144-13149):
// OLD: Rebuild HashMap every sample (~800μs per 4096-sample buffer)
self.voice_output_cache.clear();
for (&source_node, buffer) in &voice_buffers {
    self.voice_output_cache.insert(source_node, buffer[i]);
}

// NEW: Just set sample index for O(1) lookup
self.current_sample_idx = i;

// Lookup in eval_node (unified_graph.rs:10444):
let buffer_output = self.voice_buffers.get(node_id.0, self.current_sample_idx);  // O(1)!
```

**Performance Gain:**
- HashMap lookup: ~20-50ns per access (hash + probe)
- Vec index: ~1ns per access (direct memory)
- Per 4096-sample buffer with 10 nodes: saves ~800μs (40,960 HashMap operations eliminated)

## Files Modified
- `src/main.rs` - buffer sizes, profiling
- `src/voice_manager.rs` - VoiceBuffers struct, process_buffer_vec() method
- `src/unified_graph.rs` - Vec-based lookup, current_sample_idx tracking

## Profiling Results
With PROFILE_BUFFER=1:
- Most buffers: 1-4ms (well under 85ms budget)
- Occasional spikes: ~48ms (rayon thread pool init, one-time)
- 100% of time in voice processing (expected)
