const CSR_NO: usize = 0x180;

/// Returns the satp value corresponding to the given `ppn`
/// and `mode`. Any set bits in `ppn` other than the lower
/// 44 bits are ignored.
pub fn compute_value(ppn: usize, mode: Mode) -> usize {
    let ppn = ppn & 0xfff_ffff_ffff;
    let mode_value = get_mode_value(mode);
    (mode_value << 60) | ppn
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
