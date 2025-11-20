# Route to Reliable Parallelism

## The Goal

Complex FX pipelines + synthesis → Needs all 16 cores working reliably

## Current Blocker

**Multi-threaded crashes immediately with segfault (no Rust panic)**

This means:
- Memory corruption OR
- Stack overflow OR
- Unsafe code issue

NOT a RefCell borrow panic (those we can debug easily).

## Diagnostic Strategy

### Step 1: Isolate the Crash Point

**Hypothesis**: The crash happens during graph evaluation, not cloning.

**Test**:
```rust
// In main.rs parallel loop, add logging BEFORE process_buffer:
eprintln!("Thread {} starting block {}", thread_id, block_idx);
my_graph.process_buffer(&mut block_buffer);
eprintln!("Thread {} finished block {}", thread_id, block_idx);
```

This will show if crash is in:
- Graph cloning (crash before any "starting" message)
- First process_buffer call (crash after one "starting", no "finished")
- Later in processing (some blocks finish, then crash)

### Step 2: Check for Stack Overflow

**Hypothesis**: Recursive graph evaluation × 16 threads = stack overflow

**Test**:
```bash
# Increase stack size
ulimit -s unlimited
./target/release/phonon render /tmp/test_4voices.ph output.wav --cycles 2
```

If this fixes it → need to reduce stack usage or increase default stack size.

### Step 3: Audit Shared Mutable State

**Every RefCell in UnifiedSignalGraph:**
```rust
sample_bank: RefCell<SampleBank>,         // OK - cloned per thread
voice_manager: RefCell<VoiceManager>,     // OK - cloned per thread
synth_voice_manager: RefCell<SynthVoiceManager>,  // Check this
```

**Every RefCell in SignalNode enum:**
```rust
Oscillator { phase: RefCell<f32>, ... }   // PROBLEM if shared!
Schmidt { state: RefCell<SchmidtState> }  // PROBLEM if shared!
Latch { state: RefCell<LatchState> }      // PROBLEM if shared!
// ... etc
```

**The Issue**: SignalNodes are wrapped in `Rc<SignalNode>` (line 4014):
```rust
nodes: Vec<Option<std::rc::Rc<SignalNode>>>,
```

When we clone the graph, `Rc::clone()` just increments the ref count - **ALL THREADS SHARE THE SAME NODES!**

This is the bug. Each thread's "cloned" graph shares the same Rc<SignalNode> instances, which contain RefCells.

## The Fix: Deep Clone SignalNodes

### Option 1: Replace Rc with Full Clone

**Change**:
```rust
// unified_graph.rs:4014
nodes: Vec<Option<SignalNode>>,  // Remove Rc wrapper
```

**Impact**:
- Heavier clone (full deep copy of all nodes)
- But enables true per-thread independence
- Worth it for parallelism

### Option 2: Make SignalNode Clone-able

**Implement**:
```rust
impl Clone for SignalNode {
    fn clone(&self) -> Self {
        match self {
            SignalNode::Oscillator { freq, waveform, phase, pending_freq, last_sample } => {
                SignalNode::Oscillator {
                    freq: freq.clone(),
                    waveform: *waveform,
                    phase: RefCell::new(*phase.borrow()),  // Deep clone the RefCell contents
                    pending_freq: RefCell::new(*pending_freq.borrow()),
                    last_sample: RefCell::new(*last_sample.borrow()),
                }
            },
            // ... clone all other variants
        }
    }
}
```

Then in `UnifiedSignalGraph::clone()`:
```rust
nodes: self.nodes.iter().map(|opt_rc| {
    opt_rc.as_ref().map(|rc| Rc::new((**rc).clone()))  // Deep clone the node
}).collect(),
```

### Option 3: Arc<RwLock> Instead of Rc<RefCell>

**Change SignalNode fields**:
```rust
Oscillator {
    phase: Arc<RwLock<f32>>,  // Thread-safe
    // ...
}
```

**Pros**:
- Thread-safe
- Can share nodes across threads

**Cons**:
- Lock contention overhead
- Defeats purpose of per-thread graphs

**Verdict**: Option 1 or 2 is better (true independence).

## Recommended Fix Path

### Phase 1: Prove the Diagnosis (30 minutes)

1. Add logging to parallel loop to see where crash happens
2. Check if removing Rc wrapper fixes it (temporarily)
3. Confirm this is the root cause

### Phase 2: Implement Deep Clone (2-4 hours)

1. Implement `Clone` for `SignalNode` enum
2. Update `UnifiedSignalGraph::clone()` to deep-clone nodes
3. Test multi-threaded rendering
4. Verify no crashes

### Phase 3: Re-enable Parallel Voice Processing (2-4 hours)

Once graphs are truly independent:

1. Re-enable Rayon parallel voice processing (voice_manager.rs:1347)
2. Re-enable SIMD optimizations (voice_manager.rs:1315)
3. Test with complex scenarios
4. Measure actual multi-core utilization

### Phase 4: Optimize (Ongoing)

1. Profile to find bottlenecks
2. Optimize hot paths
3. Consider lock-free data structures where needed

## Success Metrics

### Immediate (After Phase 2)
- ✅ Multi-threaded rendering doesn't crash
- ✅ 16 buses × 16 threads = stable
- ✅ Complex FX pipelines work

### Performance (After Phase 3)
- 🎯 16 cores showing ~1500% CPU usage (like early tests)
- 🎯 100+ voices rendering in < 11.6ms
- 🎯 Linear scaling with core count

## Time Estimate

- **Diagnosis**: 30 minutes
- **Core fix (deep clone)**: 2-4 hours
- **Re-enable optimizations**: 2-4 hours
- **Testing & validation**: 1-2 hours

**Total**: 6-11 hours of focused work

## Next Immediate Action

Run the diagnostic logging test to confirm the Rc sharing hypothesis:

```bash
# Add this to main.rs around line 1365, rebuild, test
eprintln!("[Thread] Starting block processing");
my_graph.process_buffer(&mut block_buffer);
eprintln!("[Thread] Finished block processing");
```

If we see "Starting" but no "Finished" → crash is in process_buffer → confirms shared state issue.

Then proceed with deep clone implementation.
