//! Rate limiting that holds across every API task.
//!
//! A per-process counter is worthless behind a load balancer — N tasks give an
//! attacker N times the budget — so limits live in Redis: one fixed-window
//! counter per (key, window), `INCR` + `EXPIRE` in one round trip. Without
//! Redis (a laptop, a single task) an in-memory table gives the same
//! behaviour for one process.
//!
//! A Redis outage fails *open*: blocking every login because the rate-limit
//! store is down would turn a cache incident into a full outage, and the
//! per-account lockout in Postgres still brakes password guessing.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    pub allowed: bool,
    pub remaining: u32,
    /// Seconds until the window resets.
    pub retry_after: u64,
}

#[async_trait]
pub trait RateLimiter: Send + Sync {
    /// Count one event against `key`, allowing at most `limit` per `window`.
    async fn hit(&self, key: &str, limit: u32, window: Duration) -> Decision;
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn decide(count: u64, limit: u32, window_start: u64, window: u64, now: u64) -> Decision {
    Decision {
        allowed: count <= limit as u64,
        remaining: (limit as u64).saturating_sub(count) as u32,
        retry_after: (window_start + window).saturating_sub(now).max(1),
    }
}

// ─────────────────────────────────────────────────────────────────────────────

pub struct RedisLimiter {
    conn: redis::aio::ConnectionManager,
}

impl RedisLimiter {
    pub fn new(conn: redis::aio::ConnectionManager) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl RateLimiter for RedisLimiter {
    async fn hit(&self, key: &str, limit: u32, window: Duration) -> Decision {
        let w = window.as_secs().max(1);
        let now = now_secs();
        let start = now - now % w;
        let k = format!("rl:{key}:{start}");
        let mut conn = self.conn.clone();
        let res: redis::RedisResult<(u64,)> = redis::pipe()
            .atomic()
            .incr(&k, 1u64)
            .expire(&k, (w + 5) as i64)
            .ignore()
            .query_async(&mut conn)
            .await;
        match res {
            Ok((count,)) => decide(count, limit, start, w, now),
            Err(e) => {
                tracing::warn!(error = %e, "rate limiter unavailable; allowing request");
                Decision {
                    allowed: true,
                    remaining: limit,
                    retry_after: 0,
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct MemoryLimiter {
    windows: Mutex<HashMap<String, (u64, u64)>>,
}

#[async_trait]
impl RateLimiter for MemoryLimiter {
    async fn hit(&self, key: &str, limit: u32, window: Duration) -> Decision {
        let w = window.as_secs().max(1);
        let now = now_secs();
        let start = now - now % w;
        let mut map = self.windows.lock().expect("limiter lock");
        // Bounded: drop finished windows once the table grows, so a flood of
        // distinct keys (one per IP) cannot grow memory without limit.
        if map.len() > 50_000 {
            map.retain(|_, (s, _)| *s + 3600 > now);
        }
        let entry = map.entry(key.to_string()).or_insert((start, 0));
        if entry.0 != start {
            *entry = (start, 0);
        }
        entry.1 += 1;
        decide(entry.1, limit, start, w, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_memory_limiter_allows_exactly_the_limit() {
        let l = MemoryLimiter::default();
        let w = Duration::from_secs(3600);
        for i in 0..5 {
            let d = l.hit("login:1.2.3.4", 5, w).await;
            assert!(d.allowed, "hit {i}");
            assert_eq!(d.remaining, 4 - i);
        }
        let d = l.hit("login:1.2.3.4", 5, w).await;
        assert!(!d.allowed);
        assert!(d.retry_after >= 1);
        assert!(
            l.hit("login:5.6.7.8", 5, w).await.allowed,
            "keys are independent"
        );
    }

    #[test]
    fn retry_after_counts_to_the_window_edge() {
        let d = decide(11, 10, 1_000, 60, 1_030);
        assert!(!d.allowed);
        assert_eq!(d.retry_after, 30);
        assert_eq!(d.remaining, 0);
    }
}
