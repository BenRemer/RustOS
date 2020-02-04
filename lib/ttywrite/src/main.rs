mod parsers;

use serial;
use structopt;
use structopt_derive::StructOpt;
use xmodem::Xmodem;
use xmodem::Progress;

use std::path::PathBuf;
use std::time::Duration;

use structopt::StructOpt;
use serial::core::{CharSize, BaudRate, StopBits, FlowControl, SerialDevice, SerialPortSettings};

use parsers::{parse_width, parse_stop_bits, parse_flow_control, parse_baud_rate};

#[derive(StructOpt, Debug)]
#[structopt(about = "Write to TTY using the XMODEM protocol by default.")]
struct Opt {
    #[structopt(short = "i", help = "Input file (defaults to stdin if not set)", parse(from_os_str))]
    input: Option<PathBuf>,

    #[structopt(short = "b", long = "baud", parse(try_from_str = "parse_baud_rate"),
                help = "Set baud rate", default_value = "115200")]
    baud_rate: BaudRate,

    #[structopt(short = "t", long = "timeout", parse(try_from_str),
                help = "Set timeout in seconds", default_value = "10")]
    timeout: u64,

    #[structopt(short = "w", long = "width", parse(try_from_str = "parse_width"),
                help = "Set data character width in bits", default_value = "8")]
    char_width: CharSize,

    #[structopt(help = "Path to TTY device", parse(from_os_str))]
    tty_path: PathBuf,

    #[structopt(short = "f", long = "flow-control", parse(try_from_str = "parse_flow_control"),
                help = "Enable flow control ('hardware' or 'software')", default_value = "none")]
    flow_control: FlowControl,

    #[structopt(short = "s", long = "stop-bits", parse(try_from_str = "parse_stop_bits"),
                help = "Set number of stop bits", default_value = "1")]
    stop_bits: StopBits,

    #[structopt(short = "r", long = "raw", help = "Disable XMODEM")]
    raw: bool,
}

fn progress_fn(progress: Progress) {
    println!("Progress: {:?}", progress);
}

fn main() {
    use std::fs::File;
    use std::io::{self, BufReader};
 
    let opt = Opt::from_args();
    let mut port = serial::open(&opt.tty_path).expect("path points to invalid TTY");
  
    let mut settings = match port.read_settings() {
        Ok(settings) => settings,
        Err(_err) => panic!("No settings found: {}", _err),
    };
   
    match settings.set_baud_rate(opt.baud_rate) {
        Ok(()) => println!("baud rate changed"),
        Err(_err) => panic!("error with baud rate"),
    }

    settings.set_char_size(opt.char_width);
    settings.set_stop_bits(opt.stop_bits);
    settings.set_flow_control(opt.flow_control);

    match port.write_settings(&settings) {
        Ok(()) => println!("settings changed"),
        Err(_err) => panic!("error settings not changed"),
    }

    match port.set_timeout(Duration::new(opt.timeout, 0)) {
        Ok(()) => println!("timeout changed"),
        Err(_err) => panic!("timeout not changed"),
    }
 
    match opt.input {
        Some(input_file) => { // input file given
            let path = match input_file.into_os_string().into_string() {
                Ok(p) => p,
                Err(_err) => panic!("invalid string"), 
            };
 
            let file = match File::open(path) {
                Ok(f) => f,
                Err(_err) => panic!("{}", _err),    
            };

            let mut buffer = BufReader::new(file);
 
            match opt.raw {
                true => { // take as raw
                    match io::copy(&mut buffer, &mut port)  {
                        Ok(num_bytes) => println!("wrote {} bytes to input", num_bytes),
                        Err(_err) => println!("error: {}", _err),
                    }
                },
                false => { // use Xmodem
                    match Xmodem::transmit_with_progress(buffer, port, progress_fn) {
                        Ok(num_bytes) => println!("wrote {} bytes to input", num_bytes),
                        Err(_err) => println!("error: {}", _err),
                    }
                }
            }
        },
        None => { // read stdin
            let mut buffer = BufReader::new(io::stdin());
            match opt.raw {
                true => {
                    match io::copy(&mut buffer, &mut port) {
                        Ok(num_bytes) => println!("wrote {} bytes to input", num_bytes),
                        Err(_err) => println!("error: {}", _err),
                    }
                },
                false => {
                    match Xmodem::transmit_with_progress(buffer, port, progress_fn) {
                        Ok(n) => println!("wrote {} bytes to input", n),
                        Err(_err) => println!("error: {}", _err),
                    }
                }
            }
        }
    }
   
}
