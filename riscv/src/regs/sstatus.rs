use crate::{csrr, csrrc, csrrs};

const CSR_NO: usize = 0x100;
const SIE_BIT: usize = 1 << 1;
const SPP_BIT: usize = 1 << 8;
const SUM_BIT: usize = 1 << 18;

pub fn read() -> usize {
    let mut result: usize;
    csrr!(CSR_NO, result);
    result
}

/// Sets the SPP bit to indicate a user-mode source and returns the new
/// value of the register.
pub fn set_spp_user() -> usize {
    let mut result: usize;
    csrrc!(CSR_NO, result, SPP_BIT);
    result & (!SPP_BIT)
}

/// Sets the SIE bit to globally enable all interrupts in supervisor mode,
/// and returns the old value of the register.
pub fn set_sie() -> usize {
    let mut result: usize;
    csrrs!(CSR_NO, result, SIE_BIT);
    result
}

/// Sets the SUM bit to permit S-mode memory accesses to page that
/// are accessible by U-mode.
pub fn set_sum_permit() -> usize {
    let mut result: usize;
    csrrs!(CSR_NO, result, SUM_BIT);
    result
}
