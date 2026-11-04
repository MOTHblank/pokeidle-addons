use std::io::{self, Write};
use std::path::Path;

pub const DEFAULT_ICON_SIZE: u32 = 256;

pub fn rgba(size: u32) -> Vec<u8> {
    assert!(size > 0);
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let sx = (x * 32 / size) as i32;
            let sy = (y * 32 / size) as i32;
            let dx = sx - 16;
            let dy = sy - 16;
            let in_badge = dx * dx + dy * dy <= 225;

            let upper_left = ((sx as f32 - 10.0) / 6.0).powi(2)
                + ((sy as f32 - 12.0) / 5.0).powi(2)
                <= 1.0;
            let upper_right = ((sx as f32 - 22.0) / 6.0).powi(2)
                + ((sy as f32 - 12.0) / 5.0).powi(2)
                <= 1.0;
            let lower_left = ((sx as f32 - 11.0) / 5.0).powi(2)
                + ((sy as f32 - 20.0) / 4.0).powi(2)
                <= 1.0;
            let lower_right = ((sx as f32 - 21.0) / 5.0).powi(2)
                + ((sy as f32 - 20.0) / 4.0).powi(2)
                <= 1.0;
            let body = (15..=17).contains(&sx) && (8..=24).contains(&sy);

            let color = if body {
                [245, 242, 255, 255]
            } else if upper_left || upper_right {
                [123, 97, 255, 255]
            } else if lower_left || lower_right {
                [76, 160, 255, 255]
            } else if in_badge {
                [18, 21, 33, 255]
            } else {
                [0, 0, 0, 0]
            };

            let offset = ((y * size + x) * 4) as usize;
            rgba[offset..offset + 4].copy_from_slice(&color);
        }
    }

    rgba
}

// Also compiled into build.rs, which calls this on Windows. The runtime
// binary includes this module for RGBA icons but does not write ICO files.
#[allow(dead_code)]
pub fn write_ico(path: &Path, size: u32) -> io::Result<()> {
    if !(1..=256).contains(&size) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ICO size must be between 1 and 256 pixels",
        ));
    }

    let rgba = rgba(size);
    let mask_stride = size.div_ceil(32) * 4;
    let mask_size = mask_stride * size;
    let dib_size = 40 + size * size * 4 + mask_size;
    let mut file = std::fs::File::create(path)?;

    file.write_all(&0u16.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;

    let dimension = if size == 256 { 0 } else { size as u8 };
    file.write_all(&[dimension, dimension, 0, 0])?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&32u16.to_le_bytes())?;
    file.write_all(&dib_size.to_le_bytes())?;
    file.write_all(&(6u32 + 16).to_le_bytes())?;

    file.write_all(&40u32.to_le_bytes())?;
    file.write_all(&(size as i32).to_le_bytes())?;
    file.write_all(&((size * 2) as i32).to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&32u16.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;
    file.write_all(&(size * size * 4).to_le_bytes())?;
    file.write_all(&0i32.to_le_bytes())?;
    file.write_all(&0i32.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;

    for y in (0..size).rev() {
        for x in 0..size {
            let offset = ((y * size + x) * 4) as usize;
            let r = rgba[offset];
            let g = rgba[offset + 1];
            let b = rgba[offset + 2];
            let a = rgba[offset + 3];
            file.write_all(&[b, g, r, a])?;
        }
    }

    file.write_all(&vec![0u8; mask_size as usize])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_has_expected_rgba_shape_and_key_colors() {
        let pixels = rgba(32);
        assert_eq!(pixels.len(), 32 * 32 * 4);

        let pixel = |x: usize, y: usize| {
            let offset = (y * 32 + x) * 4;
            &pixels[offset..offset + 4]
        };

        assert_eq!(pixel(0, 0), &[0, 0, 0, 0]);
        assert_eq!(pixel(16, 16), &[245, 242, 255, 255]);
        assert_eq!(pixel(10, 12), &[123, 97, 255, 255]);
        assert_eq!(pixel(11, 20), &[76, 160, 255, 255]);
        assert_eq!(pixel(16, 29), &[18, 21, 33, 255]);
    }

    #[test]
    fn ico_writer_produces_icon_header() {
        let path = std::env::temp_dir().join(format!(
            "moth-controller-icon-{}.ico",
            std::process::id()
        ));
        write_ico(&path, 32).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(path);

        assert_eq!(&bytes[..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!(bytes[6], 32);
        assert_eq!(bytes[7], 32);
    }
}
