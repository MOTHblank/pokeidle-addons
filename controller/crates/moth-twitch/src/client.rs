pub const REFERENCE_CLIENT_ID: &str = "ue6666qo983tsx6so1t0vnawi233wa";
pub const DEFAULT_CLIENT_VERSION: &str = "ef928475-9403-42f2-8a34-55784bd08e16";
pub const ANDROID_TV_USER_AGENT: &str =
    "Mozilla/5.0 (Linux; Android 7.1; Smart Box C1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36";
pub const LINUX_FIREFOX_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:85.0) Gecko/20100101 Firefox/85.0";
pub const DEVICE_FLOW_SCOPES: &str =
    "channel_read chat:read user_blocks_edit user_blocks_read user_follows_edit user_read";

const REFERENCE_CREDENTIAL_PREFIX:&str="mothref1";

pub fn encode_reference_credential(token:&str,device_id:&str)->Result<String,String> {
    let token=token.trim();
    if token.is_empty() || token.len()>2048
        || token.bytes().any(|byte|byte.is_ascii_whitespace()||byte.is_ascii_control()) {
        return Err("Invalid Twitch OAuth token".into());
    }
    if device_id.len()!=32 || !device_id.bytes().all(|byte|byte.is_ascii_alphanumeric()) {
        return Err("Invalid Twitch device id".into());
    }
    Ok(format!("{REFERENCE_CREDENTIAL_PREFIX}:{device_id}:{token}"))
}

pub(crate) fn decode_reference_credential(value:&str)->Result<(&str,Option<&str>),String> {
    let value=value.trim();
    if !value.starts_with("mothref") {
        return Ok((value,None));
    }
    let mut parts=value.splitn(3,':');
    if parts.next()!=Some(REFERENCE_CREDENTIAL_PREFIX) {
        return Err("Unsupported stored Twitch credential version".into());
    }
    let device_id=parts.next().ok_or_else(||"Stored Twitch credential is incomplete".to_owned())?;
    let token=parts.next().ok_or_else(||"Stored Twitch credential is incomplete".to_owned())?;
    if device_id.len()!=32 || !device_id.bytes().all(|byte|byte.is_ascii_alphanumeric()) {
        return Err("Stored Twitch device id is invalid".into());
    }
    if token.is_empty() || token.len()>2048
        || token.bytes().any(|byte|byte.is_ascii_whitespace()||byte.is_ascii_control()) {
        return Err("Stored Twitch OAuth token is invalid".into());
    }
    Ok((token,Some(device_id)))
}

pub fn random_string(length: usize) -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut bytes=vec![0u8;length];
    if getrandom::fill(&mut bytes).is_err() {
        // Device/session identifiers are not secrets. Preserve a usable
        // fallback only for platforms where OS randomness is unavailable.
        let seed=std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        for (index,byte) in bytes.iter_mut().enumerate() {
            *byte=(seed.rotate_left((index%128) as u32) as u8)
                .wrapping_add(index as u8);
        }
    }
    bytes.into_iter().map(|byte|CHARSET[byte as usize%CHARSET.len()] as char).collect()
}

pub fn random_hex(byte_length: usize) -> String {
    use std::fmt::Write as _;
    let mut bytes=vec![0u8;byte_length];
    if getrandom::fill(&mut bytes).is_err() {
        let fallback=random_string(byte_length);
        bytes.copy_from_slice(fallback.as_bytes());
    }
    let mut output=String::with_capacity(byte_length.saturating_mul(2));
    for byte in bytes {
        let _=write!(output,"{byte:02x}");
    }
    output
}

pub(crate) fn random_u64_inclusive(min:u64,max:u64)->u64 {
    if max<=min {return min;}
    let mut bytes=[0u8;8];
    if getrandom::fill(&mut bytes).is_err() {
        return min;
    }
    let value=u64::from_le_bytes(bytes);
    min+value%(max-min+1)
}

pub(crate) fn extract_twilight_build_id(page: &str) -> Option<String> {
    let marker = "window.__twilightBuildID";
    let tail = page.get(page.find(marker)? + marker.len()..)?;
    let tail = tail.get(tail.find('=')? + 1..)?.trim_start();
    let quote = tail.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &tail[quote.len_utf8()..];
    let end = rest.find(quote)?;
    let value = rest[..end].trim();
    (!value.is_empty() && value.len() <= 128).then(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_client_identity_has_reference_shapes() {
        let device = random_string(32);
        let session = random_hex(8);
        assert_eq!(device.len(), 32);
        assert!(device.bytes().all(|byte| byte.is_ascii_alphanumeric()));
        assert_eq!(session.len(), 16);
        assert!(session.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }

    #[test]
    fn reference_credential_round_trips_device_id() {
        let encoded=encode_reference_credential(
            "oauth-token",
            "abcdefghijklmnopqrstuvwxyzABCDEF",
        ).unwrap();
        let (token,device)=decode_reference_credential(&encoded).unwrap();
        assert_eq!(token,"oauth-token");
        assert_eq!(device,Some("abcdefghijklmnopqrstuvwxyzABCDEF"));
        assert_eq!(decode_reference_credential("legacy-token").unwrap(),
            ("legacy-token",None));
    }

    #[test]
    fn extracts_twilight_build_id() {
        let page = r#"<script>window.__twilightBuildID = "12345678-abcd";</script>"#;
        assert_eq!(extract_twilight_build_id(page).as_deref(), Some("12345678-abcd"));
    }
}
