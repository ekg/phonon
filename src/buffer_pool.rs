/// Lock-free buffer pool for efficient memory reuse in dataflow architecture
///
/// This module provides a thread-safe buffer pool that minimizes allocations
/// during audio processing by recycling buffers between nodes.
///
/// # Design
/// - Lock-free ArrayQueue for thread-safe access
/// - Pre-allocated buffers to avoid runtime allocation
/// - Automatic size enforcement (all buffers same size)
/// - Graceful degradation (allocates new if pool empty)
use crossbeam_queue::ArrayQueue;
use std::sync::Arc;

/// Lock-free buffer pool for recycling Vec<f32> allocations
///
/// # Example
/// ```ignore
/// let pool = BufferPool::new(512, 64);  // 64 buffers of 512 samples each
///
/// // Acquire buffer (from pool or allocate new)
/// let mut buffer = pool.acquire();
///
/// // Use buffer...
/// buffer.fill(0.5);
///
/// // Release back to pool
/// pool.release(buffer);
/// ```
pub struct BufferPool {
    /// Lock-free queue of available buffers
    free_buffers: Arc<ArrayQueue<Vec<f32>>>,

    /// Size of each buffer (e.g., 512 samples)
    buffer_size: usize,

    /// Maximum number of buffers in pool
    max_buffers: usize,

    /// Statistics (atomic counters)
    stats: BufferPoolStats,
}

/// Statistics for buffer pool performance monitoring
struct BufferPoolStats {
    /// Total allocations (pool miss)
    allocations: std::sync::atomic::AtomicUsize,

    /// Total reuses (pool hit)
    reuses: std::sync::atomic::AtomicUsize,

    /// Current pool size
    current_size: std::sync::atomic::AtomicUsize,
}

impl BufferPool {
    /// Create a new buffer pool
    ///
    /// # Arguments
    /// * `buffer_size` - Size of each buffer in samples (e.g., 512)
    /// * `max_buffers` - Maximum number of buffers to pool (e.g., 64)
    ///
    /// # Example
    /// ```ignore
    /// // Pool for 512-sample blocks, max 64 buffers
    /// let pool = BufferPool::new(512, 64);
    /// ```
    pub fn new(buffer_size: usize, max_buffers: usize) -> Self {
        Self {
            free_buffers: Arc::new(ArrayQueue::new(max_buffers)),
            buffer_size,
            max_buffers,
            stats: BufferPoolStats {
                allocations: std::sync::atomic::AtomicUsize::new(0),
                reuses: std::sync::atomic::AtomicUsize::new(0),
                current_size: std::sync::atomic::AtomicUsize::new(0),
            },
        }
    }

    /// Pre-fill the pool with buffers
    ///
    /// This allocates buffers upfront to avoid allocation during audio processing.
    ///
    /// # Arguments
    /// * `count` - Number of buffers to pre-allocate (up to max_buffers)
    pub fn prefill(&self, count: usize) {
        let count = count.min(self.max_buffers);
        for _ in 0..count {
            let buffer = vec![0.0; self.buffer_size];
            let _ = self.free_buffers.push(buffer);
        }
        self.stats
            .current_size
            .store(count, std::sync::atomic::Ordering::Relaxed);
    }

    /// Acquire a buffer from the pool
    ///
    /// Returns a buffer from the pool if available, otherwise allocates new.
    /// The returned buffer is guaranteed to be of size `buffer_size`.
    ///
    /// # Returns
    /// Vec<f32> with length equal to buffer_size, zeroed
    pub fn acquire(&self) -> Vec<f32> {
        match self.free_buffers.pop() {
            Some(mut buffer) => {
                // Got buffer from pool - clear and return
                buffer.fill(0.0);
                self.stats
                    .reuses
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.stats
                    .current_size
                    .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                buffer
            }
            None => {
                // Pool empty - allocate new
                self.stats
                    .allocations
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                vec![0.0; self.buffer_size]
            }
        }
    }

