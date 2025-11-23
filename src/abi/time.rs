use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

#[repr(C)]
pub struct AtomicDuration(AtomicU64);

#[repr(C)]
pub struct AtomicInstant(AtomicDuration, Instant);

impl AtomicDuration {
    #[inline]
    pub fn new(dur: Duration) -> Self {
        let micros = dur.as_micros();

        if micros > u64::MAX as u128 {
            panic!("duration overflowed");
        }

        Self(AtomicU64::new(micros as u64))
    }

    #[inline]
    pub fn load(&self, order: Ordering) -> Duration {
        Duration::from_micros(self.0.load(order))
    }

    #[inline]
    pub fn store(&self, dur: Duration, order: Ordering) {
        let micros = dur.as_micros();

        if micros > u64::MAX as u128 {
            panic!("duration overflowed");
        }

        self.0.store(micros as u64, order);
    }
}

impl AtomicInstant {
    #[inline]
    pub fn now() -> Self {
        let now = Instant::now();
        let dur = now.duration_since(now);
        Self(AtomicDuration::new(dur), now)
    }

    #[inline]
    pub fn load(&self, order: Ordering) -> Instant {
        self.1 + self.0.load(order)
    }

    #[inline]
    pub fn store(&self, ins: Instant, order: Ordering) {
        let dur = ins.duration_since(self.1);
        self.0.store(dur, order)
    }
}
