#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

pub mod console;
pub mod mutex;
pub mod shell;

use console::kprintln;

// FIXME: You need to add dependencies here to
// test your drivers (Phase 2). Add them as needed.
use pi::timer::*;
use core::time::Duration;
use pi::gpio::Gpio;


unsafe fn kmain() -> ! {
    // FIXME: Start the shell.
    blink();
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
