use crate::csrr;

const CSR_NO: usize = 0x142;
const INTERRUPT_BIT: usize = 1 << 63;

pub fn read() -> usize {
    let result: usize;
    csrr!(CSR_NO, result);
    result
}

pub fn is_interrupt(value: usize) -> bool {
    value & INTERRUPT_BIT != 0
}

#[derive(Debug)]
pub enum Cause {
    // Interrupt
    SupervisorSoftwareInterrupt,
    SupervisorTimerInterrupt,
    SupervisorExternalInterrupt,
    CounterOverflowInterrupt,
    // Exception
    InstructionAddressMisaligned,
    InstructionAccessFault,
    IllegalInstruction,
    Breakpoint,
    LoadAddressMisaligned,
    LoadAccessFault,
    StoreOrAmoAddressMisaligned,
    StoreOrAmoAccessFault,
    UserEnvironmentCall,
    SupervisorEnvironmentCall,
    InstructionPageFault,
    LoadPageFault,
    StoreOrAmoPageFault,
    SoftwareCheck,
    HardwareError,
    // Other
    Unknown,
}

/// Returns the [Cause] corresponding to the `value`.
pub fn match_cause(value: usize) -> Cause {
    if is_interrupt(value) {
        match_interrupt_cause(value)
    } else {
        match_exception_cause(value)
    }
}

fn match_interrupt_cause(value: usize) -> Cause {
    match value ^ INTERRUPT_BIT {
        1 => Cause::SupervisorSoftwareInterrupt,
        5 => Cause::SupervisorTimerInterrupt,
        9 => Cause::SupervisorExternalInterrupt,
        13 => Cause::CounterOverflowInterrupt,
        _ => Cause::Unknown,
    }
}

fn match_exception_cause(value: usize) -> Cause {
    match value {
        0 => Cause::InstructionAddressMisaligned,
        1 => Cause::InstructionAccessFault,
        2 => Cause::IllegalInstruction,
        3 => Cause::Breakpoint,
        4 => Cause::LoadAddressMisaligned,
        5 => Cause::LoadAccessFault,
        6 => Cause::StoreOrAmoAddressMisaligned,
        7 => Cause::StoreOrAmoAccessFault,
        8 => Cause::UserEnvironmentCall,
        9 => Cause::SupervisorEnvironmentCall,
        12 => Cause::InstructionPageFault,
        13 => Cause::LoadPageFault,
        15 => Cause::StoreOrAmoPageFault,
        18 => Cause::SoftwareCheck,
        19 => Cause::HardwareError,
        _ => Cause::Unknown,
    }
}
