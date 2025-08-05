use crate::csrrs;

const CSR_NO: usize = 0x104;
const STIE_BIT: usize = 1 << 5;

/// Sets the STIE bit to enable supervisor-level timer
/// interrupts, and returns the old value of the register.
pub fn set_stie() -> usize {
    let result: usize;
    csrrs!(CSR_NO, result, STIE_BIT);
    result
}
