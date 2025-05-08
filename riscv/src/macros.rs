#[macro_export]
macro_rules! csrr {
    ($csr_no:expr, $output:ident) => {
        unsafe {
            core::arch::asm!("csrr {rd}, {csr}", rd = lateout(reg) $output, csr = const $csr_no)
        };
    };
}