    /// Release a buffer back to the pool
    ///
    /// If the pool is full, the buffer is dropped. Buffer size is enforced.
    ///
    /// # Arguments
    /// * `buffer` - Buffer to return (must be size buffer_size)
    pub fn release(&self, mut buffer: Vec<f32>) {
        // Enforce buffer size
        if buffer.len() != self.buffer_size {
            buffer.resize(self.buffer_size, 0.0);
        }

        // Clear buffer
        buffer.fill(0.0);

        // Try to return to pool (ignore if full)
        if self.free_buffers.push(buffer).is_ok() {
            self.stats
                .current_size
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Get pool statistics for monitoring
    ///
    /// Returns (allocations, reuses, current_pool_size, max_pool_size)
    pub fn stats(&self) -> (usize, usize, usize, usize) {
        (
            self.stats
                .allocations
                .load(std::sync::atomic::Ordering::Relaxed),
            self.stats.reuses.load(std::sync::atomic::Ordering::Relaxed),
            self.stats
                .current_size
                .load(std::sync::atomic::Ordering::Relaxed),
            self.max_buffers,
        )
    }

    /// Calculate pool efficiency (0.0 to 1.0)
    ///
    /// Returns the ratio of reuses to total acquisitions.
    /// Higher is better (fewer allocations).
    pub fn efficiency(&self) -> f64 {
        let allocs = self
            .stats
            .allocations
            .load(std::sync::atomic::Ordering::Relaxed);
        let reuses = self.stats.reuses.load(std::sync::atomic::Ordering::Relaxed);
        let total = allocs + reuses;

        if total == 0 {
            0.0
        } else {
            reuses as f64 / total as f64
        }
    }

    /// Get buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// Get maximum pool size
    pub fn max_buffers(&self) -> usize {
        self.max_buffers
    }
}

impl Clone for BufferPool {
    fn clone(&self) -> Self {
        Self {
            free_buffers: self.free_buffers.clone(),
            buffer_size: self.buffer_size,
            max_buffers: self.max_buffers,
            stats: BufferPoolStats {
                allocations: std::sync::atomic::AtomicUsize::new(0),
                reuses: std::sync::atomic::AtomicUsize::new(0),
                current_size: std::sync::atomic::AtomicUsize::new(
                    self.stats
                        .current_size
                        .load(std::sync::atomic::Ordering::Relaxed),
                ),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pool_basic() {
        let pool = BufferPool::new(512, 16);

        // Acquire buffer
        let buffer = pool.acquire();
        assert_eq!(buffer.len(), 512);
        assert!(buffer.iter().all(|&x| x == 0.0));

        // Should allocate (pool empty)
        let (allocs, reuses, _, _) = pool.stats();
        assert_eq!(allocs, 1);
        assert_eq!(reuses, 0);
    }

    #[test]
    fn test_buffer_pool_reuse() {
        let pool = BufferPool::new(512, 16);

        // Acquire and release
        let buffer = pool.acquire();
        pool.release(buffer);

        // Acquire again - should reuse
        let _buffer2 = pool.acquire();

        let (allocs, reuses, _, _) = pool.stats();
        assert_eq!(allocs, 1); // First acquire
        assert_eq!(reuses, 1); // Second acquire (reused)
    }

    #[test]
    fn test_buffer_pool_prefill() {
        let pool = BufferPool::new(512, 16);
        pool.prefill(8);

        let (_, _, current, _) = pool.stats();
        assert_eq!(current, 8);

        // All acquisitions should be reuses
        for _ in 0..8 {
            let _buffer = pool.acquire();
        }

        let (allocs, reuses, _, _) = pool.stats();
        assert_eq!(allocs, 0);
        assert_eq!(reuses, 8);
    }

    #[test]
    fn test_buffer_pool_efficiency() {
        let pool = BufferPool::new(512, 16);
        pool.prefill(4);

        // 4 reuses, 2 allocations
        for _ in 0..4 {
            let _buffer = pool.acquire();
        }
        for _ in 0..2 {
            let _buffer = pool.acquire();
        }

        let efficiency = pool.efficiency();
        assert!((efficiency - 0.666).abs() < 0.01); // 4/6 ≈ 0.666
    }

    #[test]
    fn test_buffer_pool_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let pool = Arc::new(BufferPool::new(512, 64));
        pool.prefill(32);

        let mut handles = vec![];

        // Spawn 8 threads, each acquiring/releasing 100 times
        for _ in 0..8 {
            let pool_clone = pool.clone();
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    let buffer = pool_clone.acquire();
                    assert_eq!(buffer.len(), 512);
                    pool_clone.release(buffer);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // All buffers should be reused (prefilled enough)
        let (allocs, _, _, _) = pool.stats();
        assert_eq!(allocs, 0, "Should reuse all buffers from prefill");
    }
}
