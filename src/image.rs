use std::io::{self, Write};
use std::path::Path;

pub fn save(path: &Path, width: usize, height: usize, rgb: &[u8]) -> io::Result<()> {
    let row_bytes = width * 3;
    let padding = (4 - (row_bytes % 4)) % 4;
    let padded_row = row_bytes + padding;
    let pixel_data_size = padded_row * height;
    let file_size = 54 + pixel_data_size;

    let mut f = std::fs::File::create(path)?;

    // BMP file header (14 bytes)
    f.write_all(b"BM")?;
    f.write_all(&(file_size as u32).to_le_bytes())?;
    f.write_all(&0u32.to_le_bytes())?; // reserved
    f.write_all(&54u32.to_le_bytes())?; // pixel data offset

    // DIB header — BITMAPINFOHEADER (40 bytes)
    f.write_all(&40u32.to_le_bytes())?; // header size
    f.write_all(&(width as i32).to_le_bytes())?;
    f.write_all(&(-(height as i32)).to_le_bytes())?; // negative = top-down
    f.write_all(&1u16.to_le_bytes())?; // color planes
    f.write_all(&24u16.to_le_bytes())?; // bits per pixel
    f.write_all(&0u32.to_le_bytes())?; // no compression
    f.write_all(&(pixel_data_size as u32).to_le_bytes())?;
    f.write_all(&2835i32.to_le_bytes())?; // pixels per meter X (~72 dpi)
    f.write_all(&2835i32.to_le_bytes())?; // pixels per meter Y
    f.write_all(&0u32.to_le_bytes())?; // colors in table
    f.write_all(&0u32.to_le_bytes())?; // important colors

    // Pixel data: BMP stores BGR, rows padded to 4 bytes
    let pad = [0u8; 3];
    for row in rgb.chunks_exact(row_bytes) {
        for px in row.chunks_exact(3) {
            f.write_all(&[px[2], px[1], px[0]])?; // RGB -> BGR
        }
        f.write_all(&pad[..padding])?;
    }

    Ok(())
}