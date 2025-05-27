use core::arch::asm;

const TIMEBASE_FREQUENCY: usize = 10_000_000; // Hz

/// `read_time_ms` returns the time since system start in millisecond.
pub(super) fn read_time_ms() -> usize {
    read_time() / (TIMEBASE_FREQUENCY / 1_000)
}

/// `read_time` returns the current value of the time counter.
fn read_time() -> usize {
    let mut result: usize;
    unsafe { asm!("rdtime {}", lateout(reg) result) };
    result
}
