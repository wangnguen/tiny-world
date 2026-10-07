//! "Chạy cùng Windows": ghi đường dẫn exe đang chạy vào
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`. Chỉ cần quyền user, không cần plugin, bản
//! cài hay bản portable đều trỏ đúng exe. Chuyển exe portable sang chỗ khác thì coi như đang tắt.

use crate::error::AppResult;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "TinyWorld";

pub fn is_enabled() -> AppResult<bool> {
    let expected = command()?;
    Ok(registry::read(RUN_KEY, VALUE_NAME)?.is_some_and(|value| value.eq_ignore_ascii_case(&expected)))
}

pub fn set_enabled(enabled: bool) -> AppResult<()> {
    if enabled {
        registry::write(RUN_KEY, VALUE_NAME, &command()?)
    } else {
        registry::delete(RUN_KEY, VALUE_NAME)
    }
}

/// Lệnh Windows chạy khi đăng nhập: đường dẫn exe trong ngoặc kép (đường dẫn có dấu cách).
fn command() -> AppResult<String> {
    Ok(format!("\"{}\"", std::env::current_exe()?.display()))
}

/// Đọc/ghi một giá trị chuỗi trong `HKEY_CURRENT_USER`.
#[cfg(windows)]
mod registry {
    use crate::error::{AppError, AppResult};
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
    use windows_sys::Win32::System::Registry::{
        RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// `key`: "errors.registryRead", "errors.registryWrite" hay "errors.registryDelete".
    fn fail(key: &str, status: WIN32_ERROR) -> AppError {
        AppError::internal(crate::i18n::tf(key, &[("status", &status)]))
    }

    pub fn read(key: &str, name: &str) -> AppResult<Option<String>> {
        let (key, name) = (wide(key), wide(name));
        let mut buffer = vec![0u16; 2048];
        let mut bytes = (buffer.len() * 2) as u32;
        // SAFETY: key/name là chuỗi UTF-16 kết thúc bằng 0; `bytes` là kích thước thật của buffer.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                null_mut(),
                buffer.as_mut_ptr().cast(),
                &mut bytes,
            )
        };
        match status {
            ERROR_SUCCESS => {
                // `bytes` tính cả ký tự 0 ở cuối.
                let len = (bytes as usize / 2).saturating_sub(1);
                Ok(Some(String::from_utf16_lossy(&buffer[..len])))
            }
            ERROR_FILE_NOT_FOUND => Ok(None),
            status => Err(fail("errors.registryRead", status)),
        }
    }

    pub fn write(key: &str, name: &str, value: &str) -> AppResult<()> {
        let (key, name, value) = (wide(key), wide(name), wide(value));
        // SAFETY: các chuỗi UTF-16 kết thúc bằng 0, `cbdata` là số byte của `value` tính cả số 0.
        let status = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                REG_SZ,
                value.as_ptr().cast(),
                (value.len() * 2) as u32,
            )
        };
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(fail("errors.registryWrite", status))
        }
    }

    /// Chưa có giá trị thì coi như đã xoá.
    pub fn delete(key: &str, name: &str) -> AppResult<()> {
        let (key, name) = (wide(key), wide(name));
        // SAFETY: key/name là chuỗi UTF-16 kết thúc bằng 0.
        let status = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) };
        match status {
            ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
            status => Err(fail("errors.registryDelete", status)),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use windows_sys::Win32::System::Registry::RegDeleteKeyW;

        #[test]
        fn ghi_doc_xoa_gia_tri() {
            // Khoá riêng cho test, không đụng vào khoá Run thật.
            let key = format!(r"Software\TinyWorldTest-{}", std::process::id());
            assert_eq!(read(&key, "app").unwrap(), None);
            write(&key, "app", r#""C:\Program Files\TinyWorld\tinyworld.exe""#).unwrap();
            assert_eq!(
                read(&key, "app").unwrap().as_deref(),
                Some(r#""C:\Program Files\TinyWorld\tinyworld.exe""#)
            );
            delete(&key, "app").unwrap();
            delete(&key, "app").unwrap();
            assert_eq!(read(&key, "app").unwrap(), None);
            let wide_key = wide(&key);
            // SAFETY: chuỗi UTF-16 kết thúc bằng 0, khoá vừa tạo ở trên và đã rỗng.
            unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, wide_key.as_ptr()) };
        }
    }
}

#[cfg(not(windows))]
mod registry {
    use crate::error::{AppError, AppResult};

    pub fn read(_key: &str, _name: &str) -> AppResult<Option<String>> {
        Ok(None)
    }

    pub fn write(_key: &str, _name: &str, _value: &str) -> AppResult<()> {
        Err(AppError::bad_request("Chỉ hỗ trợ Windows."))
    }

    pub fn delete(_key: &str, _name: &str) -> AppResult<()> {
        Ok(())
    }
}
