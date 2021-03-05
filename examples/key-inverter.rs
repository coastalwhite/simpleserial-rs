#![no_std]
#![no_main]

use panic_halt as _;
use cortex_m_rt::entry;

use simpleserial_rs::cmds::{ InCommand, OutCommand };

extern fn invert_16_bit_key(_: *mut u8) -> u8 {
    let result_cmd = simpleserial_rs::cmds::ResultOfFn([0x42; 16]);
    OutCommand::<32>::send(&result_cmd);
    0
}

#[entry]
fn main() -> ! {
    simpleserial_rs::init();

    simpleserial_rs::cmds::SetEncryptionKey::on_arrive::<32>(invert_16_bit_key);

    loop {
        simpleserial_rs::get();
    }
}