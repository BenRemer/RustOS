use crate::common::IO_BASE;

use volatile::prelude::*;
use volatile::{Volatile, ReadVolatile};

const INT_BASE: usize = IO_BASE + 0xB000 + 0x200;

#[derive(Copy, Clone, PartialEq)]
pub enum Interrupt {
    Timer1 = 1,
    Timer3 = 3,
    Usb = 9,
    Gpio0 = 49,
    Gpio1 = 50,
    Gpio2 = 51,
    Gpio3 = 52,
    Uart = 57,
}

impl Interrupt {
    pub const MAX: usize = 8;

    pub fn iter() -> core::slice::Iter<'static, Interrupt> {
        use Interrupt::*;
        [Timer1, Timer3, Usb, Gpio0, Gpio1, Gpio2, Gpio3, Uart].into_iter()
    }

    pub fn to_index(i: Interrupt) -> usize {
        use Interrupt::*;
        match i {
            Timer1 => 0,
            Timer3 => 1,
            Usb => 2,
            Gpio0 => 3,
            Gpio1 => 4,
            Gpio2 => 5,
            Gpio3 => 6,
            Uart => 7,
        }
    }

    pub fn from_index(i: usize) -> Interrupt {
        use Interrupt::*;
        match i {
            0 => Timer1,
            1 => Timer3,
            2 => Usb,
            3 => Gpio0,
            4 => Gpio1,
            5 => Gpio2,
            6 => Gpio3,
            7 => Uart,
            _ => panic!("Unknown interrupt: {}", i),
        }
    }
}


impl From<usize> for Interrupt {
    fn from(irq: usize) -> Interrupt {
        use Interrupt::*;
        match irq {
            1 => Timer1,
            3 => Timer3,
            9 => Usb,
            49 => Gpio0,
            50 => Gpio1,
            51 => Gpio2,
            52 => Gpio3,
            57 => Uart,
            _ => panic!("Unkonwn irq: {}", irq),
        }
    }
}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    // Fill me in.
    pending_basic: u32,
    pending: [ReadVolatile<u32>; 2],
    fiq_control: u32,
    enable: [Volatile<u32>; 2],
    enable_basic: u32,
    disable: [Volatile<u32>; 2],
    disable_basic: u32,
}

/// An interrupt controller. Used to enable and disable interrupts as well as to
/// check if an interrupt is pending.
pub struct Controller {
    registers: &'static mut Registers
}

impl Controller {
    /// Returns a new handle to the interrupt controller.
    pub fn new() -> Controller {
        Controller {
            registers: unsafe { &mut *(INT_BASE as *mut Registers) },
        }
    }

    /// Enables the interrupt `int`.
    pub fn enable(&mut self, int: Interrupt) {
        let index = int as u64;
        if index < 32 {
            self.registers.enable[0].or_mask(1 << index);
        } else {
            self.registers.enable[1].or_mask(1 << (index - 32));
        }
    }

    /// Disables the interrupt `int`.
    pub fn disable(&mut self, int: Interrupt) {
        let index = int as u64;
        if index < 32 {
            self.registers.disable[0].or_mask(1 << index);
        } else {
            self.registers.disable[1].or_mask(1 << (index - 32));
        }
    }

    /// Returns `true` if `int` is pending. Otherwise, returns `false`.
    pub fn is_pending(&self, int: Interrupt) -> bool {
        let index = int as u64;
        if index < 32 {
            self.registers.pending[0].has_mask(1 << index)
        } else {
            self.registers.pending[1].has_mask(1 << (index - 32))
        }
    }
}
