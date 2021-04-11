use crate::{SSPacket, SentPacket, UnstuffedBuffer};

/// Target to Capture board packet
#[cfg_attr(test, derive(Debug, PartialEq, Clone))]
pub struct TCPacket {
    pub cmd: u8,
    pub dlen: u8,
    pub data: [u8; 192],
}

impl SSPacket for TCPacket {
    // CMD, DLEN
    const METADATA_BYTES_LENGTH: usize = 2;
}

impl SentPacket for TCPacket {
    fn get_data_length(&self) -> usize {
        usize::from(self.dlen)
    }
    fn get_data_bytes(&self) -> [u8; 192] {
        self.data
    }
    fn set_metadata_bytes(&self, buffer: &mut UnstuffedBuffer) {
        buffer[0] = self.cmd;
        buffer[1] = self.dlen;
    }
}

#[cfg(test)]
mod tests {
    use crate::ReceivedPacket;

    impl ReceivedPacket for TCPacket {
        fn get_data_length_from_unstuffed(unstuffed_buffer: UnstuffedBuffer) -> usize {
            usize::from(unstuffed_buffer[1])
        }
        fn new_from_unstuffed(unstuffed_buffer: UnstuffedBuffer) -> Self {
            TCPacket {
                cmd: unstuffed_buffer[0],
                dlen: unstuffed_buffer[1],
                data: array_utils::sized_slice(
                    unstuffed_buffer,
                    2,
                    usize::from(unstuffed_buffer[1]) + 2,
                    0,
                ),
            }
        }
    }

    use super::*;
    use array_utils::*;

    #[test]
    fn invertible() {
        let pkt = TCPacket {
            cmd: b'p',
            dlen: 4,
            data: array_resize([5, 4, 2, 1], 0),
        };
        assert_eq!(pkt, {
            pkt.send().unwrap();
            TCPacket::fetch().unwrap()
        });
    }
}
