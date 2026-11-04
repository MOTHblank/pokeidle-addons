pub const REFERENCE_CLIENT_ID: &str = "ue6666qo983tsx6so1t0vnawi233wa";
pub const DEFAULT_CLIENT_VERSION: &str = "ef928475-9403-42f2-8a34-55784bd08e16";
pub const ANDROID_TV_USER_AGENT: &str =
    "Mozilla/5.0 (Linux; Android 7.1; Smart Box C1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36";
pub const LINUX_FIREFOX_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:85.0) Gecko/20100101 Firefox/85.0";
pub const DEVICE_FLOW_SCOPES: &str =
    "channel_read chat:read user_blocks_edit user_blocks_read user_follows_edit user_read";

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
    fn extracts_twilight_build_id() {
        let page = r#"<script>window.__twilightBuildID = "12345678-abcd";</script>"#;
        assert_eq!(extract_twilight_build_id(page).as_deref(), Some("12345678-abcd"));
    }
}
