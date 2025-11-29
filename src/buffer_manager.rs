/// Buffer management for zero-copy audio processing
///
/// This module manages audio buffer allocation and reuse, enabling efficient
/// block-based processing without repeated allocations.
use std::sync::Arc;

/// Manages audio buffers for zero-copy sharing between nodes
///
/// # Design Goals
/// - **Reuse buffers**: Avoid repeated allocations (expensive in audio thread)
/// - **Zero-copy sharing**: Use Arc to share buffers between nodes
/// - **Fixed size**: All buffers have same size (simplifies management)
///
/// # Usage Pattern
/// ```ignore
/// let mut manager = BufferManager::new(10, 512);
///
/// // Get buffer from pool
/// let mut buffer = manager.get_buffer();
///
/// // Process audio into buffer
/// process_audio(&mut buffer);
///
/// // Share buffer with other nodes (zero-copy via Arc)
/// let shared = Arc::new(buffer);
///
/// // Later, try to get buffer back for reuse
/// if let Ok(buffer) = Arc::try_unwrap(shared) {
///     manager.return_buffer(buffer);
/// }
/// ```
pub struct BufferManager {
    /// Pre-allocated buffer pool (LIFO for cache locality)
    pool: Vec<Vec<f32>>,

    /// Maximum number of buffers to keep in pool
    pool_size: usize,

    /// Size of each buffer (usually 512 samples)
    buffer_size: usize,

    /// Statistics for debugging/optimization
    stats: BufferStats,
}

/// Statistics for buffer usage (useful for optimization)
#[derive(Debug, Clone, Default)]
pub struct BufferStats {
    /// Number of times we had to allocate (pool was empty)
    pub allocations: usize,

    /// Number of times we reused from pool
    pub reuses: usize,

    /// Number of times buffer was returned to pool
    pub returns: usize,

    /// Number of times buffer was dropped (pool full)
    pub drops: usize,
}

impl BufferManager {
    /// Create a new buffer manager
    ///
    /// # Arguments
    /// * `pool_size` - Maximum number of buffers to keep in pool (e.g., 20)
    /// * `buffer_size` - Size of each buffer in samples (usually 512)
    pub fn new(pool_size: usize, buffer_size: usize) -> Self {
        // Pre-allocate the pool with buffers
        let mut pool = Vec::with_capacity(pool_size);
        for _ in 0..pool_size {
            pool.push(vec![0.0; buffer_size]);
        }

        Self {
            pool,
            pool_size,
            buffer_size,
            stats: BufferStats::default(),
        }
    }

    /// Get a zeroed buffer (from pool or newly allocated)
    ///
    /// Attempts to reuse a buffer from the pool. If pool is empty,
    /// allocates a new buffer (tracked in stats).
    pub fn get_buffer(&mut self) -> Vec<f32> {
        if let Some(buffer) = self.pool.pop() {
            self.stats.reuses += 1;
            buffer
        } else {
            self.stats.allocations += 1;
            vec![0.0; self.buffer_size]
        }
    }

    /// Return a buffer to the pool for reuse
    ///
    /// The buffer is cleared (zeroed) and returned to the pool if there's space.
    /// If the pool is full, the buffer is dropped (deallocated).
    ///
    /// # Arguments
    /// * `buffer` - Buffer to return (must be buffer_size length)
    pub fn return_buffer(&mut self, mut buffer: Vec<f32>) {
        debug_assert_eq!(
            buffer.len(),
            self.buffer_size,
            "Returned buffer has wrong size"
        );

        if self.pool.len() < self.pool_size {
            // Clear buffer for next use
            buffer.iter_mut().for_each(|x| *x = 0.0);
            self.pool.push(buffer);
            self.stats.returns += 1;
        } else {
            // Pool full, drop the buffer
            self.stats.drops += 1;
        }
    }

    /// Get current buffer statistics
    pub fn stats(&self) -> &BufferStats {
        &self.stats
    }

    /// Reset statistics (useful for benchmarking)
    pub fn reset_stats(&mut self) {
        self.stats = BufferStats::default();
    }

    /// Get current number of buffers in pool
    pub fn pool_len(&self) -> usize {
        self.pool.len()
    }

    /// Check if pool is empty
    pub fn pool_empty(&self) -> bool {
        self.pool.is_empty()
    }

    /// Get buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
}

