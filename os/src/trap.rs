use core::arch::global_asm;

global_asm!(include_str!("trap/trap.S"));

pub fn init() {
    const STVEC_NO: usize = 0x105;
    unsafe extern "C" {
        unsafe fn __stvec();
    }
    riscv::csrw!(STVEC_NO, __stvec as usize);
}

#[unsafe(no_mangle)]
fn trap_handler() {
    panic!("os caught a trap!")
}
