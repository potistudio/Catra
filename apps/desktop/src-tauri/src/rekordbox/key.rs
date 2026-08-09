//! Rekordbox master.db key extraction (ported from pyrekordbox).

const BLOB_KEY: &[u8] = b"657f48f84c437cc1";
const BLOB: &[u8] = b"PN_Pq^*N>(JYe*u^8;Yg76HuZ<mR13S?=>)b9;DpoTXV(6ItkU`}8*m6tx_I{Solh_N#dfe{v=";

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

    let decoded = decode_ascii85(blob)?;
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

fn decode_ascii85(input: &[u8]) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    let mut value: u32 = 0;
    let mut group_len: i32 = 0;

    for &byte in input {
        if matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c') {
            continue;
        }

        if byte == b'z' {
            if group_len != 0 {
                return Err("invalid ASCII85: 'z' inside a group".to_string());
            }
            output.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }

        if !(b'!'..=b'u').contains(&byte) {
            return Err(format!("invalid ASCII85 byte: {byte}"));
        }

        value = value
            .checked_mul(85)
            .and_then(|next| next.checked_add((byte - b'!') as u32))
            .ok_or_else(|| "invalid ASCII85 value".to_string())?;
        group_len += 1;

        if group_len == 5 {
            output.push((value >> 24) as u8);
            output.push((value >> 16) as u8);
            output.push((value >> 8) as u8);
            output.push(value as u8);
            value = 0;
            group_len = 0;
        }
    }

    if group_len > 0 {
        for _ in group_len..5 {
            value = value
                .checked_mul(85)
                .and_then(|next| next.checked_add(84))
                .ok_or_else(|| "invalid ASCII85 value".to_string())?;
        }

        let bytes = group_len.saturating_sub(1);
        output.push((value >> 24) as u8);
        if bytes >= 2 {
            output.push((value >> 16) as u8);
        }
        if bytes >= 3 {
            output.push((value >> 8) as u8);
        }
    }

    Ok(output)
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
