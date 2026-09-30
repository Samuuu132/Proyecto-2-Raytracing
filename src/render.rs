//! Render en paralelo con hilos. La imagen se divide en bandas de filas;
//! cada hilo toma la siguiente banda libre (balanceo dinámico).

use crate::camera::Camera;
use crate::math::Vec3;
use crate::ray::Ray;
use std::sync::Mutex;

const BAND_ROWS: usize = 4;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![255; width * height * 4],
        }
    }

    pub fn to_rgb(&self) -> Vec<u8> {
        self.pixels
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect()
    }
}

pub trait Shade: Sync {
    fn shade(&self, ray: &Ray) -> Vec3;
}

#[inline]
pub fn to_byte(c: f32) -> u8 {
    let mapped = c / (1.0 + c * 0.15); // Reinhard suave
    let gamma = mapped.max(0.0).powf(1.0 / 2.2);
    (gamma.min(1.0) * 255.0 + 0.5) as u8
}

/// `aa` = muestras por lado de cada píxel (1 = sin antialiasing, 2 = 4 muestras)
pub fn render<S: Shade>(scene: &S, cam: &Camera, fb: &mut Framebuffer, threads: usize, aa: usize) {
    let aa = aa.max(1);
    let inv = 1.0 / (aa * aa) as f32;
    let (w, h) = (fb.width, fb.height);
    let row_bytes = w * 4;
    let bands = Mutex::new(fb.pixels.chunks_mut(row_bytes * BAND_ROWS).enumerate());

    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let next = bands.lock().unwrap().next();
                let Some((band, chunk)) = next else { break };
                let y0 = band * BAND_ROWS;
                for (row, line) in chunk.chunks_mut(row_bytes).enumerate() {
                    let y = y0 + row;
                    for x in 0..w {
                        let mut c = Vec3::ZERO;
                        for j in 0..aa {
                            for i in 0..aa {
                                let fx = x as f32 + (i as f32 + 0.5) / aa as f32;
                                let fy = y as f32 + (j as f32 + 0.5) / aa as f32;
                                let sx = 2.0 * fx / w as f32 - 1.0;
                                let sy = 1.0 - 2.0 * fy / h as f32;
                                c += scene.shade(&cam.ray(sx, sy));
                            }
                        }
                        let c = c * inv;
                        let px = &mut line[x * 4..x * 4 + 4];
                        px[0] = to_byte(c.x);
                        px[1] = to_byte(c.y);
                        px[2] = to_byte(c.z);
                        px[3] = 255;
                    }
                }
            });
        }
    });
}