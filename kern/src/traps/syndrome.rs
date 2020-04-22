use aarch64::ESR_EL1;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Fault {
    AddressSize,
    Translation,
    AccessFlag,
    Permission,
    Alignment,
    TlbConflict,
    Other(u8),
}

impl From<u32> for Fault {
    fn from(val: u32) -> Fault {
        use self::Fault::*;
        match (val & 0b111111) as u8 {
            0b000000..=0b000011 => AddressSize,
            0b000100..=0b000111 => Translation,
            0b001001..=0b001011 => AccessFlag,
            0b001101..=0b001111 => Permission,
            0b100001 => Alignment,
            0b110000 => TlbConflict,
            other => Other(other),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Syndrome {
    Unknown,
    WfiWfe,
    SimdFp,
    IllegalExecutionState,
    Svc(u16),
    Hvc(u16),
    Smc(u16),
    MsrMrsSystem,
    InstructionAbort { kind: Fault, level: u8 },
    PCAlignmentFault,
    DataAbort { kind: Fault, level: u8 },
    SpAlignmentFault,
    TrappedFpu,
    SError,
    Breakpoint,
    Step,
    Watchpoint,
    Brk(u16),
    Other(u32),
}

/// Converts a raw syndrome value (ESR) into a `Syndrome` (ref: D1.10.4).
impl From<u32> for Syndrome {
    fn from(esr: u32) -> Syndrome {
        use self::Syndrome::*;
        let esr_mask = 0xFFFF;
        let ecc = ESR_EL1::get_value(esr as u64, ESR_EL1::EC) as u8;
        let iss = ESR_EL1::get_value(esr as u64, ESR_EL1::ISS) as u32;
        let kind = Fault::from(iss);
        let level = (iss & 0b11) as u8;
        let esr_masked = ESR_EL1::get_value(esr as u64, ESR_EL1::ISS_HSVC_IMM) as u16;
        match ecc {
            0b000000 => Unknown,
            0b000001 => WfiWfe,
            0b000111 => SimdFp,
            0b001110 => IllegalExecutionState,
            0b010001 => Svc(esr_masked),
            0b010010 => Hvc(esr_masked),
            0b010011 => Smc((esr >> 19) as u16),
            0b010101 => Svc(esr_masked),
            0b010110 => Hvc(esr_masked),
            0b010111 => Smc(esr_masked),
            0b011000 => MsrMrsSystem,
            0b100000 => InstructionAbort { kind: kind, level: level },
            0b100001 => InstructionAbort { kind: kind, level: level },
            0b100010 => PCAlignmentFault,
            0b100100 => DataAbort { kind: kind, level: level },
            0b100101 => DataAbort { kind: kind, level: level },
            0b100110 => SpAlignmentFault,
            0b101000 => TrappedFpu,
            0b101100 => TrappedFpu,
            0b101111 => SError,
            0b110000 => Breakpoint,
            0b110001 => Breakpoint,
            0b110010 => Step,
            0b110011 => Step,
            0b110100 => Watchpoint,
            0b110101 => Watchpoint,
            0b111000 => Breakpoint,
            0b111100 => Brk((esr & esr_mask) as u16),
            _ => Other(esr)
        }
    }
}
