//! Escritura de imágenes byte a byte, sin librerías: PPM (P6) y BMP de 24 bits.

use std::fs::File;
use std::io::{BufWriter, Result, Write};
use std::path::Path;

/// PPM binario: cabecera de texto + RGB crudo. ffmpeg lo lee directamente.
pub fn write_ppm(path: &Path, width: usize, height: usize, rgb: &[u8]) -> Result<()> {
    let mut w = BufWriter::new(File::create(path)?);
    write!(w, "P6\n{} {}\n255\n", width, height)?;
    w.write_all(rgb)?;
    w.flush()
}

/// BMP de 24 bits (se abre con doble clic en Windows).
/// Filas de abajo hacia arriba, en orden BGR y rellenadas a múltiplos de 4 bytes.
pub fn write_bmp(path: &Path, width: usize, height: usize, rgb: &[u8]) -> Result<()> {
    let row_size = (width * 3 + 3) & !3;
    let image_size = row_size * height;
    let file_size = 54 + image_size;
    let mut w = BufWriter::new(File::create(path)?);

    // BITMAPFILEHEADER (14 bytes)
    w.write_all(b"BM")?;
    w.write_all(&(file_size as u32).to_le_bytes())?;
    w.write_all(&0u32.to_le_bytes())?;
    w.write_all(&54u32.to_le_bytes())?;
    // BITMAPINFOHEADER (40 bytes)
    w.write_all(&40u32.to_le_bytes())?;
    w.write_all(&(width as i32).to_le_bytes())?;
    w.write_all(&(height as i32).to_le_bytes())?;
    w.write_all(&1u16.to_le_bytes())?; // planos
    w.write_all(&24u16.to_le_bytes())?; // bits por píxel
    w.write_all(&0u32.to_le_bytes())?; // sin compresión
    w.write_all(&(image_size as u32).to_le_bytes())?;
    w.write_all(&2835u32.to_le_bytes())?; // 72 DPI
    w.write_all(&2835u32.to_le_bytes())?;
    w.write_all(&0u32.to_le_bytes())?;
    w.write_all(&0u32.to_le_bytes())?;

    let padding = [0u8; 3];
    for y in (0..height).rev() {
        for x in 0..width {
            let i = (y * width + x) * 3;
            w.write_all(&[rgb[i + 2], rgb[i + 1], rgb[i]])?;
        }
        w.write_all(&padding[..row_size - width * 3])?;
    }
    w.flush()
}

/// Elige el formato por la extensión del archivo
pub fn save(path: &Path, width: usize, height: usize, rgb: &[u8]) -> Result<()> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("bmp") => write_bmp(path, width, height, rgb),
        _ => write_ppm(path, width, height, rgb),
    }
}
