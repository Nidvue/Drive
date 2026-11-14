// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Rate limiting, retry policy, adaptive concurrency and request coalescing.
//!
//! The rule that matters: never let a retry storm amplify load. A 429 halves
//! concurrency, jitter spreads the retries, and identical in-flight requests
//! share one network call.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rand::Rng;
use tokio::sync::Mutex as AsyncMutex;

/// A token bucket: `rate` tokens per second, up to `burst` at once.
#[derive(Debug)]
pub struct TokenBucket {
    rate: f64,
    burst: f64,
    state: Mutex<BucketState>,
}

#[derive(Debug)]
struct BucketState {
    tokens: f64,
    last: Instant,
}

impl TokenBucket {
    /// Build a bucket that refills `rate` tokens per second and holds `burst`.
    pub fn new(rate: f64, burst: u32) -> Self {
        let burst = burst.max(1) as f64;
        Self {
            rate: rate.max(0.001),
            burst,
            state: Mutex::new(BucketState {
                tokens: burst,
                last: Instant::now(),
            }),
        }
    }

    /// Try to take one token. Returns how long to wait when there is none.
    pub fn try_take(&self) -> std::result::Result<(), Duration> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(state.last).as_secs_f64();
        state.last = now;
        state.tokens = (state.tokens + elapsed * self.rate).min(self.burst);
        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            Ok(())
        } else {
            let missing = 1.0 - state.tokens;
            Err(Duration::from_secs_f64(missing / self.rate))
        }
    }

    /// Wait until a token is available, then take it.
    pub async fn acquire(&self) {
        loop {
            match self.try_take() {
                Ok(()) => return,
                Err(wait) => tokio::time::sleep(wait).await,
            }
        }
    }

    /// Tokens currently available (for tests and diagnostics).
    pub fn available(&self) -> f64 {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.tokens
    }
}

/// Exponential backoff with full jitter and an optional server hint.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// First delay.
    pub base: Duration,
    /// Delay ceiling.
    pub max: Duration,
    /// Maximum attempts, including the first.
    pub attempts: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            base: Duration::from_millis(500),
            max: Duration::from_secs(30),
            attempts: 6,
        }
    }
}

impl RetryPolicy {
    /// The delay before attempt `n` (1-based). Full jitter: `rand(0, base*2^n)`.
    pub fn delay(&self, attempt: u32, retry_after: Option<Duration>) -> Duration {
        if let Some(hint) = retry_after {
            return hint.min(self.max);
        }
        let exp = 2u32.saturating_pow(attempt.saturating_sub(1).min(20));
        let ceiling = self.base.saturating_mul(exp).min(self.max);
        let millis = ceiling.as_millis() as u64;
        if millis == 0 {
            return Duration::ZERO;
        }
        let jittered = rand::rng().random_range(0..=millis);
        Duration::from_millis(jittered)
    }

    /// True when another attempt is allowed.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.attempts
    }
}

/// Additive-increase / multiplicative-decrease concurrency control.
#[derive(Debug)]
pub struct Aimd {
    current: AtomicU32,
    min: u32,
    max: u32,
}

impl Aimd {
    /// Start at `min`, bounded by `max`.
    pub fn new(min: u32, max: u32) -> Self {
        let min = min.max(1);
        let max = max.max(min);
        Self {
            current: AtomicU32::new(min),
            min,
            max,
        }
    }

    /// The current concurrency limit.
    pub fn current(&self) -> u32 {
        self.current.load(Ordering::Relaxed)
    }

    /// A request succeeded: grow by one, up to the maximum.
    pub fn on_success(&self) {
        let _ = self
            .current
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |c| {
                Some((c + 1).min(self.max))
            });
    }

    /// A request was rate limited or failed: halve, down to the minimum.
    pub fn on_congestion(&self) {
        let _ = self
            .current
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |c| {
                Some((c / 2).max(self.min))
            });
    }

    /// The floor.
    pub fn min(&self) -> u32 {
        self.min
    }

    /// The ceiling.
    pub fn max(&self) -> u32 {
        self.max
    }
}

