use cobs_rs::{stuff, unstuff};
use core::convert::TryInto;
use crc8_rs::{fetch_crc8, has_valid_crc8, insert_crc8};

struct CTPacket {
    cmd: u8,
    sub_cmd: u8,
    dlen: u8,
    data: [u8; 248],
}

struct TCPacket {
    cmd: u8,
    dlen: u8,
    data: [u8; 252],
}

type CmdFn = &'static dyn Fn(u8, u8, &[u8]) -> u8;
type CmdSpecification = (u8, CmdFn);

pub struct SimpleSerial<const MAX_CMDS: usize> {
    cmds: [Option<CmdSpecification>; MAX_CMDS],
    current_size: usize,
}

impl<const MAX_CMDS: usize> SimpleSerial<MAX_CMDS> {
    pub fn new() -> Self {
        let mut ss = Self {
            cmds: [None; MAX_CMDS],
            current_size: 2,
        };

        // Set `Fetch version` command
        ss.cmds[0] = Some((b'v', &|_, _, _| TCPacket::new(b'r', [2]).send()));
        // Set `List commands` command
        ss.cmds[0] = Some((b'w', &|_, _, _| ss.list_cmds()));

        ss
    }

    /// Add an extra command handler
    pub fn push(&mut self, cmd: u8, handler: CmdFn) -> Result<(), ()> {
        // Check that we are not exceeding the command limit
        if MAX_CMDS - self.current_size <= 1 {
            putch(b'a');
            return Err(());
        }

        self.cmds[self.current_size] = Some((cmd, handler));
        self.current_size += 1;

        Ok(())
    }

    pub fn attempt_handle(&self) -> Result<(), ()> {
        let pkt = CTPacket::try_getch()?;
        self.handle(pkt); 
    }

    /// Handle all the commands that belong with a packet
    fn handle(&self, packet: CTPacket) {
        for i in 0..self.current_size {
            let (spec_cmd, spec_handler) = self.cmds[i].unwrap();

            // If the gotten command is the same as the command in listeners,
            // go and handle that listener.
            if packet.cmd == spec_cmd {
                spec_handler(packet.sub_cmd, packet.dlen, &packet.data);
            }
        }
    }

    fn list_cmds(&self) -> u8 {
        let cmd_chars: [u8; MAX_CMDS] = [0; MAX_CMDS];
        for i in 0..self.current_size {
            cmd_chars[i] = self.cmds[i].unwrap().0;
        }

        TCPacket::new(b'r', cmd_chars[..self.current_size].try_into().unwrap()).send()
    }
}

const CRC_GEN_POLY: u8 = 0xA6;

impl CTPacket {
    fn try_getch() -> Result<Self, ()> {
        let mut byte_array = [0; 256];

        // 0: PTR
        // 1: CMD
        // 2: SCMD
        // 3: LEN
        for i in 0..4 {
            byte_array[i] = getch();
            if byte_array[i] == 0x00 {
                return Err(());
            }
        }

        // Fetch Data + CRC
        let mut index = 0;
        loop {
            let token = getch();
            if token == 0x00 {
                break;
            }

            byte_array[4 + index] = token;

            index += 1;
        }
        
        // Unstuff the buffer
        let (unstuffed, buf_len) = unstuff(byte_array, 0x00);

        // Verify the CRC correctness
        if !has_valid_crc8(unstuffed[..buf_len].try_into().unwrap(), CRC_GEN_POLY) {
            // Invalid CRC
            return Err(());
        }

        // Filter out a data buffer
        let mut data = [0; 248];
        for i in 4..buf_len-4 {
            data[i] = unstuffed[i];
        }

        // Form a packet
        let pkt = CTPacket {
            cmd: unstuffed[0],
            sub_cmd: unstuffed[1],
            dlen: unstuffed[2],
            data
        };

        // Verify that the length given is equal to the data length
        if pkt.dlen == data.len() {
            Ok(pkt)
        } else {
            Err(())
        }
    }
}

impl<const DLEN: usize> TCPacket<DLEN> {
    pub fn new(cmd: u8, data: [u8; DLEN]) -> Self {
        // Compile-time check the DLEN bounds, this shouldn't be higher than 249
        let _ = [(); 250][DLEN];

        let mut pkt = Self { cmd, data, crc: 0 };

        pkt.crc = CRC_GEN_POLY ^ fetch_crc8(&pkt.to_byte_array(), CRC_GEN_POLY);

        pkt
    }

    fn to_byte_array(&self) -> [u8; 256] {
        let mut byte_array = [0; 256];

        byte_array[0] = self.cmd;
        byte_array[1] = self.sub_cmd;
        byte_array[2] = self.dlen;
        for i in 0..self.dlen {
            byte_array[3 + i] = self.data[i];
        }
        
        // Convert the byte array into the actual crc-ed size
        byte_array = byte_array[..self.dlen + 4].try_into().unwrap();

        // We need to insert the correct CRC and stuff the data
        stuff(insert_crc8(byte_array, CRC_GEN_POLY), 0x00)
    }
    pub fn send(&self) -> u8 {
        unimplemented!()
    }
}

impl<const DLEN: usize> SimpleSerialPacket for TCPacket<DLEN> {
    fn to_byte_array(&self) -> [u8; 256] {
        let _ = [(); 250][DLEN];
        let mut byte_array = [0; 256];

        byte_array[255 - DLEN - 2] = self.cmd;
        byte_array[255 - DLEN - 1] = DLEN.try_into().unwrap();
        for i in 0..DLEN {
            byte_array[255 - DLEN + i] = self.data[i];
        }
        byte_array[255] = self.crc;

        byte_array
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod simpleserial_struct {
        use super::*;

        #[test]
        fn handle_cmds() {
            let mut ss: SimpleSerial<256> = SimpleSerial::new();
            ss.push(b'a', |sub_cmd, _, _| {
                assert!(true);
                sub_cmd
            });

            ss.handle(CTPacket::new(b'a', b'b', []));
        }

        #[test]
        #[should_panic]
        fn handle_panicking_cmd() {
            let mut ss: SimpleSerial<256> = SimpleSerial::new();
            ss.push(b'a', |sub_cmd, _, _| {
                assert!(false);
                sub_cmd
            });

            ss.handle(CTPacket::new(b'a', b'b', []));
        }
    }

    mod ct_packet {
        use super::*;
        #[test]
        fn try_getch() {
            [0x02, b'p', b'n', 0x01,
        }
    }
}

