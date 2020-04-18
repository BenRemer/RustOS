use core::fmt;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct TrapFrame {
    // Fill me in.
    pub xzr: u64,
    pub x30: u64,
    pub x0_x29: [u64; 30],
    pub q_regs: [u128; 32],
    pub tpidr: u64, 
    pub sp: u64, 
    pub pstate: u64,
    pub pc: u64,
}

