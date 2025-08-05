use crate::csrr;
use crate::csrw;

const CSR_NO: usize = 0x141;

pub fn read() -> usize {
    let result: usize;
    csrr!(CSR_NO, result);
    result
}

pub fn write(value: usize) {
    csrw!(CSR_NO, value);
}
