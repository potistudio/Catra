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
    use data_encoding::ASCII85;
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let decoded = ASCII85
        .decode(blob)
        .map_err(|error| format!("failed to decode Rekordbox key blob: {error}"))?;
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

#[cfg(test)]
mod tests {
    use super::master_db_key;

    #[test]
    fn key_starts_with_expected_prefix() {
        let key = master_db_key().expect("key");
        assert!(key.starts_with("402fd"));
    }
}
