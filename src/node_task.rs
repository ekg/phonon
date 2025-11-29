/// Node task - wraps AudioNode as continuous async task for dataflow architecture
///
/// This module implements the core dataflow primitive: a continuously running
/// task that processes audio blocks as messages flow through the graph.
///
/// # Architecture
/// - Each NodeTask wraps one AudioNode (oscillator, filter, effect, etc.)
/// - Runs continuously in background (tokio task)
/// - Receives input buffers via channels (non-blocking when ready)
/// - Processes block (512 samples)
/// - Sends output buffers to downstream nodes (flows immediately)
/// - Returns buffers to pool for recycling
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use crate::buffer_pool::BufferPool;
use crossbeam::channel::{Receiver, Sender};
use std::sync::Arc;

/// Continuous audio processing task
///
/// Each NodeTask runs independently, processing blocks as inputs arrive.
/// This enables natural parallelism and pipelining across blocks.
pub struct NodeTask {
    /// Node ID for debugging/tracing
    id: NodeId,

    /// The AudioNode implementation (oscillator, filter, etc.)
    node: Box<dyn AudioNode>,

    /// Input channels from dependencies
    /// Empty for source nodes (oscillators with constant freq)
    input_rx: Vec<Receiver<Arc<Vec<f32>>>>,

    /// Output channel to downstream nodes
    /// Vec to support multiple consumers (fan-out)
    output_tx: Vec<Sender<Arc<Vec<f32>>>>,

    /// Context update channel
    /// Receives updated ProcessContext before each block
    context_rx: Receiver<ProcessContext>,

    /// Shared buffer pool for recycling allocations
    buffer_pool: Arc<BufferPool>,

    /// Current processing context (cycle position, tempo, etc.)
    /// Updated from context_rx each block
    context: ProcessContext,

    /// Shutdown signal
    shutdown: Arc<std::sync::atomic::AtomicBool>,
}

impl NodeTask {
    /// Create a new node task
    ///
    /// # Arguments
    /// * `id` - NodeId for debugging
    /// * `node` - AudioNode to wrap
    /// * `input_rx` - Input channels (one per dependency)
    /// * `output_tx` - Output channels (one per dependent)
    /// * `context_rx` - Context update channel
    /// * `buffer_pool` - Shared buffer pool
    /// * `context` - Initial processing context
    /// * `shutdown` - Shutdown signal
    pub fn new(
        id: NodeId,
        node: Box<dyn AudioNode>,
        input_rx: Vec<Receiver<Arc<Vec<f32>>>>,
        output_tx: Vec<Sender<Arc<Vec<f32>>>>,
        context_rx: Receiver<ProcessContext>,
        buffer_pool: Arc<BufferPool>,
        context: ProcessContext,
        shutdown: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            id,
            node,
            input_rx,
            output_tx,
            context_rx,
            buffer_pool,
            context,
            shutdown,
        }
    }

    /// Run the task continuously
    ///
    /// This is the main processing loop. It runs until shutdown signal.
    ///
    /// # Processing Steps
    /// 0. Receive updated ProcessContext for this block
    /// 1. Wait for all input buffers to arrive (non-blocking)
    /// 2. Acquire output buffer from pool
    /// 3. Process block (call AudioNode::process_block)
    /// 4. Send output to all downstream nodes
    /// 5. Return input buffers to pool (if no other refs)
    ///
    /// # Returns
    /// Ok(()) on graceful shutdown, Err on processing failure
    pub fn run(mut self) -> Result<(), String> {
        loop {
            // Check shutdown signal
            if self.shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                return Ok(());
            }

            // 0. Receive updated context (blocking until coordinator sends update)
            self.context = match self.context_rx.recv() {
                Ok(ctx) => ctx,
                Err(_) => {
                    // Context channel closed - graceful shutdown
                    return Ok(());
                }
            };

            // 1. Receive all input buffers (blocking until ready)
            let inputs = match self.receive_inputs() {
                Ok(inputs) => inputs,
                Err(e) => {
                    // Channel closed - graceful shutdown
                    if e.contains("disconnected") {
                        return Ok(());
                    }
                    return Err(e);
                }
            };

            // 2. Acquire output buffer from pool
            let mut output = self.buffer_pool.acquire();

            // 3. Process the block
            let input_slices: Vec<&[f32]> = inputs.iter().map(|buf| buf.as_slice()).collect();

            let sample_rate = self.context.sample_rate;
            self.node
                .process_block(&input_slices, &mut output, sample_rate, &self.context);

            // 4. Wrap in Arc and send to all downstream nodes
            let output_arc = Arc::new(output);
            for tx in &self.output_tx {
                // Send fails if channel closed (downstream shutdown)
                if tx.send(output_arc.clone()).is_err() {
                    return Ok(()); // Graceful shutdown
                }
            }

            // 5. Try to return input buffers to pool
            // Only succeeds if we have the last reference (no other nodes using it)
            for input in inputs {
                if let Ok(buf) = Arc::try_unwrap(input) {
                    self.buffer_pool.release(buf);
                }
            }
        }
    }

    /// Receive all input buffers
    ///
    /// Blocks until all dependencies have sent their outputs.
    /// This is the synchronization point in the dataflow.
    ///
    /// # Returns
    /// Vec of input buffers (one per dependency)
    fn receive_inputs(&self) -> Result<Vec<Arc<Vec<f32>>>, String> {
        let mut inputs = Vec::with_capacity(self.input_rx.len());

        for (i, rx) in self.input_rx.iter().enumerate() {
            match rx.recv() {
                Ok(buffer) => inputs.push(buffer),
                Err(_) => {
                    return Err(format!(
                        "Node {}: Input channel {} disconnected",
                        self.id, i
                    ));
                }
            }
        }

        Ok(inputs)
    }

    /// Get node ID
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Check if this is a source node (no inputs)
    pub fn is_source(&self) -> bool {
        self.input_rx.is_empty()
    }

    /// Check if this is a sink node (no outputs)
    pub fn is_sink(&self) -> bool {
        self.output_tx.is_empty()
    }
}

