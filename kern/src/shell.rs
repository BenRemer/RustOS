use shim::io;
use shim::path::{Path, PathBuf};

use stack_vec::StackVec;
use core::fmt::Write;


use pi::atags::Atags;

use fat32::traits::FileSystem;
use fat32::traits::{Dir, Entry};

use crate::console::{kprint, kprintln, CONSOLE};
use crate::ALLOCATOR;
use crate::FILESYSTEM;

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs,
}

/// A structure representing a single shell command.
struct Command<'a> {
    args: StackVec<'a, &'a str>,
}

impl<'a> Command<'a> {
    /// Parse a command from a string `s` using `buf` as storage for the
    /// arguments.
    ///
    /// # Errors
    ///
    /// If `s` contains no arguments, returns `Error::Empty`. If there are more
    /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
    fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split(' ').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }

        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Command { args })
    }

    /// Returns this command's path. This is equivalent to the first argument.
    fn path(&self) -> &str {
        self.args[0]
    }
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// returns if the `exit` command is called.
const BACKSPACE: u8 = 8;
const DELETE:u8 = 127;
const BELL: u8 = 7;

pub fn shell(prefix: &str) -> ! {
    loop { // loop forever
        let mut buf_storage = [0u8; 512];
        let mut buf = StackVec::new(&mut buf_storage);

        kprint!("{} ", prefix);

        loop { // loop until they exit
            let byte = CONSOLE.lock().read_byte();
            if byte == b'\r' || byte == b'\n' {
                let cmd: &str = match core::str::from_utf8(&buf.as_slice()) {
                    Ok(s) => s,
                    Err(_) => {
                        kprint!("\nerror: failed to parse command");
                        break
                    }
                };

                let mut line = [""; 64];
                let command = match Command::parse(cmd, &mut line) {
                    Ok(command) => command,
                    Err(Error::TooManyArgs) => {
                        kprint!("\nerror: too many arguments");
                        break
                    },
                    Err(Error::Empty) => {
                        break
                    }
                };

                match command.path() { // match it's command
                    "echo" => {
                        let mut console = CONSOLE.lock();
                        console.write_str("\n\r");
                        for arg in command.args.iter().skip(1) {
                            console.write_str(arg);
                            console.write_str(" ");
                        }
                        break
                    }
                    "exit" => {
                        kprint!("\nThank you for using shell.");
                        break
                    }
                    _ => {
                        kprint!("\nunknown command: {}", command.path());
                        break
                    }
                }

            } else if byte == BACKSPACE || byte == DELETE { // backspace or delete
                let mut console = CONSOLE.lock();
                if !(buf.is_empty()) {
                    console.write_byte(BACKSPACE);
                    console.write_byte(b' ');
                    console.write_byte(BACKSPACE);
                    buf.pop();
                }
            } else if byte >= 32 && byte <= 126 { // write byte to console
                let mut console = CONSOLE.lock();
                console.write_byte(byte);
                buf.push(byte);
            } else { // error, char not allowed
                let mut console = CONSOLE.lock();
                console.write_byte(BELL);
            }
        }
        kprintln!("\r") // print \r to show end of line
    }
}
