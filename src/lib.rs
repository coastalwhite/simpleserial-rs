//! The ChipWhisperer Simple Serial Protocol
//!
#![no_std]
#![warn(missing_docs)]
#![deny(macro_use_extern_crate)]

#[link(name = "simpleserial", kind = "static")]
extern "C" {
    fn simpleserial_init();
    fn simpleserial_addcmd(
        c: u8,
        len: usize,
        fp: Option<extern "C" fn(arg1: *mut u8) -> u8>,
    ) -> i32;
    fn simpleserial_get();
    fn simpleserial_put(c: u8, size: u8, output: *mut u8);
    fn platform_init();
    fn init_uart();
    fn trigger_setup();
}

mod hex_ascii;

/// All SimpleSerial Commands as structs
pub mod cmds {
    /// The base command trait
    pub trait Command {
        /// The prefix character for a command
        const CMD_PREFIX: u8;
    }

    /// Trait for incoming commands
    pub trait InCommand: Command {
        /// Register handler to fire on arriving of command
        fn on_arrive<const ARG_SIZE: usize>(cb_handler: extern "C" fn(*mut u8) -> u8) {
            unsafe {
                super::simpleserial_addcmd(Self::CMD_PREFIX, ARG_SIZE, Some(cb_handler));
            }
        }
    }

    /// Trait for outgoing commands
    pub trait OutCommand<const ARG_SIZE: usize>: Command {
        /// Fetch the Hex ASCII string for a certain command
        fn get_cmd_arg(&self) -> [u8; ARG_SIZE];

        /// Send the outgoing command to the host machine
        fn send(&self) {
            use core::convert::TryInto;

            unsafe {
                super::simpleserial_put(
                    Self::CMD_PREFIX,
                    ARG_SIZE.try_into().unwrap(),
                    self.get_cmd_arg().as_mut_ptr(),
                );
            }
        }
    }

    // In's
    /// Select stack / hardware to use (if supported).
    pub struct SelectStackOrHardware;
    /// Set encryption key; possibly trigger key scheduling
    pub struct SetEncryptionKey;
    /// Select cipher mode (if supported)
    pub struct SetCipherMode;
    /// Send input plain-text, cause encryption
    pub struct SendPlainText;
    /// Authentication challenge (i.e., expected AES result if using AES as auth-method)
    pub struct AuthChallenge;
    /// Check protocol version (no reply on v1.0; ACK on v1.1)
    pub struct CheckProtocolVersion;
    /// Clears Buffers (resets to 'IDLE' state), does not clear any variables.
    pub struct ClearBuffers;

    // Out's
    /// Result of function - if encryption is encrypted result, if auth is '0..0' or '100..0'.
    pub struct ResultOfFn<const R: usize>(pub [u8; R]);
    /// ACK - Command processing done (with optional status code)
    pub struct Ack(pub u8);

    impl Command for SelectStackOrHardware {
        const CMD_PREFIX: u8 = b'h';
    }
    impl Command for SetEncryptionKey {
        const CMD_PREFIX: u8 = b'k';
    }
    impl Command for SetCipherMode {
        const CMD_PREFIX: u8 = b'm';
    }
    impl Command for SendPlainText {
        const CMD_PREFIX: u8 = b'p';
    }
    impl Command for AuthChallenge {
        const CMD_PREFIX: u8 = b't';
    }
    impl Command for CheckProtocolVersion {
        const CMD_PREFIX: u8 = b'v';
    }
    impl Command for ClearBuffers {
        const CMD_PREFIX: u8 = b'x';
    }

    impl<const R: usize> Command for ResultOfFn<R> {
        const CMD_PREFIX: u8 = b'r';
    }
    impl Command for Ack {
        const CMD_PREFIX: u8 = b'z';
    }

    impl InCommand for SelectStackOrHardware {}
    impl InCommand for SetEncryptionKey {}
    impl InCommand for SetCipherMode {}
    impl InCommand for SendPlainText {}
    impl InCommand for AuthChallenge {}
    impl InCommand for CheckProtocolVersion {}
    impl InCommand for ClearBuffers {}

    impl<const R: usize, const ARG_SIZE: usize> OutCommand<ARG_SIZE> for ResultOfFn<R> {
        fn get_cmd_arg(&self) -> [u8; ARG_SIZE] {
            let mut arg = [0; ARG_SIZE];

            let ResultOfFn(bytes) = self;

            for (index, byte) in bytes.iter().enumerate() {
                let hex_ascii = super::hex_ascii::byte_to_hex_ascii(byte);

                arg[index * 2] = hex_ascii[0];
                arg[index * 2 + 1] = hex_ascii[1];
            }

            arg
        }
    }

    impl OutCommand<2> for Ack {
        fn get_cmd_arg(&self) -> [u8; 2] {
            let Ack(byte) = self;
            super::hex_ascii::byte_to_hex_ascii(byte)
        }
    }
}

/// Set up the SimpleSerial module
/// This prepares any internal commands
pub fn init() {
    unsafe {
        platform_init();
        init_uart();
        trigger_setup();
        simpleserial_init();
    }
}

/// Attempt to process a command
/// If a full string is found, the relevant callback function is called
/// Might return without calling a callback for several reasons:
/// - First character didn't match any known commands
/// - One of the characters wasn't in [0-9|A-F|a-f]
/// - Data was too short or too long
pub fn get() {
    unsafe {
        simpleserial_get();
    }
}
