//! Full-server connection queueing.
//!
//! When [`limbo::LimboHandler::try_connect_backend`](crate::net::limbo)
//! resolves a candidate backend that's at capacity
//! ([`BackendServer::is_full`](crate::server::BackendServer::is_full)), the
//! player is enqueued here instead of being handed a connection — limbo
//! keeps them in its fake world (already retrying every few seconds) and
//! surfaces a live position message. Once the backend has a free slot, only
//! the player at the front of that server's queue is let through, so
//! multiple queued limbo pollers don't race for the same freed slot.
//!
//! Queues are per-backend-name and hold only a player's UUID — no other
//! state. A player can be queued for at most one server at a time in
//! practice (limbo only ever resolves one candidate per poll), but nothing
//! here enforces that beyond the caller's usage pattern.

use std::collections::VecDeque;

use dashmap::DashMap;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct QueueManager {
    queues: DashMap<String, Mutex<VecDeque<Uuid>>>,
    max_queue_size: usize,
}

impl QueueManager {
    pub fn new(max_queue_size: usize) -> Self {
        Self {
            queues: DashMap::new(),
            max_queue_size,
        }
    }

    /// Add `uuid` to the back of `server`'s queue if it isn't already
    /// queued there. Returns the 1-based position on success, or `Err` if
    /// the queue is at `max_queue_size` (and `uuid` isn't already in it).
    pub async fn enqueue(&self, server: &str, uuid: Uuid) -> Result<usize, &'static str> {
        let entry = self
            .queues
            .entry(server.to_string())
            .or_insert_with(|| Mutex::new(VecDeque::new()));
        let mut queue = entry.lock().await;

        if let Some(pos) = queue.iter().position(|u| *u == uuid) {
            return Ok(pos + 1);
        }
        if self.max_queue_size != 0 && queue.len() >= self.max_queue_size {
            return Err("queue is full");
        }
        queue.push_back(uuid);
        Ok(queue.len())
    }

    /// 1-based position of `uuid` in `server`'s queue, or `None` if it
    /// isn't queued there (including "no queue exists for that server yet").
    pub async fn position(&self, server: &str, uuid: Uuid) -> Option<usize> {
        let queue = self.queues.get(server)?;
        let queue = queue.lock().await;
        queue.iter().position(|u| *u == uuid).map(|i| i + 1)
    }

    /// True if `uuid` is at the front of `server`'s queue (or the queue is
    /// empty / doesn't exist — nothing to wait behind).
    pub async fn is_front_or_unqueued(&self, server: &str, uuid: Uuid) -> bool {
        match self.queues.get(server) {
            Some(queue) => {
                let queue = queue.lock().await;
                match queue.front() {
                    Some(front) => *front == uuid,
                    None => true,
                }
            },
            None => true,
        }
    }

    /// Remove `uuid` from `server`'s queue (called once it successfully
    /// connects, or on disconnect while still waiting).
    pub async fn remove(&self, server: &str, uuid: Uuid) {
        if let Some(queue) = self.queues.get(server) {
            queue.lock().await.retain(|u| *u != uuid);
        }
    }

    /// Remove `uuid` from every per-server queue. Used on a player's full
    /// disconnect, where the caller doesn't necessarily know (or the player
    /// may never have been told) which single server they were queued for.
    pub async fn remove_from_all(&self, uuid: Uuid) {
        for entry in self.queues.iter() {
            entry.value().lock().await.retain(|u| *u != uuid);
        }
    }

    /// Drop empty per-server queue entries so the outer map doesn't grow
    /// forever across server renames/removals. Call periodically.
    pub fn evict_empty(&self) {
        self.queues.retain(|_, queue| {
            queue
                .try_lock()
                .map(|q| !q.is_empty())
                // A queue that's momentarily locked is in active use — keep it.
                .unwrap_or(true)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn enqueue_assigns_increasing_positions() {
        let q = QueueManager::new(0);
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        assert_eq!(q.enqueue("survival", a).await, Ok(1));
        assert_eq!(q.enqueue("survival", b).await, Ok(2));
        assert_eq!(q.position("survival", a).await, Some(1));
        assert_eq!(q.position("survival", b).await, Some(2));
    }

    #[tokio::test]
    async fn enqueue_is_idempotent() {
        let q = QueueManager::new(0);
        let a = Uuid::new_v4();
        assert_eq!(q.enqueue("survival", a).await, Ok(1));
        assert_eq!(q.enqueue("survival", a).await, Ok(1));
    }

    #[tokio::test]
    async fn max_queue_size_rejects_once_full() {
        let q = QueueManager::new(1);
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        assert_eq!(q.enqueue("survival", a).await, Ok(1));
        assert!(q.enqueue("survival", b).await.is_err());
    }

    #[tokio::test]
    async fn only_front_is_reported_as_front() {
        let q = QueueManager::new(0);
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        q.enqueue("survival", a).await.unwrap();
        q.enqueue("survival", b).await.unwrap();
        assert!(q.is_front_or_unqueued("survival", a).await);
        assert!(!q.is_front_or_unqueued("survival", b).await);
    }

    #[tokio::test]
    async fn unqueued_player_counts_as_front() {
        let q = QueueManager::new(0);
        assert!(q.is_front_or_unqueued("survival", Uuid::new_v4()).await);
    }

    #[tokio::test]
    async fn remove_drops_from_queue_and_shifts_positions() {
        let q = QueueManager::new(0);
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        q.enqueue("survival", a).await.unwrap();
        q.enqueue("survival", b).await.unwrap();
        q.remove("survival", a).await;
        assert_eq!(q.position("survival", a).await, None);
        assert_eq!(q.position("survival", b).await, Some(1));
    }
}
