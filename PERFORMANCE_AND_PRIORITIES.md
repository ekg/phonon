# Performance Verification & Next Priorities

## 🔬 Performance Test Results

### What We Discovered

**Buffer Refactor Status**: ✅ **WORKING**
- Code is using `process_buffer()` path (confirmed by "Profiling mode" message)
- Single-threaded performance is improved

**Multi-Threading Status**: ❌ **BROKEN** 
```
thread '<unnamed>' panicked at src/unified_graph.rs:5870:47:
RefCell already borrowed
```

This is a **critical blocker** for performance!

---

## 📊 Priority Assessment

Based on testing, here's the recommended priority order:

### **P0 (CRITICAL - Blocks Performance):** Multi-Threading Fix
**Why**: RefCell conflicts prevent parallel processing
**Impact**: Without this, we can't utilize all 16 CPU cores
**Effort**: 2-3 days (replace RefCell with Arc<Mutex> or message passing)
**Benefit**: 10-16x potential speedup on complex patches

### **P1 (HIGH - User Experience):** DAW-like Live Parameter Changes
**Why**: Core functionality for live coding
**Current Status**: Needs investigation - can effects change in realtime?
**Effort**: 1-2 days
**Benefit**: Essential for live performance workflow

### **P2 (HIGH - Discoverability):** Autocomplete + Keyword Arguments
**Why**: Makes system learnable without docs
**Components**:
1. Keyword argument syntax (`lpf freq:1000 q:0.8`)
2. Automatic function metadata extraction
3. Runtime tab completion database
**Effort**: 3-4 days  
**Benefit**: Dramatically improves new user onboarding

---

## 🚨 The Multi-Threading Problem (P0)

### Root Cause
SignalNode state uses `RefCell` for interior mutability:
```rust
pub struct OscillatorState {
    phase: RefCell<f32>,  // ← Multiple threads trying to borrow!
}
```

When parallel processing tries to evaluate the same node from multiple threads:
```
Thread 1: state.phase.borrow_mut()  // Gets lock
Thread 2: state.phase.borrow_mut()  // PANIC! Already borrowed
```

### Solution Options

**Option A: Arc<Mutex<T>> (Thread-Safe)**
```rust
pub struct OscillatorState {
    phase: Arc<Mutex<f32>>,
}
```
- Pro: Simple, correct
- Con: Mutex contention overhead

**Option B: Lock-Free Per-Thread State**
```rust
pub struct OscillatorState {
    phases: Vec<f32>,  // One per thread
    thread_id: ThreadLocal<usize>,
}
```
- Pro: No contention
- Con: More complex, merging required

**Option C: Message Passing (Actor Model)**
```rust
// Each node processes in its own thread
// Communicate via channels
```
- Pro: Clean separation
- Con: Significant architecture change

**Recommended**: **Option A** for now (get it working), then **Option B** for optimization

---

## 🎯 Implementation Plan

### Week 1: Multi-Threading Fix (P0)
**Days 1-2**: 
- Replace RefCell with Arc<Mutex<T>> in all state structs
- Update eval_node_buffer to handle mutex locks
- Test on all 60 nodes

**Day 3**:
- Benchmark multi-threaded performance
- Verify 10-16x speedup on heavy patches
- Regression test suite

**Deliverable**: Parallel processing actually works!

### Week 2: Live Parameter Changes (P1)
**Days 1-2**:
- Investigate current live mode behavior
- Test if parameters can change mid-render
- Implement atomic parameter updates if needed

**Deliverable**: Effects can be tweaked while playing

### Week 3: Autocomplete System (P2)
**Days 1-2**:
- Implement keyword argument parsing (`func param1:val1 param2:val2`)
- Extract function signatures from compiler
- Generate autocomplete database

**Days 3-4**:
- Integrate with live editor completion
- Add parameter hints and validation
- Documentation tooltips

**Deliverable**: Tab-complete shows all functions + parameters

---

## 📈 Expected Performance After Multi-Threading

Current (single-threaded):
```
Simple patch:   0.21s for 100 cycles (398% CPU - throttled)
Complex patch:  FAILS due to RefCell panic
```

After fix (16 cores):
```
Simple patch:   0.05s for 100 cycles (1200%+ CPU)
Complex patch:  0.8s for 100 cycles (800%+ CPU)  
Extreme patch:  Real-time capable!
```

**Performance multiplier**: 10-16x on heavy patches

---

## 🎵 DAW-Like Behavior Questions

### Current Unknown Status:
1. **Can filter cutoff change while playing?**
   - Need to test: `~bass: saw 55 # lpf 1000 0.8` → change 1000 to 2000 live
   
2. **Do changes apply immediately or next cycle?**
   - Test with live editor modifications
   
3. **Is there audio glitching during parameter changes?**
   - Measure if cross-fades are needed

### To Investigate:
```bash
# Start live mode
./target/release/phonon live

# Try changing parameters mid-playback:
~bass: saw 55 # lpf 1000 0.8   # Initial
~bass: saw 55 # lpf 5000 0.8   # Modified - does this work?
```

---

## 🔧 Autocomplete System Design

### Keyword Argument Syntax
```phonon
-- Current (positional):
~filt: saw 110 # lpf 1000 0.8

-- Proposed (keyword):
~filt: saw freq:110 # lpf cutoff:1000 q:0.8

-- Mixed (keywords optional for clarity):
~filt: saw 110 # lpf cutoff:1000 0.8  # Cutoff explicit, Q positional
```

### Metadata Extraction
```rust
// Build script or macro to generate:
pub struct FunctionMeta {
    name: &'static str,
    params: &'static [ParamMeta],
    description: &'static str,
}

const FUNCTION_DB: &[FunctionMeta] = &[
    FunctionMeta {
        name: "lpf",
        params: &[
            ParamMeta { name: "cutoff", type: "frequency", default: None },
            ParamMeta { name: "q", type: "float", default: Some("0.707") },
        ],
        description: "Low-pass filter (2-pole resonant)",
    },
    // ... auto-generated for all functions
];
```

### Tab Completion Integration
```
User types: lpf <TAB>
Shows:      lpf cutoff:<frequency> q:<resonance>
            Low-pass filter (2-pole resonant)

User types: lpf 1000 <TAB>
Shows:      q:<resonance>
```

---

## Recommendation

**Start with P0 (Multi-Threading)** because:
1. It's blocking performance improvements RIGHT NOW
2. It affects everything else (autocomplete testing, DAW testing)
3. It's a known, solvable problem
4. 2-3 day effort for 10-16x performance gain

Then P1 (DAW behavior), then P2 (Autocomplete).

**All three are important**, but multi-threading unblocks the others.

---

Ready to proceed with multi-threading fix?
