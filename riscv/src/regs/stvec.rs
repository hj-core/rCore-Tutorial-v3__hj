use crate::csrw;

const CSR_NO: usize = 0x105; // Value comes from the RISCV manual Vol II
const MODE_BITS: usize = 0b11;

pub enum Mode {
    Direct,
    Vectored,
}

/// Installs a new exception handler.
///
/// The `base_addr` parameter specifies the address of the handler, and
/// the `mode` parameter specifies the handler mode. Returns true if the
/// handler was successfully installed, and false if the `base_addr` was
/// not properly aligned.
pub fn install(base_addr: usize, mode: Mode) -> bool {
    if base_addr & MODE_BITS != 0 {
        return false;
    }

    let addr = match mode {
        Mode::Direct => base_addr,
        Mode::Vectored => base_addr | 1,
    };
    csrw!(CSR_NO, addr);
    true
}
