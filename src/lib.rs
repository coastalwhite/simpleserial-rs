//! The ChipWhisperer Simple Serial Protocol
//!
#![no_std]
#![warn(missing_docs)]
#![deny(macro_use_extern_crate)]

use array_utils::{array_resize, drift_to_end, superimpose};

pub(crate) mod util;

pub(crate) mod firmware;
use firmware::*;

mod capture_to_target;
mod target_to_capture;

use capture_to_target::CTPacket;
use target_to_capture::TCPacket;

use util::{pkt_insert_crc8, pkt_stuff, read_away, write_away};

/// The generator polynomial used for the Cyclic Redundancy Checks
pub(crate) const CRC_GEN_POLY: u8 = 0xA6;

/// Errors
#[cfg_attr(test, derive(Debug, PartialEq, Clone))]
pub enum PktError {
    /// There were insufficient bytes in the BUS
    InsufficientBytes {
        /// Length of BUS buffer
        buffer_length: usize,
    },
    /// The DLEN didn't match the actual data
    IncorrectDataLength {
        /// Length of BUS buffer
        buffer_length: usize,
        /// DLEN
        data_length: usize,
    },
    /// There was an invalid CRC
    CrcInvalid,
}

pub enum CmdError {
    OK,
    InvalidCommand,
    BadCRC,
    Timeout,
    InvalidLength,
    UnexpectedFrameByte,
    Custom(u8),
}

impl CmdError {
    fn get_byte(&self) -> u8 {
        use CmdError::*;

        match self {
            OK => 0,
            InvalidCommand => 1,
            BadCRC => 2,
            Timeout => 3,
            InvalidLength => 4,
            UnexpectedFrameByte => 5,
            Custom(b) => *b,
        }
    }
}

type CmdResponse = (u8, u8, [u8; 192]);
type CmdFn = &'static dyn Fn(u8, u8, &[u8]) -> Result<Option<CmdResponse>, CmdError>;
type CmdSpecification = (u8, CmdFn);

/// Container for SimpleSerial commands
pub struct SimpleSerial<const MAX_CMDS: usize> {
    cmds: [Option<CmdSpecification>; MAX_CMDS],
    current_size: u8,
}

impl<const MAX_CMDS: usize> SimpleSerial<MAX_CMDS> {
    /// Create a new SimpleSerial Instance
    pub fn new() -> Self {
        unsafe {
            platform_init();
            init_uart();
            trigger_setup();
        }

        Self::new_no_init()
    }

    /// Trigger a high on the trace trigger
    pub fn trace_trigger_high() {
        unsafe {
            firmware::trigger_high();
        }
    }

    /// Trigger a low on the trace trigger
    pub fn trace_trigger_low() {
        unsafe {
            firmware::trigger_low();
        }
    }

    /// Create a new SimpleSerial Instance without initializing the platform, uart and trigger.
    pub fn new_no_init() -> Self {
        SimpleSerial {
            cmds: [None; MAX_CMDS],
            current_size: 0,
        }
    }

    /// Add an extra command handler
    pub fn push(&mut self, cmd: u8, handler: CmdFn) -> Result<(), ()> {
        let cur_size = usize::from(self.current_size);

        // Check that we are not exceeding the command limit
        if MAX_CMDS - cur_size <= 1 {
            unsafe {
                putch(b'a');
            }
            return Err(());
        }

        self.cmds[cur_size] = Some((cmd, handler));
        self.current_size += 1;

        Ok(())
    }

    /// Attempt to handle a command put on the BUS
    pub fn attempt_handle(&self) -> Result<(), PktError> {
        let pkt = CTPacket::fetch()?;
        self.handle(pkt)
    }

