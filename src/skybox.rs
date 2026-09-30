//! Skybox de atardecer como cube map de 6 caras.
//!
//! Las 6 caras se "pintan" una sola vez al inicio evaluando un cielo
//! procedural (gradiente crepuscular + nubes con ruido de Perlin 3D). En el
//! render, cuando un rayo no toca ningún bloque (ray miss), solo se hace un
//! muestreo bilineal de la cara correspondiente: muy barato por píxel.

use crate::math::{smoothstep, srgb, Vec3};
use crate::perlin::fbm3;

pub struct Skybox {
    res: usize,
    faces: Vec<Vec<Vec3>>,
    sun_dir: Vec3,
    sun_color: Vec3,
    ambient_sky: Vec3,
    ambient_ground: Vec3,
}

impl Skybox {
    pub fn new(res: usize, sun_dir: Vec3, sun_color: Vec3) -> Self {
        let mut faces = Vec::with_capacity(6);
        for f in 0..6 {
            let mut data = Vec::with_capacity(res * res);
            for j in 0..res {
                for i in 0..res {
                    let u = (i as f32 + 0.5) / res as f32;
                    let v = (j as f32 + 0.5) / res as f32;
                    data.push(procedural_sky(face_dir(f, u, v).normalize(), sun_dir));
                }
            }
            faces.push(data);
        }
        Self {
            res,
            faces,
            sun_dir,
            sun_color,
            ambient_sky: srgb(0.62, 0.70, 0.95) * 0.55,
            ambient_ground: srgb(0.60, 0.50, 0.58) * 0.28,
        }
    }

    /// Muestreo bilineal del cube map
    pub fn sample(&self, d: Vec3) -> Vec3 {
        let (f, u, v) = dir_to_face(d);
        let n = self.res as f32;
        let x = (u * n - 0.5).clamp(0.0, n - 1.0);
        let y = (v * n - 0.5).clamp(0.0, n - 1.0);
        let (x0, y0) = (x as usize, y as usize);
        let (x1, y1) = ((x0 + 1).min(self.res - 1), (y0 + 1).min(self.res - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let face = &self.faces[f];
        let p = |xx: usize, yy: usize| face[yy * self.res + xx];
        let top = Vec3::lerp(p(x0, y0), p(x1, y0), fx);
        let bottom = Vec3::lerp(p(x0, y1), p(x1, y1), fx);
        Vec3::lerp(top, bottom, fy)
    }

    /// Color de fondo: cube map + disco solar nítido (analítico)
    pub fn background(&self, d: Vec3) -> Vec3 {
        let mut c = self.sample(d);
        let cos = d.dot(self.sun_dir);
        if cos > 0.9994 {
            c += self.sun_color * 6.0;
        }
        c += self.sun_color * (cos.max(0.0).powf(300.0) * 0.8 + cos.max(0.0).powf(12.0) * 0.08);
        c
    }

    /// Luz ambiental del cielo según hacia dónde mira la normal
    #[inline]
    pub fn ambient(&self, n: Vec3) -> Vec3 {
        Vec3::lerp(self.ambient_ground, self.ambient_sky, n.y * 0.5 + 0.5)
    }
}

/// Dirección 3D -> (cara, u, v) del cube map
fn dir_to_face(d: Vec3) -> (usize, f32, f32) {
    let (ax, ay, az) = (d.x.abs(), d.y.abs(), d.z.abs());
    let (f, sc, tc, ma) = if ax >= ay && ax >= az {
        if d.x > 0.0 {
            (0, -d.z, -d.y, ax)
        } else {
            (1, d.z, -d.y, ax)
        }
    } else if ay >= az {
        if d.y > 0.0 {
            (2, d.x, d.z, ay)
        } else {
            (3, d.x, -d.z, ay)
        }
    } else if d.z > 0.0 {
        (4, d.x, -d.y, az)
    } else {
        (5, -d.x, -d.y, az)
    };
    (f, (sc / ma + 1.0) * 0.5, (tc / ma + 1.0) * 0.5)
}

/// Inversa de `dir_to_face`
fn face_dir(f: usize, u: f32, v: f32) -> Vec3 {
    let sc = 2.0 * u - 1.0;
    let tc = 2.0 * v - 1.0;
    match f {
        0 => Vec3::new(1.0, -tc, -sc),
        1 => Vec3::new(-1.0, -tc, sc),
        2 => Vec3::new(sc, 1.0, tc),
        3 => Vec3::new(sc, -1.0, -tc),
        4 => Vec3::new(sc, -tc, 1.0),
        _ => Vec3::new(-sc, -tc, -1.0),
    }
}

/// Cielo de atardecer procedural: gradiente vertical, calidez hacia el sol,
/// y nubes de ruido fractal por encima y por debajo del horizonte (las islas
/// flotan entre las nubes).
fn procedural_sky(d: Vec3, sun: Vec3) -> Vec3 {
    let y = d.y;
    let zenith = srgb(0.22, 0.34, 0.68);
    let upper = srgb(0.43, 0.58, 0.88);
    let horizon = srgb(0.98, 0.74, 0.64);
    let below = srgb(0.58, 0.56, 0.80);
    let abyss = srgb(0.20, 0.20, 0.42);

    let mut c = if y >= 0.0 {
        let c1 = Vec3::lerp(horizon, upper, smoothstep(0.0, 0.3, y));
        Vec3::lerp(c1, zenith, smoothstep(0.25, 1.0, y))
    } else {
        let c1 = Vec3::lerp(horizon, below, smoothstep(0.0, -0.25, y));
        Vec3::lerp(c1, abyss, smoothstep(-0.2, -1.0, y))
    };

    // Resplandor cálido en el lado del sol, pegado al horizonte
    let sun_h = Vec3::new(sun.x, 0.0, sun.z).normalize();
    let d_h = Vec3::new(d.x, 0.0, d.z).normalize();
    let warm = d_h.dot(sun_h).max(0.0).powi(3) * (1.0 - y.abs()).max(0.0).powi(3);
    c = Vec3::lerp(c, srgb(1.0, 0.56, 0.36), warm * 0.55);

    // Nubes: fBm 3D evaluado sobre la dirección (sin costuras en el cube map)
    let p = d * 2.4 + Vec3::new(11.0, 3.0, 7.0);
    let n = fbm3(p.x, p.y, p.z, 5);
    let band = 1.0 - smoothstep(0.30, 0.80, (y + 0.05).abs());
    let coverage = smoothstep(0.02, 0.30, n) * band;
    if coverage > 0.0 {
        // "Iluminación" de la nube: comparar con el ruido desplazado hacia el sol
        let q = p + sun * 0.15;
        let n2 = fbm3(q.x, q.y, q.z, 4);
        let lit = (0.6 + (n - n2) * 3.0).clamp(0.0, 1.0);
        let shadow = srgb(0.62, 0.60, 0.80);
        let bright = srgb(1.0, 0.95, 0.92);
        let mut cloud = Vec3::lerp(shadow, bright, lit);
        cloud = Vec3::lerp(cloud, srgb(1.0, 0.78, 0.66), warm * 0.6);
        c = Vec3::lerp(c, cloud, coverage * 0.92);
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_map_round_trip() {
        for f in 0..6 {
            let d = face_dir(f, 0.3, 0.8).normalize();
            let (f2, u, v) = dir_to_face(d);
            assert_eq!(f, f2);
            assert!((u - 0.3).abs() < 1e-4 && (v - 0.8).abs() < 1e-4);
        }
    }
}