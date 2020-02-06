#![feature(asm)]
#![feature(global_asm)]

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

use xmodem::Xmodem;
use core::time::Duration;
use pi;
// use pi::uart::MiniUart;

/// Start address of the binary to load and of the bootloader.
const BINARY_START_ADDR: usize = 0x80000;
const BOOTLOADER_START_ADDR: usize = 0x4000000;

/// Pointer to where the loaded binary expects to be loaded.
const BINARY_START: *mut u8 = BINARY_START_ADDR as *mut u8;

/// Free space between the bootloader and the loaded binary's start address.
const MAX_BINARY_SIZE: usize = BOOTLOADER_START_ADDR - BINARY_START_ADDR;

/// Branches to the address `addr` unconditionally.
unsafe fn jump_to(addr: *mut u8) -> ! {
    asm!("br $0" : : "r"(addr as usize));
    loop {
        asm!("wfe" :::: "volatile")
    }
}

fn kmain() -> ! {
    // Implement the bootloader.
    let mut location: &mut [u8] = unsafe {core::slice::from_raw_parts_mut(BINARY_START, MAX_BINARY_SIZE)};
    let mut uart = pi::uart::MiniUart::new();
    uart.set_read_timeout(Duration::from_millis(750));
    loop {
        match Xmodem::receive(&mut uart, &mut location) {
            Ok(_) => {
                // kprintln!("Received transmission");
                break
            },
            Err(_) => {
                // kprintln!("Error, retrying");
                continue
            }
        }
    }
    unsafe {jump_to(BINARY_START)}
}