    /// Handle all the commands that belong with a packet
    fn handle(&self, packet: CTPacket) -> Result<(), PktError> {
        match packet.cmd {
            // Check version command
            b'v' => {
                TCPacket {
                    cmd: b'r',
                    dlen: 1,
                    data: array_resize([2], 0),
                }
                .send()?;
            }

            // List commands command
            b'w' => {
                let mut cmd_chars: [u8; 192] = [0; 192];
                for i in 0..usize::from(self.current_size) {
                    cmd_chars[i] = packet.data[i];
                }

                TCPacket {
                    cmd: b'r',
                    dlen: self.current_size,
                    data: cmd_chars,
                }
                .send()?;
            }

            // Rest of commands
            _ => {
                for i in 0..usize::from(self.current_size) {
                    let (spec_cmd, spec_handler) = self.cmds[i].unwrap();

                    // If the gotten command is the same as the command in listeners,
                    // go and handle that listener.
                    if packet.cmd == spec_cmd {
                        match spec_handler(packet.sub_cmd, packet.dlen, &packet.data) {
                            Err(err) => {
                                TCPacket {
                                    cmd: b'e',
                                    dlen: 1,
                                    data: array_resize([err.get_byte()], 0),
                                }
                                .send()?;

                                return Ok(())
                            }
                            Ok(Some((cmd, dlen, data))) => {
                                TCPacket { cmd, dlen, data }.send()?;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        TCPacket {
            cmd: b'e',
            dlen: 1,
            data: array_resize([CmdError::OK.get_byte()], 0),
        }.send()?;

        Ok(())
    }
}

type UnstuffedBuffer = [u8; 254];

trait SSPacket {
    const METADATA_BYTES_LENGTH: usize;
}

trait SentPacket: SSPacket {
    fn get_data_length(&self) -> usize;
    fn get_data_bytes(&self) -> [u8; 192];
    fn set_metadata_bytes(&self, buffer: &mut UnstuffedBuffer);
    fn send(&self) -> Result<(), PktError> {
        let data_size = self.get_data_length();

        // Create a buffer with all the data in the packet
        let mut buffer = superimpose([0; 254], self.get_data_bytes(), Self::METADATA_BYTES_LENGTH);
        self.set_metadata_bytes(&mut buffer);
        let length = data_size + Self::METADATA_BYTES_LENGTH;

        let buffer = pkt_insert_crc8(buffer, length);
        let buffer = pkt_stuff(buffer, length);

        // Write the stuffed buffer away to the BUS
        write_away(buffer);
        Ok(())
    }
}

trait ReceivedPacket: Sized + SSPacket {
    // PTR, CRC, NULL
    const STUFFED_LOWER_BOUND: usize = Self::METADATA_BYTES_LENGTH + 3;

    // CRC
    const UNSTUFFED_LENGTH_MIN_DATA: usize = Self::METADATA_BYTES_LENGTH + 1;

    fn get_data_length_from_unstuffed(unstuffed_buffer: UnstuffedBuffer) -> usize;
    fn new_from_unstuffed(unstuffed_buffer: UnstuffedBuffer) -> Self;
    fn fetch() -> Result<Self, PktError> {
        // Read BUS into buffer
        let (buffer, length) = read_away::<256>();
        // Verify a lower bound of the read buffer
        if length < Self::STUFFED_LOWER_BOUND {
            return Err(PktError::InsufficientBytes {
                buffer_length: length,
            });
        }

        // Unstuff the buffer
        let (unstuffed, unstuffed_length) = cobs_rs::unstuff(buffer, 0x00);
        // Verify the CRC correctness
        if !crc8_rs::has_valid_crc8(
            drift_to_end(unstuffed, unstuffed_length, 0, 0),
            CRC_GEN_POLY,
        ) {
            return Err(PktError::CrcInvalid);
        }

        // Fetch the data length
        let data_length = Self::get_data_length_from_unstuffed(unstuffed);
        // Now verify the exact length of the buffer
        if unstuffed_length - data_length != Self::UNSTUFFED_LENGTH_MIN_DATA {
            return Err(PktError::IncorrectDataLength {
                buffer_length: unstuffed_length,
                data_length,
            });
        }

        // Form a packet
        Ok(Self::new_from_unstuffed(unstuffed))
    }
}