/// Coalesces concurrent calls for the same key and caches the result briefly.
///
/// Two `getattr`s for one path in flight at the same time produce one request.
/// The cache TTL is deliberately short: this is coalescing, not a metadata
/// cache (the index owns that).
pub struct Coalescer<K, V> {
    inflight: Mutex<HashMap<K, Arc<AsyncMutex<()>>>>,
    results: Mutex<HashMap<K, (Instant, V)>>,
    ttl: Duration,
}

impl<K, V> Coalescer<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Build a coalescer with a result TTL.
    pub fn new(ttl: Duration) -> Self {
        Self {
            inflight: Mutex::new(HashMap::new()),
            results: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    /// Run `work` once per key: concurrent callers wait for the first result.
    pub async fn run<F, Fut, E>(&self, key: K, work: F) -> std::result::Result<V, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<V, E>>,
    {
        {
            let cache = self.results.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((at, value)) = cache.get(&key) {
                if at.elapsed() < self.ttl {
                    return Ok(value.clone());
                }
            }
        }
        let lock = {
            let mut map = self.inflight.lock().unwrap_or_else(|e| e.into_inner());
            map.entry(key.clone())
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };
        let _guard = lock.lock().await;
        {
            let cache = self.results.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((at, value)) = cache.get(&key) {
                if at.elapsed() < self.ttl {
                    return Ok(value.clone());
                }
            }
        }
        let result = work().await;
        let mut map = self.inflight.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(&key);
        drop(map);
        if let Ok(value) = &result {
            let mut cache = self.results.lock().unwrap_or_else(|e| e.into_inner());
            cache.retain(|_, (at, _)| at.elapsed() < self.ttl);
            cache.insert(key, (Instant::now(), value.clone()));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn bucket_refills_over_time() {
        let bucket = TokenBucket::new(100.0, 2);
        assert!(bucket.try_take().is_ok());
        assert!(bucket.try_take().is_ok());
        let wait = bucket.try_take().unwrap_err();
        assert!(wait <= Duration::from_millis(20));
    }

    #[tokio::test]
    async fn acquire_waits_instead_of_spinning() {
        let bucket = Arc::new(TokenBucket::new(1000.0, 1));
        bucket.acquire().await;
        let start = Instant::now();
        bucket.acquire().await;
        assert!(start.elapsed() >= Duration::from_micros(500));
    }

    #[test]
    fn backoff_grows_and_respects_hints() {
        let p = RetryPolicy {
            base: Duration::from_millis(100),
            max: Duration::from_secs(10),
            attempts: 5,
        };
        let d1 = p.delay(1, None);
        let d5 = p.delay(5, None);
        assert!(d1 <= Duration::from_millis(100));
        assert!(d5 <= Duration::from_secs(10));
        assert_eq!(
            p.delay(3, Some(Duration::from_secs(2))),
            Duration::from_secs(2)
        );
        assert_eq!(
            p.delay(3, Some(Duration::from_secs(60))),
            Duration::from_secs(10)
        );
        assert!(p.should_retry(4));
        assert!(!p.should_retry(5));
    }

    #[test]
    fn aimd_halves_and_grows() {
        let a = Aimd::new(1, 8);
        for _ in 0..20 {
            a.on_success();
        }
        assert_eq!(a.current(), 8);
        a.on_congestion();
        assert_eq!(a.current(), 4);
        for _ in 0..10 {
            a.on_congestion();
        }
        assert_eq!(a.current(), 1);
    }

    #[tokio::test]
    async fn coalesces_concurrent_calls() {
        let c = Arc::new(Coalescer::<String, u32>::new(Duration::from_millis(50)));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let c = c.clone();
            let calls = calls.clone();
            tasks.push(tokio::spawn(async move {
                c.run("k".to_string(), || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        Ok::<u32, ()>(7)
                    }
                })
                .await
                .unwrap()
            }));
        }
        for t in tasks {
            assert_eq!(t.await.unwrap(), 7);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn does_not_cache_errors() {
        let c = Coalescer::<String, u32>::new(Duration::from_secs(1));
        let first: std::result::Result<u32, ()> = c.run("k".into(), || async { Err(()) }).await;
        assert!(first.is_err());
        let second: std::result::Result<u32, ()> = c.run("k".into(), || async { Ok(1) }).await;
        assert_eq!(second.unwrap(), 1);
    }
}
