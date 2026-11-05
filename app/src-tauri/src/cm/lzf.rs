//! LZF codec compatible with FirstFloor.ModernUI (liblzf).
//!
//! Content Manager writes `Values.data` as `0x0B` followed by an LZF stream.

pub fn decompress(input: &[u8]) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = Vec::new();
    let mut index = 0usize;

    while index < input.len() {
        let control = input[index] as usize;
        index += 1;

        if control < 32 {
            let length = control + 1;
            if index + length > input.len() {
                return Err("LZF: run letterale troncato".into());
            }
            out.extend_from_slice(&input[index..index + length]);
            index += length;
            continue;
        }

        let mut length = control >> 5;
        let mut reference = out.len().wrapping_sub(((control & 0x1f) << 8) + 1);

        if length == 7 {
            if index >= input.len() {
                return Err("LZF: lunghezza troncata".into());
            }
            length += input[index] as usize;
            index += 1;
        }
        if index >= input.len() {
            return Err("LZF: riferimento troncato".into());
        }
        reference = reference.wrapping_sub(input[index] as usize);
        index += 1;

        if reference >= out.len() {
            return Err("LZF: riferimento fuori range".into());
        }

        let end = reference + length + 2;
        for index in reference..end {
            let byte = out[index];
            out.push(byte);
        }
        reference = end;
    }

    Ok(out)
}

/// Emits a valid LZF stream using literal runs only. It does not reduce size,
/// which is enough for round-trips and snapshots; Content Manager itself still
/// reads it without issues.
#[allow(dead_code)]
pub fn compress(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() + input.len() / 32 + 1);
    let mut index = 0usize;
    while index < input.len() {
        let length = (input.len() - index).min(32);
        out.push((length - 1) as u8);
        out.extend_from_slice(&input[index..index + length]);
        index += length;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_then_decompress_roundtrip() {
        let source = b"version: 2\nSettings.DriveSettings.PlayerName\tDriver\n__RaceGrid\t{\"CarIds\":[\"a\",\"b\"]}\n";
        let packed = compress(source);
        let unpacked = decompress(&packed).expect("decompress");
        assert_eq!(unpacked, source);
    }

    #[test]
    fn decompresses_back_reference() {
        // literal "ab", then match: length 4 at distance 2 -> "ababab"
        let packed = [1u8, b'a', b'b', 0x40, 0x01];
        let unpacked = decompress(&packed).expect("decompress");
        assert_eq!(unpacked, b"ababab");
    }
}
