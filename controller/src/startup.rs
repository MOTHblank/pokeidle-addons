//! Windows per-user startup registration, only changed by explicit user action.
#[cfg(windows)]
pub fn set_start_with_windows(enabled: bool) -> Result<(), String> {
    use std::ffi::c_void;

    type HKey = *mut c_void;
    const HKEY_CURRENT_USER: HKey = 0x8000_0001u32 as usize as HKey;
    const KEY_SET_VALUE: u32 = 0x0002;
    const REG_SZ: u32 = 1;
    const ERROR_SUCCESS: i32 = 0;
    const ERROR_FILE_NOT_FOUND: i32 = 2;
    const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "MothController";

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegCreateKeyExW(
            key: HKey,
            sub_key: *const u16,
            reserved: u32,
            class: *const u16,
            options: u32,
            desired: u32,
            security_attributes: *const c_void,
            result: *mut HKey,
            disposition: *mut u32,
        ) -> i32;
        fn RegOpenKeyExW(
            key: HKey,
            sub_key: *const u16,
            options: u32,
            desired: u32,
            result: *mut HKey,
        ) -> i32;
        fn RegSetValueExW(
            key: HKey,
            value_name: *const u16,
            reserved: u32,
            kind: u32,
            data: *const u8,
            data_bytes: u32,
        ) -> i32;
        fn RegDeleteValueW(key: HKey, value_name: *const u16) -> i32;
        fn RegCloseKey(key: HKey) -> i32;
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let subkey = wide(RUN_SUBKEY);
    let value_name = wide(VALUE_NAME);

    if enabled {
        let executable = std::env::current_exe()
            .map_err(|error| format!("could not locate controller executable: {error}"))?;
        let quoted_executable = wide(&format!("\"{}\"", executable.display()));
        let mut handle: HKey = std::ptr::null_mut();
        let open_status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                std::ptr::null(),
                0,
                KEY_SET_VALUE,
                std::ptr::null(),
                &mut handle,
                std::ptr::null_mut(),
            )
        };
        if open_status != ERROR_SUCCESS {
            return Err(format!("could not configure Windows startup: error {open_status}"));
        }
        let byte_len = u32::try_from(quoted_executable.len() * 2).unwrap_or(u32::MAX);
        let write_status = unsafe {
            RegSetValueExW(
                handle,
                value_name.as_ptr(),
                0,
                REG_SZ,
                quoted_executable.as_ptr() as *const u8,
                byte_len,
            )
        };
        unsafe { RegCloseKey(handle) };
        if write_status != ERROR_SUCCESS {
            return Err(format!("Windows startup registration failed: error {write_status}"));
        }
        Ok(())
    } else {
        let mut handle: HKey = std::ptr::null_mut();
        let open_status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                KEY_SET_VALUE,
                &mut handle,
            )
        };
        if open_status == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        if open_status != ERROR_SUCCESS {
            return Err(format!("could not configure Windows startup: error {open_status}"));
        }
        let delete_status = unsafe { RegDeleteValueW(handle, value_name.as_ptr()) };
        unsafe { RegCloseKey(handle) };
        if delete_status != ERROR_SUCCESS && delete_status != ERROR_FILE_NOT_FOUND {
            return Err(format!("Windows startup removal failed: error {delete_status}"));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
pub fn set_start_with_windows(_enabled: bool) -> Result<(), String> {
    Err("Windows startup registration is not available on this platform".into())
}
