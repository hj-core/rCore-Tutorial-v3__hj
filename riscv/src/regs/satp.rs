use crate::csrw;

const CSR_NO: usize = 0x180;

/// Configures the `satp` register. Any set bits in `ppn` other
/// than the lower 44 bits are ignored.
pub fn enable(ppn: usize, mode: Mode) {
    let ppn = ppn & 0xfff_ffff_ffff;
    let mode_value = get_mode_value(mode);
    let value = (mode_value << 60) | ppn;
    csrw!(CSR_NO, value)
}

pub enum Mode {
    Bare,
    Sv39,
    Sv48,
    Sv57,
    Sv64,
}

fn get_mode_value(mode: Mode) -> usize {
    match mode {
        Mode::Bare => 0,
        Mode::Sv39 => 8,
        Mode::Sv48 => 9,
        Mode::Sv57 => 10,
        Mode::Sv64 => 11,
    }
}
