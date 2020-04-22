use shim::io;
use shim::path::{Path, PathBuf};

use stack_vec::StackVec;
use core::fmt::Write;
// use crate::alloc::string::ToString;

use alloc::string::String; // TODO might be wrong

use pi::atags::Atags;

use fat32::traits::FileSystem;
use fat32::traits::{Dir, Entry, Metadata, Timestamp};

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

fn echo(args: &[&str]) {
    // kprintln!(" ");
    for arg in args {
        kprintln!("{} ", arg);
    }
}

fn pwd(working_dir: &PathBuf) {
    // let mut console = CONSOLE.lock();
    // console.write_str(working_dir.as_path().display());
    kprintln!("{}", working_dir.as_path().display());
}

fn cd(args: &[&str], working_dir: &mut PathBuf) {
    // let mut console = CONSOLE.lock();
    if args.len() != 1 {
        kprintln!("incorrect usage, please use: cd <directory>");
        kprintln!("");
        return;
    }
    if args[0] == "." {
        // nothing
    } else if args[0] == ".." {
        working_dir.pop();
    } else {
        let path = Path::new(args[0]);
        let mut new_dir = working_dir.clone();
        new_dir.push(path);
        let entry = FILESYSTEM.open(new_dir.as_path());
        if entry.is_err() {
            kprintln!("Path not found.");
            return;
        }
        if entry.unwrap().as_dir().is_some() {
            working_dir.push(path);
        } else {
            kprintln!("Not a directory.");
        }
    }
}

fn ls(mut args: &[&str], working_dir: &PathBuf) {
    // let mut console = CONSOLE.lock();
    let show_hidden = args.len() > 0 && args[0] == "-a";
    if show_hidden {
        args = &args[1..];
    }
    if args.len() > 1 { // incorrect usage
        kprintln!("Incorrect agrs, please use: ls [-a] [directory]");
        kprintln!("");
        return;
    }
    let mut dir = working_dir.clone();
    if !args.is_empty() {
        if args[0] == "." {
            // empty
        } else if args[0] == ".." {
            dir.pop();
        } else {
            dir.push(args[0]);
        }
    }
    let result = FILESYSTEM.open(dir.as_path());
    if result.is_err() {
        kprintln!("Path not found.");
        return;
    }
    let entry = result.unwrap();
    if let Some(dir) = entry.into_dir() {
        let mut entries = dir.entries().expect("List dir");
        for item in entries {
            if show_hidden || !item.metadata().hidden() {
                // let mut console = CONSOLE.lock();
                fn write_fields(b: bool, c: char) {
                    if b {
                        kprint!("{}", c); 
                    } else { 
                        kprint!("-"); 
                    }
                }

                fn timestamp<T: Timestamp>(ts: T) {
                    kprint!("{:02}/{:02}/{} {:02}:{:02}:{:02} ",
                        ts.month(), ts.day(), ts.year(), ts.hour(), ts.minute(), ts.second());
                }
                write_fields(item.is_dir(), 'd');
                write_fields(item.is_file(), 'f');
                write_fields(item.metadata().read_only(), 'r');
                write_fields(item.metadata().hidden(), 'h');
                kprintln!("\t");
                timestamp(item.metadata().created());
                timestamp(item.metadata().modified());
                timestamp(item.metadata().accessed());
                kprintln!("\t");
                kprintln!("{}", item.name());
            }
        }
    } else {
        kprintln!("no dir found");
    }
}

fn cat(args: &[&str], working_dir: &PathBuf) {
    use shim::io::Read;
    // let mut console = CONSOLE.lock();
    if args.len() != 1 {
        kprintln!("Incorrect Arguments, use cat <file>");
        kprintln!("");
        return;
    }
    let mut dir = working_dir.clone();
    dir.push(args[0]);
    let result = FILESYSTEM.open(dir.as_path());
    if result.is_err() {
        kprintln!("Path not found");
        return;
    }
    let entry = result.unwrap();
    if let Some(ref mut file) = entry.into_file() {
        loop {
            let mut buffer = [0u8; 512];
            match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(_) => kprintln!("{}", String::from_utf8_lossy(&buffer)),
                Err(e) => kprintln!("Failed to read file: {:?}", e)
            }
        }
        kprintln!("");
    } else {
        kprintln!("Not a file");
    }
}

fn sleep(ms: &str) {
    use core::str::FromStr;
    use core::time::Duration;
    use kernel_api::syscall::sleep;
    let ms = match u32::from_str(ms) {
        Ok(t) => t,
        Err(_) => {
            kprintln!("Error changing to ms");
            return
        }
    };
    match sleep(Duration::from_millis(ms.into())) {
        Ok(t) => kprintln!("Slept {:?}", t),
        Err(e) => kprintln!("Error while sleeping {:?}", e),
    }
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// returns if the `exit` command is called.
const BACKSPACE: u8 = 8;
const DELETE:u8 = 127;
const BELL: u8 = 7;

pub fn shell(prefix: &str) -> ! {
    // panic!("panic");
    let mut working_directory = PathBuf::from("/");
    // kprintln!("{}", working_directory.as_path().display());
    loop { // loop forever
        let mut buf_storage = [0u8; 512];
        let mut buf = StackVec::new(&mut buf_storage);

        // match FILESYSTEM.open("/") {
        //     Ok(_) => {},
        //     Err(e) => {
        //         kprintln!( "error opening at /: {}", e );
        //         panic!();
        //     }
        // };
    
        // let mut working_directory = PathBuf::new();
        // working_directory.push("/");

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
                kprintln!("{}", BACKSPACE);
                match command.path() { // match it's command
                    "echo" => echo(&command.args[1..]),
                    "pwd" => pwd(&working_directory),
                    "cd" => cd(&command.args[1..], &mut working_directory),
                    "ls" => ls(&command.args[1..], &working_directory),
                    "cat" => cat(&command.args[1..], &working_directory),
                    "sleep" => sleep(&command.args[1]),
                    _ =>  kprint!("\nunknown command: {}", command.path()),
                }
                break
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
