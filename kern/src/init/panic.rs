use core::panic::PanicInfo;
use crate::console::{kprint, kprintln, CONSOLE};
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    if let Some(location) = _info.location() {
        kprintln!("
                (
            (      )     )
              )   (    (
             (          `
         .-\"\"\"\"\"^\"\"^\"\"\"^\"\"-.
       (//\\//\\//\\//\\//\\//)
        ~\\^^^^^^^^^^^^^^^^^^/~
          `================`
     
         The pi is overdone.
     
     ---------- PANIC ----------
     
     FILE: {:?}
     LINE: {:?}
     COL: {:?}
     
     {:?}", 
     location.file(), location.line(), location.column(), _info.message());
    } else {
      kprintln!("Pi panic in unknown location");
    }
    loop {}
}
