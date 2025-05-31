use core::arch::asm;

use crate::sbi;

const TIMEBASE_FREQUENCY: usize = 10_000_000; // Hz

/// `read_time_ms` returns the time since system start in millisecond.
pub(super) fn read_time_ms() -> usize {
    read_time() / (TIMEBASE_FREQUENCY / 1_000)
}

/// `read_time` returns the current value of the time counter.
pub(super) fn read_time() -> usize {
    let mut result: usize;
    unsafe { asm!("rdtime {}", lateout(reg) result) };
    result
}

/// `set_next_timer_interrupt` sets a timer interrupt 10 ms later.
///
/// The duration until the first timer interrupt should be long enough
/// to avoid triggering a trap before the sscratch has been initialized.
pub(super) fn set_next_timer_interrupt() {
    let time = TIMEBASE_FREQUENCY / 100 + read_time();
    sbi::set_mtimecmp(time);
}