/// Source node task - generates audio without inputs
///
/// Source nodes (oscillators, noise generators) don't wait for inputs.
/// They process on a trigger signal (empty buffer).
pub struct SourceNodeTask {
    inner: NodeTask,
}

impl SourceNodeTask {
    /// Create a source node task
    ///
    /// Source nodes have a special input channel for trigger signals.
    pub fn new(
        id: NodeId,
        node: Box<dyn AudioNode>,
        trigger_rx: Receiver<Arc<Vec<f32>>>,
        output_tx: Vec<Sender<Arc<Vec<f32>>>>,
        context_rx: Receiver<ProcessContext>,
        buffer_pool: Arc<BufferPool>,
        context: ProcessContext,
        shutdown: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            inner: NodeTask::new(
                id,
                node,
                vec![trigger_rx], // Trigger as input
                output_tx,
                context_rx,
                buffer_pool,
                context,
                shutdown,
            ),
        }
    }

    /// Run the source node task
    ///
    /// Waits for trigger, then processes block.
    pub fn run(self) -> Result<(), String> {
        self.inner.run()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::ConstantNode;
    use crate::pattern::Fraction;
    use crossbeam::channel::bounded;

    #[test]
    fn test_node_task_creation() {
        let pool = Arc::new(BufferPool::new(512, 16));
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);
        let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let node = Box::new(ConstantNode::new(0.5));
        let (tx, _rx) = bounded(4);
        let (ctx_tx, ctx_rx) = bounded(4);

        let task = NodeTask::new(
            0,
            node,
            vec![], // No inputs (source node)
            vec![tx],
            ctx_rx,
            pool,
            context,
            shutdown,
        );

        assert_eq!(task.id(), 0);
        assert!(task.is_source());
        assert!(!task.is_sink());
    }

    #[test]
    fn test_node_task_single_block() {
        let pool = Arc::new(BufferPool::new(512, 16));
        pool.prefill(4);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);
        let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));

        // Create constant node (outputs 0.5)
        let node = Box::new(ConstantNode::new(0.5));

        // Setup channels
        let (trigger_tx, trigger_rx) = bounded(4);
        let (output_tx, output_rx) = bounded(4);
        let (ctx_tx, ctx_rx) = bounded(4);

        let task = NodeTask::new(
            0,
            node,
            vec![trigger_rx],
            vec![output_tx],
            ctx_rx,
            pool.clone(),
            context.clone(),
            shutdown.clone(),
        );

        // Run task in background
        std::thread::spawn(move || task.run());

        // Send context update first (NodeTask expects this before processing)
        ctx_tx.send(context).unwrap();

        // Send trigger
        let trigger = Arc::new(vec![0.0; 512]);
        trigger_tx.send(trigger).unwrap();

        // Receive output
        let output = output_rx.recv().unwrap();
        assert_eq!(output.len(), 512);
        assert!((output[0] - 0.5).abs() < 0.001);
        assert!((output[511] - 0.5).abs() < 0.001);

        // Shutdown
        shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    #[test]
    fn test_node_task_pipeline() {
        use crate::nodes::{OscillatorNode, Waveform};

        let pool = Arc::new(BufferPool::new(512, 16));
        pool.prefill(8);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);
        let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));

        // Node 0: Constant (440 Hz)
        let const_node = Box::new(ConstantNode::new(440.0));
        let (trigger_tx, trigger_rx) = bounded(4);
        let (const_out_tx, const_out_rx) = bounded(4);
        let (const_ctx_tx, const_ctx_rx) = bounded(4);

        let const_task = NodeTask::new(
            0,
            const_node,
            vec![trigger_rx],
            vec![const_out_tx],
            const_ctx_rx,
            pool.clone(),
            context.clone(),
            shutdown.clone(),
        );

        // Node 1: Oscillator (uses constant as frequency)
        let osc_node = Box::new(OscillatorNode::new(0, Waveform::Sine));
        let (osc_out_tx, osc_out_rx) = bounded(4);
        let (osc_ctx_tx, osc_ctx_rx) = bounded(4);

        let osc_task = NodeTask::new(
            1,
            osc_node,
            vec![const_out_rx],
            vec![osc_out_tx],
            osc_ctx_rx,
            pool.clone(),
            context.clone(),
            shutdown.clone(),
        );

        // Run tasks
        std::thread::spawn(move || const_task.run());
        std::thread::spawn(move || osc_task.run());

        // Send context updates to both nodes first
        const_ctx_tx.send(context.clone()).unwrap();
        osc_ctx_tx.send(context).unwrap();

        // Trigger processing
        let trigger = Arc::new(vec![0.0; 512]);
        trigger_tx.send(trigger).unwrap();

        // Receive final output
        let output = osc_out_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();

        // Should be a sine wave
        assert_eq!(output.len(), 512);
        // Check it's not all zeros (actual sine wave)
        let has_signal = output.iter().any(|&x| x.abs() > 0.1);
        assert!(has_signal, "Should produce sine wave output");

        // Shutdown
        shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
