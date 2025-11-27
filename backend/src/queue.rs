use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::errors::LiquidationError;
use crate::models::{LiquidationCandidate, Position};

const COOLDOWN: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
struct QueueItem {
    candidate: LiquidationCandidate,
    last_attempt: Instant,
}

impl PartialEq for QueueItem {
    fn eq(&self, other: &Self) -> bool {
        self.candidate.position.id == other.candidate.position.id
    }
}
impl Eq for QueueItem {}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // lower margin ratio => higher priority
        other.candidate.margin_ratio.partial_cmp(&self.candidate.margin_ratio)
    }
}
impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

pub struct LiquidationQueue {
    heap: Mutex<BinaryHeap<QueueItem>>,
}

impl LiquidationQueue {
    pub fn new() -> Self {
        Self {
            heap: Mutex::new(BinaryHeap::new()),
        }
    }

    pub fn enqueue(
        &self,
        position: Position,
        mark_price: f64,
        margin_ratio: f64,
    ) -> Result<(), LiquidationError> {
        let mut heap = self.heap.lock().unwrap();
        heap.push(QueueItem {
            candidate: LiquidationCandidate {
                position,
                mark_price,
                margin_ratio,
            },
            last_attempt: Instant::now(),
        });
        Ok(())
    }

    pub fn pop(&self) -> Option<LiquidationCandidate> {
        let mut heap = self.heap.lock().unwrap();
        heap.pop().map(|item| item.candidate)
    }

    pub fn snapshot_ids(&self) -> Vec<String> {
        let heap = self.heap.lock().unwrap();
        heap.iter()
            .map(|item| item.candidate.position.id.clone())
            .collect()
    }
    pub fn snapshot(&self) -> Vec<LiquidationCandidate> {
        let heap = self.heap.lock().unwrap();
        heap.iter()
            .map(|item| item.candidate.clone())
            .collect()
    }

}
