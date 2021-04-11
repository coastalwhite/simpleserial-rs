#![no_main]
#![no_std]

use panic_halt as _;

use simpleserial_rs::{CmdError, SimpleSerial};

fn invert_16_bit_key(scmd: u8, dlen: u8, data: &[u8]) -> Result<Option<(u8, u8, [u8; 192])>, CmdError> {
    if dlen == 16 {
        Ok(Some((b'r', 16, {
            let mut buff = [0; 192];
            for i in 0..16 {
                buff[i] = data[15 - i];
            }
            buff
        })))
    } else {
        Err(CmdError::InvalidLength)
    }
}

#[no_mangle]
fn main() -> ! {
    let mut cmds: SimpleSerial<8> = SimpleSerial::new();
    cmds.push(b'p', &invert_16_bit_key);

    loop {
        cmds.attempt_handle();
    }
}