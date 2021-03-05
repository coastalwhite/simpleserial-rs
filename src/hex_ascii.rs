/// Turn a num from 0 - 15 to its ascii representative
fn num_to_hex_ascii(num: &u8) -> Option<u8> {
    match num {
        0..=9 => Some(num + b'0'),
        10..=15 => Some(num + b'a' - 10),
        _ => None,
    }
}

/// Turns a byte into its hex ascii form
///
/// e.g.
/// ```text
/// 0xaf => b'af'
/// 0x1b => b'1b'
/// ```
pub fn byte_to_hex_ascii(byte: &u8) -> [u8; 2] {
    let right_hand = byte % 16;
    let left_hand = byte / 16;

    [
        num_to_hex_ascii(&left_hand)
            .expect("Tried to convert byte to hex, left side is bigger than 15?"),
        num_to_hex_ascii(&right_hand)
            .expect("Tried to convert byte to hex, right side is bigger than 15?"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_num_to_hex_ascii() {
        assert_eq!(num_to_hex_ascii(&0), Some(b'0'));

        assert_eq!(num_to_hex_ascii(&10), Some(b'a'));
        assert_eq!(num_to_hex_ascii(&6), Some(b'6'));

        assert_eq!(num_to_hex_ascii(&15), Some(b'f'));

        assert_eq!(num_to_hex_ascii(&100), None);
    }

    #[test]
    fn test_byte_to_hex_ascii() {
        assert_eq!(byte_to_hex_ascii(&0x00), [b'0', b'0']);

        assert_eq!(byte_to_hex_ascii(&0xaf), [b'a', b'f']);
        assert_eq!(byte_to_hex_ascii(&0x1c), [b'1', b'c']);

        assert_eq!(byte_to_hex_ascii(&0xff), [b'f', b'f']);
    }
}