/// Node output storage - shared across dependent nodes
///
/// This represents the output of a node that can be shared (zero-copy)
/// with multiple dependent nodes via Arc.
#[derive(Clone)]
pub struct NodeOutput {
    /// The audio buffer (shared via Arc for zero-copy)
    pub buffer: Arc<Vec<f32>>,

    /// Whether this output is ready to be read
    /// (used for parallel execution scheduling)
    pub ready: bool,
}

impl NodeOutput {
    /// Create a new node output from a buffer
    pub fn new(buffer: Vec<f32>) -> Self {
        Self {
            buffer: Arc::new(buffer),
            ready: true,
        }
    }

    /// Create a new node output from an Arc'd buffer
    pub fn from_arc(buffer: Arc<Vec<f32>>) -> Self {
        Self {
            buffer,
            ready: true,
        }
    }

    /// Create an empty/not-ready output
    pub fn empty(buffer_size: usize) -> Self {
        Self {
            buffer: Arc::new(vec![0.0; buffer_size]),
            ready: false,
        }
    }

    /// Try to unwrap the Arc to get the buffer back
    ///
    /// This succeeds if this is the only reference to the buffer,
    /// allowing it to be returned to the buffer pool.
    pub fn try_unwrap(self) -> Result<Vec<f32>, Arc<Vec<f32>>> {
        Arc::try_unwrap(self.buffer)
    }

    /// Get a reference to the buffer data
    pub fn as_slice(&self) -> &[f32] {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_manager_get_and_return() {
        let mut manager = BufferManager::new(5, 512);

        // Should have full pool initially
        assert_eq!(manager.pool_len(), 5);

        // Get buffer (should come from pool)
        let buffer = manager.get_buffer();
        assert_eq!(buffer.len(), 512);
        assert_eq!(manager.pool_len(), 4);
        assert_eq!(manager.stats().reuses, 1);

        // Return buffer
        manager.return_buffer(buffer);
        assert_eq!(manager.pool_len(), 5);
        assert_eq!(manager.stats().returns, 1);
    }

    #[test]
    fn test_buffer_manager_allocation_when_empty() {
        let mut manager = BufferManager::new(2, 512);

        // Drain the pool
        let b1 = manager.get_buffer();
        let b2 = manager.get_buffer();
        assert_eq!(manager.pool_len(), 0);

        // Next get should allocate
        let b3 = manager.get_buffer();
        assert_eq!(b3.len(), 512);
        assert_eq!(manager.stats().allocations, 1);

        // Return buffers
        manager.return_buffer(b1);
        manager.return_buffer(b2);
        manager.return_buffer(b3);

        // Pool should be back to full (2 buffers max)
        assert_eq!(manager.pool_len(), 2);
    }

    #[test]
    fn test_buffer_manager_drops_when_full() {
        let mut manager = BufferManager::new(2, 512);

        // Fill pool
        let b1 = manager.get_buffer();
        let b2 = manager.get_buffer();
        let b3 = manager.get_buffer();

        manager.return_buffer(b1);
        manager.return_buffer(b2);

        // Pool is full (2 max), next return should drop
        manager.return_buffer(b3);
        assert_eq!(manager.pool_len(), 2);
        assert_eq!(manager.stats().drops, 1);
    }

    #[test]
    fn test_buffer_cleared_on_return() {
        let mut manager = BufferManager::new(2, 4);

        let mut buffer = manager.get_buffer();
        buffer[0] = 1.0;
        buffer[1] = 2.0;
        buffer[2] = 3.0;
        buffer[3] = 4.0;

        manager.return_buffer(buffer);

        // Get it back
        let buffer = manager.get_buffer();
        assert_eq!(buffer[0], 0.0);
        assert_eq!(buffer[1], 0.0);
        assert_eq!(buffer[2], 0.0);
        assert_eq!(buffer[3], 0.0);
    }

    #[test]
    fn test_node_output_zero_copy_sharing() {
        let buffer = vec![1.0, 2.0, 3.0, 4.0];
        let output = NodeOutput::new(buffer);

        // Clone should share the Arc (zero-copy)
        let output2 = output.clone();
        assert_eq!(Arc::strong_count(&output.buffer), 2);

        // Both should see same data
        assert_eq!(output.as_slice()[0], 1.0);
        assert_eq!(output2.as_slice()[0], 1.0);

        // try_unwrap should fail (two references)
        let result = output.try_unwrap();
        assert!(result.is_err());

        // After dropping one reference, try_unwrap should succeed
        drop(result);
        let unwrapped = output2.try_unwrap();
        assert!(unwrapped.is_ok());
    }
}
