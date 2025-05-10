/// Reads a CSR.
///
/// This macro reads the value of the CSR specified by `csr_no` and writes
/// it to the variable `output`. It uses the `csrr` instruction.
#[macro_export]
macro_rules! csrr {
    ($csr_no:expr, $output:ident) => {
        unsafe {
            core::arch::asm!(
                "csrr {rd}, {csr}",
                rd = lateout(reg) $output,
                csr = const $csr_no,
            )
        };
    };
}

#[macro_export]
macro_rules! csrw {
    ($csr_no:expr, $value:expr) => {
        unsafe {
            core::arch::asm!("csrw {csr}, {rs1}", csr = const $csr_no, rs1 = in(reg) $value)
        }
    };
}
