# Dataflow Architecture - Known Issues

**Status**: Context update fix implemented, testing in progress (USE_DATAFLOW = true)
**Date**: 2025-11-21
**Update**: 2025-11-21 - Context update mechanism implemented (Option 1)

## Critical Issue: Static ProcessContext

### Problem

The current dataflow implementation has a fundamental issue with ProcessContext management:

1. **NodeTasks use static context**: When DataflowGraph is created, each NodeTask receives a ProcessContext with initial values (cycle_position=0, sample_count=0)
2. **Context never updates**: The ProcessContext is not updated between blocks
3. **Pattern nodes break**: Pattern-based nodes (Sample, Pattern, etc.) rely on advancing cycle_position to trigger events correctly

### Impact

**Test Results**:
- USE_DATAFLOW = false: ALL TESTS PASS
- USE_DATAFLOW = true: 21 TESTS FAIL (out of 1784 total)

**Affected Systems**:
- Pattern sequencing (samples trigger at wrong times)
- Temporal effects (delays, loops don't advance properly)
- Any node that uses ProcessContext for timing

### Root Cause

In batch-synchronous BlockProcessor:
```rust
// Context created fresh each block with updated values
let context = ProcessContext::new(
    self.cycle_position.clone(),  // <- Updated each block
    0,
    buffer.len(),
    self.tempo,
    self.sample_rate,
);
block_processor.process_block(buffer, &context)?;
```

In dataflow architecture:
```rust
// Context created ONCE and never updated
let context = ProcessContext::new(...);  // Created in build_processor()
DataflowGraph::new(nodes, output_node, context)?;  // Passed to NodeTasks
// NodeTasks keep this same context forever!
```

### Proposed Solutions

#### Option 1: Context Update Messages (Recommended)
Send context updates through a dedicated control channel:
```rust
struct NodeTask {
    context_rx: Receiver<ProcessContext>,  // Receive context updates
    // ... other fields
}

impl DataflowGraph {
    fn process_block(&mut self, output: &mut [f32]) -> Result<(), String> {
        // Send updated context to all nodes
        let context = ProcessContext::new(/* updated values */);
        for tx in &self.context_txs {
            tx.send(context.clone())?;
        }
        // Then send trigger and process
    }
}
```

#### Option 2: Atomic Context Sharing
Use Arc<AtomicCell<ProcessContext>> for lock-free updates:
```rust
struct NodeTask {
    context: Arc<AtomicCell<ProcessContext>>,
}

impl DataflowGraph {
    fn process_block(&mut self, output: &mut [f32]) -> Result<(), String> {
        // Update shared context
        self.shared_context.store(updated_context);
        // Process block
    }
}
```

#### Option 3: Self-Updating Context
NodeTasks track their own cycle position:
```rust
impl NodeTask {
    fn run(mut self) {
        loop {
            // Update context before processing
            self.context.sample_count += 512;
            self.context.cycle_position = advance_by_samples(...);
            // Process block
        }
    }
}
```

### Recommendation

**Implement Option 1** (Context Update Messages):
- Clean separation of concerns
- Centralized time management in DataflowGraph
- No race conditions
- Easy to test

### Timeline to Fix

1. **Short term** (this session): Keep USE_DATAFLOW = false, document issue
2. **Medium term** (next session): Implement context update mechanism
3. **Long term**: Full test suite passing with dataflow enabled

### Work Required

1. Add `context_rx` channel to NodeTask
2. Add `context_txs` to DataflowGraph
3. Update DataflowGraph::process_block() to broadcast context updates
4. Update NodeTask::run() to receive and apply context updates
5. Test thoroughly with pattern-based nodes

**Estimated time**: 2-3 hours

---

## Other Minor Issues

### Test Flakiness
- 2 DataflowGraph tests ignored due to intermittent hanging
- Likely related to thread startup timing
- Needs proper synchronization barrier instead of sleep()

### Thread Lifecycle
- NodeTasks don't have clean shutdown when graph is dropped
- Should implement Drop trait to signal shutdown

---

## Current State

**Previous State**: USE_DATAFLOW = false (all tests passing)
**Current State**: USE_DATAFLOW = true (context update mechanism implemented)

## Implementation Update (2025-11-21)

**Status**: Context update mechanism (Option 1) has been fully implemented.

### Changes Made:
1. **NodeTask**: Added `context_rx` channel to receive ProcessContext updates before each block
2. **DataflowGraph**: Added `context_txs` to broadcast context to all nodes before processing
3. **AudioNodeGraph**: Modified to create and pass ProcessContext to DataflowGraph
4. **Tests**: Fixed all NodeTask and DataflowGraph tests to provide context channels

### Code Changes:
- `src/node_task.rs`: Added context_rx field and updated run() loop
- `src/dataflow_graph.rs`: Added context broadcasting mechanism
- `src/audio_node_graph.rs`: Updated dataflow branch to pass context
- All tests updated to provide context channels

### Testing:
Full test suite is running to verify the fix resolves the 21 test failures.

**See `DATAFLOW_CONTEXT_UPDATE_STATUS.md` for detailed implementation notes.**

The architecture is sound, the fix has been implemented, verification in progress.
