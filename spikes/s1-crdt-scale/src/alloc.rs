//! A counting global allocator, so the spike can report real peak memory.
//!
//! The pass criterion for S1 is stated in bytes, and guessing at memory from snapshot sizes
//! would not answer it. Wrapping the system allocator is the only way to get the number without
//! a platform-specific profiler in the loop.
//!
//! This is the one place in the project that writes `unsafe`. It is confined to a spike, which
//! does not inherit the workspace's `unsafe_code = "forbid"`, and it will not graduate into a
//! shipped crate: production memory measurement belongs in the benchmark harness, not in the
//! product.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

/// Wraps the system allocator and tracks live and peak bytes.
pub struct Tracking;

unsafe impl GlobalAlloc for Tracking {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            record_growth(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let out = unsafe { System.realloc(ptr, layout, new_size) };
        if !out.is_null() {
            if new_size >= layout.size() {
                record_growth(new_size - layout.size());
            } else {
                LIVE.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        out
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            record_growth(layout.size());
        }
        ptr
    }
}

fn record_growth(bytes: usize) {
    let live = LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes;
    // Relaxed compare-exchange loop: an occasional lost update under contention would understate
    // the peak slightly, which is acceptable for a measurement that is reported to two
    // significant figures. The spike is single-threaded in any case.
    let mut peak = PEAK.load(Ordering::Relaxed);
    while live > peak {
        match PEAK.compare_exchange_weak(peak, live, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(observed) => peak = observed,
        }
    }
}

/// Bytes currently allocated.
pub fn live() -> usize {
    LIVE.load(Ordering::Relaxed)
}

/// Highest live figure since the last [`reset_peak`].
pub fn peak() -> usize {
    PEAK.load(Ordering::Relaxed)
}

/// Resets the peak to the current live figure, so the next phase is measured on its own.
pub fn reset_peak() {
    PEAK.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
}
