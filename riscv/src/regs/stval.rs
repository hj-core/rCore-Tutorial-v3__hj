use crate::csrr;

const CSR_NO: usize = 0x143;

pub fn read() -> usize {
    let result: usize;
    csrr!(CSR_NO, result);
    result
}
