#![feature(asm)]
#![feature(global_asm)]

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

use rand_core::{RngCore, Error, impls};
use rand::Rng;

#[cfg(not(test))]
mod init;

const GPIO_BASE: usize = 0x3F000000 + 0x200000;

const GPIO_FSEL1: *mut u32 = (GPIO_BASE + 0x04) as *mut u32;
const GPIO_SET0: *mut u32 = (GPIO_BASE + 0x1C) as *mut u32;
const GPIO_CLR0: *mut u32 = (GPIO_BASE + 0x28) as *mut u32;

#[inline(never)]
fn spin_sleep_ms(ms: usize) {
    for _ in 0..(ms * 6000) {
        unsafe { asm!("nop" :::: "volatile"); }
    }
}

unsafe fn kmain() -> ! {
    init_rng();
    let mut rng: RdRand = Default::default();
    // STEP 1: Set GPIO Pin 16 as output.
    let v = GPIO_FSEL1.read_volatile();
    GPIO_FSEL1.write_volatile(v | 1 << 18); // set pin 16 as output 001 to location 18
    // STEP 2: Continuously set and clear GPIO 16.
    loop {
        GPIO_SET0.write_volatile(1<<16); // set pin 16
        spin_sleep_ms(rng.gen_range(0, 1000));
        GPIO_CLR0.write_volatile(1<<16); // clear pin 16
        spin_sleep_ms(rng.gen_range(0, 1000));
    }
}

// from https://github.com/bztsrc/raspi3-tutorial/blob/master/06_random/rand.c
unsafe fn init_rng() {
    RNG_STATUS.write_volatile(0x40000);
    let old_int_mask = RNG_INT_MASK.read_volatile();
    RNG_INT_MASK.write_volatile(old_int_mask | 1);
    let old_ctrl = RNG_CTRL.read_volatile();
    RNG_CTRL.write_volatile(old_ctrl | 1);
    // wait for entropy
    while !(RNG_STATUS.read_volatile() >> 24) == 0 {
        asm!("nop" :::: "volatile");
    }
}

impl RngCore for RdRand {
    fn next_u32(&mut self) -> u32 {
        unsafe {RNG_DATA.read_volatile()}
    }
 
    fn next_u64(&mut self) -> u64 {
        unsafe {
            let up = RNG_DATA.read_volatile() as u64;
            let low = RNG_DATA.read_volatile() as u64;
            (up << 32) | low
        }
    }
 
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        impls::fill_bytes_via_next(self, dest)
    }
 
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        Ok(self.fill_bytes(dest))
    }
}