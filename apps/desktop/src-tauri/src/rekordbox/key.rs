//! Rekordbox master.db key extraction (ported from pyrekordbox).

const BLOB_KEY: &[u8] = b"657f48f84c437cc1";
const BLOB: &[u8] = b"PN_Pq^*N>(JYe*u^8;Yg76HuZ<mR13S?=>)b9;DpoTXV(6ItkU`}8*m6tx_I{Solh_N#dfe{v=";

/// RFC 1924 Base85 alphabet (Python `base64.b85decode`).
const B85_ALPHABET: &[u8; 85] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$%&()*+-;<=>?@^_`{|}~";

pub fn master_db_key() -> Result<String, String> {
    let key = deobfuscate(BLOB)?;
    if !key.starts_with("402fd") {
        return Err("extracted Rekordbox database key looks invalid".to_string());
    }
    Ok(key)
}

fn deobfuscate(blob: &[u8]) -> Result<String, String> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let decoded = decode_base85(blob)?;
    let xored: Vec<u8> = decoded
        .iter()
        .enumerate()
        .map(|(index, byte)| byte ^ BLOB_KEY[index % BLOB_KEY.len()])
        .collect();

    let mut decoder = ZlibDecoder::new(xored.as_slice());
    let mut key = String::new();
    decoder
        .read_to_string(&mut key)
        .map_err(|error| format!("failed to decompress Rekordbox key blob: {error}"))?;
    Ok(key)
}

fn decode_base85(input: &[u8]) -> Result<Vec<u8>, String> {
    let mut decode_map = [0xff_u8; 256];
    for (index, &byte) in B85_ALPHABET.iter().enumerate() {
        decode_map[byte as usize] = index as u8;
    }

    let mut output = Vec::with_capacity(input.len() * 4 / 5);
    let mut chunk = [0_u8; 5];
    let mut chunk_len = 0;

    for &byte in input {
        if decode_map[byte as usize] == 0xff {
            return Err(format!("invalid Base85 byte: {byte}"));
        }
        chunk[chunk_len] = byte;
        chunk_len += 1;
        if chunk_len == 5 {
            decode_base85_chunk(&decode_map, &chunk, 5, &mut output)?;
            chunk_len = 0;
        }
    }

    if chunk_len > 0 {
        // Pad short final group with '~' (last alphabet char), matching Python b85decode.
        for slot in chunk.iter_mut().take(5).skip(chunk_len) {
            *slot = b'~';
        }
        decode_base85_chunk(&decode_map, &chunk, chunk_len, &mut output)?;
    }

    Ok(output)
}

fn decode_base85_chunk(
    decode_map: &[u8; 256],
    chunk: &[u8; 5],
    raw_len: usize,
    output: &mut Vec<u8>,
) -> Result<(), String> {
    let mut value: u32 = 0;
    for &byte in chunk {
        let digit = decode_map[byte as usize];
        value = value
            .checked_mul(85)
            .and_then(|next| next.checked_add(u32::from(digit)))
            .ok_or_else(|| "invalid Base85 value".to_string())?;
    }

    let bytes = [
        (value >> 24) as u8,
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
    ];
    let out_len = if raw_len == 5 { 4 } else { raw_len - 1 };
    output.extend_from_slice(&bytes[..out_len]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::master_db_key;

    #[test]
    fn key_starts_with_expected_prefix() {
        let key = master_db_key().expect("key");
        assert!(key.starts_with("402fd"));
    }
}
