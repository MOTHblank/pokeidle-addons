//! Explicit rustls provider initialization for every native TLS entry point.
//! eframe and TLS dependencies may enable different providers transitively,
//! so rustls 0.23 cannot always select one by feature inference.
pub fn ensure() -> Result<(), String> {
    if rustls::crypto::CryptoProvider::get_default().is_some() {
        return Ok(());
    }
    // A simultaneous native worker may have installed the provider between
    // our first check and install_default. If it exists now, that is success.
    let _ = rustls::crypto::ring::default_provider().install_default();
    if rustls::crypto::CryptoProvider::get_default().is_some() {
        Ok(())
    } else {
        Err("Could not initialize rustls cryptographic provider".into())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn provider_initializes_without_panicking() {
        super::ensure().unwrap();
        super::ensure().unwrap();
        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    }
}
