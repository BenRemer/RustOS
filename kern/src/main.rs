#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![feature(ptr_internals)]
#![feature(raw_vec_internals)]
#![feature(panic_info_message)]

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

extern crate alloc;

pub mod allocator;
pub mod console;
pub mod fs;
pub mod mutex;
pub mod shell;
pub mod param;
pub mod process;
pub mod traps;
pub mod vm;

use console::kprintln;
use allocator::Allocator;
use fs::FileSystem;
use process::GlobalScheduler;
use traps::irq::Irq;
use vm::VMManager;


// You need to add dependencies here to
// test your drivers (Phase 2). Add them as needed.
use core::time::Duration;
use pi::timer::*;
use pi::gpio::Gpio;
use pi::uart::MiniUart;
use core::fmt::Write;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();
pub static SCHEDULER: GlobalScheduler = GlobalScheduler::uninitialized();
pub static VMM: VMManager = VMManager::uninitialized();
pub static IRQ: Irq = Irq::uninitialized();

fn kmain() -> ! {
    // Start the shell.
    unsafe {
        ALLOCATOR.initialize();
        FILESYSTEM.initialize();
        IRQ.initialize();
        VMM.initialize();
        SCHEDULER.initialize();
        SCHEDULER.start();
    }
    
    // spin_sleep(Duration::new(1,0));
    // unsafe{current_el()}
    // kprintln!("Welcome");
    // panic!("error");
    // use alloc::vec::Vec;

    // let mut v = Vec::new();
    // for i in 0..50 {
    //     v.push(i);
    //     kprintln!("{:?}", v);
    // }
    // shell::shell("$");
}

fn write() -> ! {
    // kprintln!("Console is working!");
    let mut uart = MiniUart::new();
    loop {
        let mut byte = uart.read_byte();
        // uart.write_byte(byte);
        // uart.write_str("<3");
    }
}

unsafe fn blink() -> ! {
    // let mut output = Gpio::new(16).into_output();
    let mut leds = [
        Gpio::new(5).into_output(),
        Gpio::new(6).into_output(),
        Gpio::new(13).into_output(),
        Gpio::new(19).into_output(),
        Gpio::new(26).into_output(),
    ];
    let dur = Duration::new(1,0);
    loop {
        for mut led in leds.iter_mut() {
            led.set();
            spin_sleep(dur);
            led.clear();
            spin_sleep(dur);
        }
    }
    // loop {
    //     output.set();
    //     spin_sleep(dur);
    //     output.clear();
    //     spin_sleep(dur);
    // }
}
