/// Asynchronous tile prefetching with background threads

use std::sync::Arc;
use std::thread;
use crossbeam::channel::{Sender, Receiver, bounded, unbounded};
use crate::formats::tiff::{IFD, IFDEntry};
use crate::error::Result;

/// Request to prefetch tiles
pub struct PrefetchRequest {
    pub tile_indices: Vec<usize>,
    pub ifd: Arc<IFD>,
    pub offsets_entry: IFDEntry,
    pub byte_counts_entry: IFDEntry,
}

/// Result from prefetching
pub struct PrefetchResult {
    pub tile_index: usize,
    pub tile_data: Vec<u8>,
}

/// Background prefetch worker
pub struct PrefetchWorker {
    request_tx: Sender<PrefetchRequest>,
    result_rx: Receiver<PrefetchResult>,
    _worker_thread: thread::JoinHandle<()>,
}

impl PrefetchWorker {
    /// Creates a new prefetch worker with background thread
    pub fn new<F>(load_tile_fn: F) -> Self
    where
        F: Fn(&IFD, &IFDEntry, &IFDEntry, usize) -> Result<Vec<u8>> + Send + 'static,
    {
        let (request_tx, request_rx) = unbounded::<PrefetchRequest>();
        let (result_tx, result_rx) = bounded::<PrefetchResult>(32);

        let worker_thread = thread::spawn(move || {
            while let Ok(request) = request_rx.recv() {
                for tile_idx in request.tile_indices {
                    if let Ok(tile_data) = load_tile_fn(
                        &request.ifd,
                        &request.offsets_entry,
                        &request.byte_counts_entry,
                        tile_idx,
                    ) {
                        let result = PrefetchResult {
                            tile_index: tile_idx,
                            tile_data,
                        };

                        if result_tx.send(result).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self {
            request_tx,
            result_rx,
            _worker_thread: worker_thread,
        }
    }

    /// Request tiles to be prefetched
    pub fn prefetch(&self, request: PrefetchRequest) {
        let _ = self.request_tx.send(request);
    }

    /// Try to get a prefetched tile (non-blocking)
    pub fn try_get_result(&self) -> Option<PrefetchResult> {
        self.result_rx.try_recv().ok()
    }

    /// Drain all pending results
    pub fn drain_results(&self) -> Vec<PrefetchResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.result_rx.try_recv() {
            results.push(result);
        }
        results
    }
}

/// Multi-threaded prefetch pool
pub struct PrefetchPool {
    workers: Vec<PrefetchWorker>,
    next_worker: std::sync::atomic::AtomicUsize,
}

impl PrefetchPool {
    /// Creates a pool with specified number of worker threads
    pub fn new<F>(num_workers: usize, load_tile_fn: F) -> Self
    where
        F: Fn(&IFD, &IFDEntry, &IFDEntry, usize) -> Result<Vec<u8>> + Send + Clone + 'static,
    {
        let workers = (0..num_workers)
            .map(|_| PrefetchWorker::new(load_tile_fn.clone()))
            .collect();

        Self {
            workers,
            next_worker: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Submit prefetch request to pool (round-robin)
    pub fn prefetch(&self, request: PrefetchRequest) {
        if self.workers.is_empty() {
            return;
        }

        let worker_idx = self.next_worker.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % self.workers.len();
        self.workers[worker_idx].prefetch(request);
    }

    /// Collect results from all workers
    pub fn collect_results(&self) -> Vec<PrefetchResult> {
        let mut all_results = Vec::new();
        for worker in &self.workers {
            all_results.extend(worker.drain_results());
        }
        all_results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_prefetch_worker() {
        let worker = PrefetchWorker::new(|_ifd, _offsets, _counts, tile_idx| {
            thread::sleep(Duration::from_millis(10));
            Ok(vec![tile_idx as u8; 100])
        });

        thread::sleep(Duration::from_millis(50));

        let results = worker.drain_results();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_prefetch_pool() {
        let pool = PrefetchPool::new(2, |_ifd, _offsets, _counts, tile_idx| {
            Ok(vec![tile_idx as u8; 100])
        });

        thread::sleep(Duration::from_millis(50));

        let results = pool.collect_results();
        assert_eq!(results.len(), 0);
    }
}
