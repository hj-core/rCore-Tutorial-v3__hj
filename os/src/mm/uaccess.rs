use core::arch::global_asm;

use riscv::regs::sstatus;

global_asm!(include_str!("uaccess.S"));

unsafe extern "C" {
    unsafe fn __uaccess(src: *const u8, dst: *mut u8, len: usize) -> usize;
    unsafe fn __uaccess_lb();
    unsafe fn __uaccess_sb();
    unsafe fn __uaccess_fix();
}

/// Copies `len` bytes from user space `src` to kernel
/// space `dst`, catching any page faults from user space.
///
/// Returns the number of bytes that failed to copy.
///
/// # Safety
///
/// `dst` must point to a valid, writable memory region
/// in kernel space with at least `len` bytes.
///
/// # Caveats
///
/// The user may supply a memory region that resides in
/// kernel space, and the kernel should check for that.
pub(crate) unsafe fn copy_from_user(src: *const u8, dst: *mut u8, len: usize) -> usize {
    sstatus::set_sum_permit();
    let result = unsafe { __uaccess(src, dst, len) };
    sstatus::set_sum_deny();
    result
}

/// Copies `len` bytes from kernel space `src` to user
/// space `dst`, catching any page faults from user space.
///
/// Returns the number of bytes that failed to copy.
///
/// # Safety
///
/// `src` must point to a valid, readable memory region
/// in kernel space with at least `len` bytes.
///
/// # Caveats
///
/// The user may supply a memory region that resides in
/// kernel space, and the kernel should check for that.
pub(crate) unsafe fn copy_to_user(src: *const u8, dst: *mut u8, len: usize) -> usize {
    sstatus::set_sum_permit();
    let result = unsafe { __uaccess(src, dst, len) };
    sstatus::set_sum_deny();
    result
}

pub(crate) fn is_load_user_fault(sepc: usize) -> bool {
    sepc == __uaccess_lb as usize
}

pub(crate) fn is_store_user_fault(sepc: usize) -> bool {
    sepc == __uaccess_sb as usize
}

/// Returns the `pc` to recover from a uaccess fault.
pub(crate) fn get_uaccess_fix() -> usize {
    __uaccess_fix as usize
}
