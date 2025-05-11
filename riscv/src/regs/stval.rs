use crate::csrr;

const CSR_NO: usize = 0x143;

pub fn read() -> usize {
    let mut result: usize;
    csrr!(CSR_NO, result);
    result
}
