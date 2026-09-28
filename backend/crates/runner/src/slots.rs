//! Execution slots: the runner's concurrency limit and its uid pool in one.
//!
//! A slot is both "permission to run a job now" and "the identity that job
//! runs as". Tying the two together is what makes the uid pool safe without
//! any bookkeeping: at most one live job holds a given slot, so at most one
//! live job runs as a given uid, so "every process owned by this uid" is
//! exactly "every process this job started" — which is what teardown kills.
//!
//! Acquisition never waits. A full runner answers `busy` at once and lets the
//! API retry elsewhere, instead of queueing unboundedly behind slow jobs.

use std::sync::{Arc, Mutex, PoisonError};

/// First job uid/gid. Far above distribution-allocated system and human
/// accounts (which stop below 1000 and 60000 respectively on Debian, and the
/// image creates none in this range), and a uid needs no `/etc/passwd` entry
/// to be switched to.
pub const UID_BASE: u32 = 20_000;

/// Upper bound on slots, so `UID_BASE + slot` stays below 65534 (`nobody`)
/// and inside the 16-bit range some tools still assume.
pub const MAX_SLOTS: usize = 4096;

/// The fixed-size pool of slots.
#[derive(Clone, Debug)]
pub struct SlotPool {
    free: Arc<Mutex<Vec<usize>>>,
    capacity: usize,
    /// Whether jobs switch to a per-slot uid (the runner is root).
    switch_uid: bool,
}

/// A held slot. Returned to the pool on drop — including on panic and on a
/// cancelled request future — so a slot can never leak.
#[derive(Debug)]
pub struct SlotGuard {
    index: usize,
    creds: Option<Credentials>,
    free: Arc<Mutex<Vec<usize>>>,
}

/// The unprivileged identity a job's processes run as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Credentials {
    pub uid: u32,
    pub gid: u32,
}

impl SlotPool {
    pub fn new(capacity: usize, switch_uid: bool) -> Self {
        let capacity = capacity.clamp(1, MAX_SLOTS);
        // Reversed so `pop` hands out slot 0 first: deterministic uids make
        // logs and tests easier to read.
        let free = (0..capacity).rev().collect();
        Self {
            free: Arc::new(Mutex::new(free)),
            capacity,
            switch_uid,
        }
    }

    /// Take a free slot, or `None` when every slot is busy.
    pub fn try_acquire(&self) -> Option<SlotGuard> {
        let index = self
            .free
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop()?;
        let creds = self.switch_uid.then(|| {
            let id = UID_BASE + index as u32;
            Credentials { uid: id, gid: id }
        });
        Some(SlotGuard {
            index,
            creds,
            free: Arc::clone(&self.free),
        })
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn in_flight(&self) -> usize {
        let free = self
            .free
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len();
        self.capacity - free
    }
}

impl SlotGuard {
    pub fn index(&self) -> usize {
        self.index
    }

    /// The uid/gid to drop to, or `None` when the runner cannot switch users
    /// (not root — e.g. Lambda) and jobs run as the runner's own uid.
    pub fn credentials(&self) -> Option<Credentials> {
        self.creds
    }
}

impl Drop for SlotGuard {
    fn drop(&mut self) {
        // A poisoned lock only means another thread panicked mid-push; the
        // Vec itself is still valid, and losing a slot would shrink capacity
        // forever.
        self.free
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(self.index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_pool_refuses_instead_of_waiting() {
        let pool = SlotPool::new(2, false);
        let a = pool.try_acquire().expect("first slot");
        let b = pool.try_acquire().expect("second slot");
        assert!(pool.try_acquire().is_none());
        assert_eq!(pool.in_flight(), 2);
        drop(a);
        assert_eq!(pool.in_flight(), 1);
        let c = pool.try_acquire().expect("a released slot is reusable");
        assert_ne!(c.index(), b.index());
    }

    #[test]
    fn concurrent_jobs_never_share_a_uid() {
        let pool = SlotPool::new(8, true);
        let held: Vec<_> = (0..8).map(|_| pool.try_acquire().unwrap()).collect();
        let mut uids: Vec<u32> = held.iter().map(|g| g.credentials().unwrap().uid).collect();
        uids.sort_unstable();
        uids.dedup();
        assert_eq!(uids.len(), 8);
        assert!(uids.iter().all(|u| (UID_BASE..65_534).contains(u)));
        let c = held[0].credentials().unwrap();
        assert_eq!(c.uid, c.gid, "each job gets its own group too");
    }

    #[test]
    fn without_root_there_is_nothing_to_switch_to() {
        let pool = SlotPool::new(1, false);
        assert!(pool.try_acquire().unwrap().credentials().is_none());
    }

    #[test]
    fn capacity_is_bounded() {
        assert_eq!(SlotPool::new(0, false).capacity(), 1);
        assert_eq!(SlotPool::new(usize::MAX, false).capacity(), MAX_SLOTS);
        assert!(UID_BASE + (MAX_SLOTS as u32) < 65_534);
    }
}
