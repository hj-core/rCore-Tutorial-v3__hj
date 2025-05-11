use crate::{csrr, csrrc};

const CSR_NO: usize = 0x100;
const SPP_BIT: usize = 1 << 8;

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
