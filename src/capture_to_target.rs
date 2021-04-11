use crate::UnstuffedBuffer;
use crate::{ReceivedPacket, SSPacket};
use array_utils::sized_slice;

/// Capture to target board packet
#[cfg_attr(test, derive(Debug, PartialEq, Clone))]
pub struct CTPacket {
    pub cmd: u8,
    pub sub_cmd: u8,
    pub dlen: u8,
    pub data: [u8; 192],
}

impl SSPacket for CTPacket {
    // CMD, SCMD, DLEN
    const METADATA_BYTES_LENGTH: usize = 3;
}

impl ReceivedPacket for CTPacket {
    fn get_data_length_from_unstuffed(unstuffed_buffer: UnstuffedBuffer) -> usize {
        usize::from(unstuffed_buffer[2])
    }
    fn new_from_unstuffed(unstuffed_buffer: UnstuffedBuffer) -> Self {
        CTPacket {
            cmd: unstuffed_buffer[0],
            sub_cmd: unstuffed_buffer[1],
            dlen: unstuffed_buffer[2],
            data: sized_slice(unstuffed_buffer, 3, usize::from(unstuffed_buffer[2]) + 3, 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::SentPacket;

    impl SentPacket for CTPacket {
        fn get_data_length(&self) -> usize {
            usize::from(self.dlen)
        }
        fn get_data_bytes(&self) -> [u8; 192] {
            self.data
        }
        fn set_metadata_bytes(&self, buffer: &mut UnstuffedBuffer) {
            buffer[0] = self.cmd;
            buffer[1] = self.sub_cmd;
            buffer[2] = self.dlen;
        }
    }

    use super::*;
    use array_utils::*;

    #[test]
    fn invertible() {
        let pkt = CTPacket {
            cmd: b'p',
            sub_cmd: 0,
            dlen: 4,
            data: array_resize([5, 4, 2, 1], 0),
        };
        assert_eq!(pkt, {
            pkt.send().unwrap();
            CTPacket::fetch().unwrap()
        });
    }
}
