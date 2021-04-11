#[cfg(not(test))]
extern "C" {
    pub fn putch(c: u8);
    pub fn getch() -> u8;

    pub fn platform_init();
    pub fn init_uart();
    pub fn trigger_low();
    pub fn trigger_high();
    pub fn trigger_setup();
}

#[cfg(test)]
pub use test_fns::*;

/// Functions defined for the unit tests
#[cfg(test)]
mod test_fns {
    use lazy_static::lazy_static;
    extern crate std;
    use std::prelude::v1::*;
    use std::sync::Mutex;

    lazy_static! {
        static ref MODEL_STREAM: Mutex<Vec<u8>> = Mutex::new(Vec::new());
    }

    pub unsafe fn platform_init() {
        std::println!("Platform Init");
    }

    pub unsafe fn init_uart() {
        std::println!("Init Uart");
    }

    pub unsafe fn trigger_setup() {
        std::println!("Trigger Setup");
    }
    
    pub unsafe fn trigger_low() {
    }

    pub unsafe fn trigger_high() {
    }

    pub unsafe fn getch() -> u8 {
        let pop = MODEL_STREAM.lock().unwrap().pop().unwrap_or(0x00);
        std::println!("Popped: 0x{0:02x} / {0:03}", pop);
        pop
    }

    pub unsafe fn putch(c: u8) {
        std::println!("Pushed: 0x{0:02x} / {0:03}", c);
        MODEL_STREAM.lock().unwrap().insert(0, c);
    }

    #[cfg(test)]
    pub fn print_stream() {
        std::println!("MODEL STREAM: {:?}", MODEL_STREAM.lock().unwrap());
    }

    mod model_stream {
        use super::*;

        fn flush() {
            super::MODEL_STREAM.lock().unwrap().clear()
        }

        #[test]
        fn getch_on_empty() {
            unsafe {
                flush();

                assert_eq!(getch(), 0x00);
                assert_eq!(getch(), 0x00);
                assert_eq!(getch(), 0x00);

                flush();

                assert_eq!(getch(), 0x00);
                assert_eq!(getch(), 0x00);
                assert_eq!(getch(), 0x00);

                putch(0xAB);
                assert_eq!(getch(), 0xAB);

                assert_eq!(getch(), 0x00);
                assert_eq!(getch(), 0x00);
                assert_eq!(getch(), 0x00);
            }
        }

        #[test]
        fn queue_properties() {
            unsafe {
                flush();

                putch(0xAB);
                putch(0xBB);
                putch(0xCB);
                putch(0xDB);

                assert_eq!(getch(), 0xAB);
                assert_eq!(getch(), 0xBB);
                assert_eq!(getch(), 0xCB);
                assert_eq!(getch(), 0xDB);
            }
        }
    }
}

