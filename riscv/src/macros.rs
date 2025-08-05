/// Reads a CSR.
///
/// This macro reads the value of the CSR specified by
/// `csr_no` and writes it to the variable `output`. It uses
/// the `csrr` instruction.
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

/// Writes a CSR.
///
/// This macro writes the `value` to the CSR specified by
/// `csr_no`. It uses the `csrw` instruction.
#[macro_export]
macro_rules! csrw {
    ($csr_no:expr, $value:expr) => {
        unsafe {
            core::arch::asm!(
                "csrw {csr}, {rs1}",
                csr = const $csr_no,
                rs1 = in(reg) $value,
            )
        }
    };
}

/// Clears bits in a CSR.
///
/// This macro clears the bits specified by the `mask` in
/// the CSR specified by `csr_no`, and writes the previous
/// value of the CSR to the variable `output`. It uses the
/// `csrrc` instruction.
#[macro_export]
macro_rules! csrrc {
    ($csr_no:expr, $output:ident, $mask:expr) => {
        unsafe {
            core::arch::asm!(
                "csrrc {rd}, {csr}, {rs1}",
                rd = lateout(reg) $output,
                csr = const $csr_no,
                rs1 = in(reg) $mask,
            )
        }
    };
}

/// Sets bits in a CSR.
///
/// This macro sets the bits specified by the `mask` in the
/// CSR specified by `csr_no`, and writes the previous value
/// of the CSR to the variable `output`. It uses the `csrrs`
/// instruction.
#[macro_export]
macro_rules! csrrs{
    ($csr_no:expr, $output:ident, $mask:expr) => {
        unsafe {
            core::arch::asm!(
                "csrrs {rd}, {csr}, {rs1}",
                rd = lateout(reg) $output,
                csr = const $csr_no,
                rs1 = in(reg) $mask,
            )
        }
    };
}
