//! Texturas pixel-art de 16x16 generadas por código (estilo Minecraft) y
//! mapas de normales derivados de mapas de altura.
//!
//! No se cargan imágenes externas: cada textura es una función (x, y) -> color
//! con ruido hash determinista, así el proyecto no depende de ningún crate.

use crate::math::{srgb, Vec3};
use crate::perlin;

pub const TEX_SIZE: usize = 16;
const S: i32 = TEX_SIZE as i32;

/// Hash entero -> [0, 1). Determinista, sin estado.
#[inline]
pub fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0x00ff_ffff) as f32 / 16_777_216.0
}

/// Textura de albedo. Los colores se guardan ya en espacio lineal.
#[derive(Clone)]
pub struct Texture {
    data: Vec<Vec3>,
}

impl Texture {
    /// Construye la textura evaluando `f(x, y)` en cada texel. `f` devuelve sRGB.
    pub fn from_fn<F: Fn(i32, i32) -> Vec3>(f: F) -> Self {
        let mut data = Vec::with_capacity(TEX_SIZE * TEX_SIZE);
        for y in 0..S {
            for x in 0..S {
                let c = f(x, y);
                data.push(srgb(c.x.clamp(0.0, 1.0), c.y.clamp(0.0, 1.0), c.z.clamp(0.0, 1.0)));
            }
        }
        Self { data }
    }

    /// Muestreo nearest-neighbor (look pixelado de Minecraft). v = 0 abajo.
    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> Vec3 {
        let x = ((u * TEX_SIZE as f32) as usize).min(TEX_SIZE - 1);
        let y = (((1.0 - v) * TEX_SIZE as f32) as usize).min(TEX_SIZE - 1);
        self.data[y * TEX_SIZE + x]
    }
}

/// Mapa de normales en espacio tangente (x -> T, y -> B, z -> N).
#[derive(Clone)]
pub struct NormalMap {
    data: Vec<Vec3>,
}

impl NormalMap {
    /// Deriva las normales de un mapa de altura con diferencias centrales:
    /// n = normalize(-dh/du * k, -dh/dv * k, 1)
    pub fn from_height<F: Fn(i32, i32) -> f32>(height: F, strength: f32) -> Self {
        let h = |x: i32, y: i32| height(x.rem_euclid(S), y.rem_euclid(S));
        let mut data = Vec::with_capacity(TEX_SIZE * TEX_SIZE);
        for y in 0..S {
            for x in 0..S {
                let du = (h(x + 1, y) - h(x - 1, y)) * 0.5;
                // la fila crece hacia abajo, v crece hacia arriba
                let dv = (h(x, y - 1) - h(x, y + 1)) * 0.5;
                data.push(Vec3::new(-du * strength, -dv * strength, 1.0).normalize());
            }
        }
        Self { data }
    }

    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> Vec3 {
        let x = ((u * TEX_SIZE as f32) as usize).min(TEX_SIZE - 1);
        let y = (((1.0 - v) * TEX_SIZE as f32) as usize).min(TEX_SIZE - 1);
        self.data[y * TEX_SIZE + x]
    }
}

// ---------------------------------------------------------------------------
// Helpers de color (valores en sRGB 0..1)
// ---------------------------------------------------------------------------

#[inline]
fn col(r: f32, g: f32, b: f32) -> Vec3 {
    Vec3::new(r, g, b)
}

/// Varía el brillo de un color según un valor aleatorio t en [0,1)
#[inline]
fn vary(c: Vec3, t: f32, amount: f32) -> Vec3 {
    c * (1.0 - amount + 2.0 * amount * t)
}

fn lerp3(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    Vec3::lerp(a, b, t.clamp(0.0, 1.0))
}

// ---------------------------------------------------------------------------
// Generadores de texturas
// ---------------------------------------------------------------------------

fn grass_color(x: i32, y: i32) -> Vec3 {
    let c = vary(col(0.40, 0.66, 0.25), hash2(x, y, 1), 0.16);
    if hash2(x, y, 2) > 0.92 {
        col(0.55, 0.78, 0.32)
    } else if hash2(x, y, 3) < 0.06 {
        col(0.30, 0.52, 0.18)
    } else {
        c
    }
}

fn dirt_color(x: i32, y: i32) -> Vec3 {
    let c = vary(col(0.53, 0.38, 0.26), hash2(x, y, 4), 0.14);
    if hash2(x, y, 5) > 0.9 {
        col(0.40, 0.28, 0.19)
    } else if hash2(x, y, 6) > 0.95 {
        col(0.62, 0.50, 0.40)
    } else {
        c
    }
}

pub fn grass_top() -> Texture {
    Texture::from_fn(grass_color)
}

pub fn grass_side() -> Texture {
    Texture::from_fn(|x, y| {
        let lip = y < 3 || (y == 3 && hash2(x, 0, 9) > 0.4) || (y == 4 && hash2(x, 0, 10) > 0.8);
        if lip {
            grass_color(x, y)
        } else {
            dirt_color(x, y)
        }
    })
}

pub fn dirt() -> Texture {
    Texture::from_fn(dirt_color)
}

pub fn stone() -> Texture {
    Texture::from_fn(|x, y| {
        let n = perlin::noise2(x as f32 * 0.35 + 3.1, y as f32 * 0.35 + 7.7);
        Vec3::splat(0.50 + n * 0.08 + (hash2(x, y, 11) - 0.5) * 0.06)
    })
}

pub fn stone_height(x: i32, y: i32) -> f32 {
    perlin::noise2(x as f32 * 0.35 + 3.1, y as f32 * 0.35 + 7.7) * 0.6 + hash2(x, y, 11) * 0.3
}

/// Patrón de ladrillos: 4 filas de 4 texeles, ladrillos de 8 de largo desfasados.
fn brick_info(x: i32, y: i32) -> (bool, i32) {
    let row = y / 4;
    let offset = if row % 2 == 1 { 4 } else { 0 };
    let mortar = y % 4 == 3 || (x + offset) % 8 == 7;
    let brick_id = row * 4 + (x + offset) / 8;
    (mortar, brick_id)
}

pub fn stone_bricks() -> Texture {
    Texture::from_fn(|x, y| {
        let (mortar, id) = brick_info(x, y);
        if mortar {
            Vec3::splat(0.36 + hash2(x, y, 12) * 0.04)
        } else {
            let base = 0.55 + (hash2(id, 0, 13) - 0.5) * 0.08;
            let crack = hash2(x, y, 14) > 0.95;
            Vec3::splat(if crack { base - 0.12 } else { base + (hash2(x, y, 15) - 0.5) * 0.07 })
        }
    })
}

/// Altura para el normal map: mortero hundido, bordes biselados, grietas.
pub fn stone_bricks_height(x: i32, y: i32) -> f32 {
    let (mortar, _) = brick_info(x, y);
    if mortar {
        return 0.0;
    }
    let near_mortar = brick_info(x + 1, y).0
        || brick_info(x - 1, y).0
        || brick_info(x, y + 1).0
        || brick_info(x, y - 1).0;
    let crack = hash2(x, y, 14) > 0.95;
    let base = if near_mortar { 0.65 } else { 1.0 };
    base - if crack { 0.5 } else { 0.0 } + hash2(x, y, 15) * 0.12
}

/// Celdas de Voronoi para el cobblestone: (F2 - F1, id de la piedra)
fn cobble_cell(x: i32, y: i32) -> (f32, i32) {
    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
    let (cx, cy) = (x / 4, y / 4);
    let (mut f1, mut f2, mut id) = (f32::MAX, f32::MAX, 0);
    for j in -1..=1 {
        for i in -1..=1 {
            let (gx, gy) = (cx + i, cy + j);
            let (wx, wy) = (gx.rem_euclid(4), gy.rem_euclid(4));
            let fx = gx as f32 * 4.0 + 0.8 + hash2(wx, wy, 21) * 2.4;
            let fy = gy as f32 * 4.0 + 0.8 + hash2(wx, wy, 22) * 2.4;
            let d = ((px - fx).powi(2) + (py - fy).powi(2)).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
                id = wy * 4 + wx;
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f2 - f1, id)
}

pub fn cobblestone() -> Texture {
    Texture::from_fn(|x, y| {
        let (edge, id) = cobble_cell(x, y);
        if edge < 0.9 {
            Vec3::splat(0.28 + hash2(x, y, 23) * 0.05)
        } else {
            let base = 0.45 + hash2(id, 3, 24) * 0.17;
            Vec3::splat(base + (hash2(x, y, 25) - 0.5) * 0.06)
        }
    })
}

pub fn cobblestone_height(x: i32, y: i32) -> f32 {
    let (edge, _) = cobble_cell(x, y);
    (edge / 2.2).min(1.0)
}

fn log_side(bark: Vec3, seed: u32) -> Texture {
    Texture::from_fn(move |x, y| {
        let stripe = hash2(x, 0, seed) * 0.7 + hash2(x, y / 3, seed + 1) * 0.3;
        vary(bark, stripe, 0.22)
    })
}

fn log_top(bark: Vec3, wood: Vec3, ring: Vec3) -> Texture {
    Texture::from_fn(move |x, y| {
        if x == 0 || y == 0 || x == S - 1 || y == S - 1 {
            return vary(bark, hash2(x, y, 31), 0.1);
        }
        let d = ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt();
        if (d * 0.9) as i32 % 2 == 0 {
            vary(wood, hash2(x, y, 32), 0.05)
        } else {
            ring
        }
    })
}

pub fn oak_log_side() -> Texture {
    log_side(col(0.40, 0.30, 0.18), 33)
}

pub fn oak_log_top() -> Texture {
    log_top(col(0.40, 0.30, 0.18), col(0.70, 0.56, 0.34), col(0.58, 0.45, 0.27))
}

pub fn oak_leaves() -> Texture {
    Texture::from_fn(|x, y| {
        let n = hash2(x, y, 41);
        if n < 0.16 {
            col(0.13, 0.30, 0.09)
        } else if n > 0.92 {
            col(0.42, 0.68, 0.27)
        } else {
            vary(col(0.26, 0.52, 0.18), hash2(x, y, 42), 0.18)
        }
    })
}

pub fn water() -> Texture {
    Texture::from_fn(|x, y| {
        let s = perlin::noise2(x as f32 * 0.45, y as f32 * 0.3) * 0.5 + 0.5;
        lerp3(col(0.16, 0.38, 0.86), col(0.40, 0.66, 0.98), s * 0.7)
    })
}

pub fn water_height(x: i32, y: i32) -> f32 {
    ((x as f32 * 0.785) + (y as f32 * 0.39)).sin() * 0.5 + perlin::noise2(x as f32 * 0.5, y as f32 * 0.5) * 0.5
}

pub fn lava() -> Texture {
    Texture::from_fn(|x, y| {
        let v = perlin::noise2(x as f32 * 0.32 + 1.3, y as f32 * 0.32 + 4.2) * 0.5
            + 0.5
            + (hash2(x, y, 71) - 0.5) * 0.25;
        let dark = col(0.62, 0.12, 0.02);
        let orange = col(0.98, 0.45, 0.05);
        let yellow = col(1.0, 0.86, 0.35);
        if v < 0.5 {
            lerp3(dark, orange, v * 2.0)
        } else {
            lerp3(orange, yellow, (v - 0.5) * 2.0)
        }
    })
}

pub fn gold() -> Texture {
    Texture::from_fn(|x, y| {
        if x == 0 || y == S - 1 {
            col(1.0, 0.96, 0.62)
        } else if x == S - 1 || y == 0 {
            col(0.80, 0.55, 0.12)
        } else if (x + y) % 9 == 0 {
            col(1.0, 0.94, 0.55)
        } else {
            vary(col(0.99, 0.80, 0.26), hash2(x, y, 81), 0.05)
        }
    })
}

pub fn oak_planks() -> Texture {
    Texture::from_fn(|x, y| {
        let row = y / 4;
        if y % 4 == 3 {
            return col(0.45, 0.34, 0.19);
        }
        // uniones de las tablas desfasadas en cada fila
        if (x + row * 7) % 16 == 0 {
            return col(0.50, 0.38, 0.22);
        }
        let base = vary(col(0.72, 0.57, 0.35), hash2(row, 0, 56), 0.05);
        let grain = if hash2(x / 3, y, 57) > 0.8 { 0.93 } else { 1.0 };
        vary(base * grain, hash2(x, y, 58), 0.03)
    })
}

pub fn oak_door() -> Texture {
    Texture::from_fn(|x, y| {
        let frame = x <= 1 || x >= S - 2 || y <= 1 || y >= S - 2;
        let window = (4..=11).contains(&x) && (3..=6).contains(&y) && x != 7 && x != 8;
        if window {
            col(0.55, 0.70, 0.80)
        } else if frame {
            col(0.42, 0.30, 0.17)
        } else if x == 7 || x == 8 || y == 9 {
            col(0.50, 0.37, 0.21)
        } else if x == 12 && y == 10 {
            col(0.25, 0.25, 0.25) // manija
        } else {
            vary(col(0.66, 0.50, 0.30), hash2(x, y, 59), 0.05)
        }
    })
}

pub fn glass() -> Texture {
    Texture::from_fn(|x, y| {
        let frame = x == 0 || y == 0 || x == S - 1 || y == S - 1;
        if frame {
            col(0.80, 0.88, 0.92)
        } else if (x - y == 3 || x - y == 4) && x > 4 && x < 12 {
            col(0.97, 0.99, 1.0) // reflejo diagonal
        } else {
            col(0.86, 0.93, 0.97)
        }
    })
}

fn netherrack_value(x: i32, y: i32) -> f32 {
    perlin::noise2(x as f32 * 0.55 + 9.0, y as f32 * 0.55 + 2.0) * 0.5 + hash2(x, y, 111) * 0.5
}

pub fn netherrack() -> Texture {
    Texture::from_fn(|x, y| {
        let v = netherrack_value(x, y);
        if v > 0.62 {
            col(0.62, 0.26, 0.26)
        } else if v < 0.12 {
            col(0.30, 0.09, 0.10)
        } else {
            vary(col(0.45, 0.15, 0.15), hash2(x, y, 112), 0.12)
        }
    })
}

pub fn netherrack_height(x: i32, y: i32) -> f32 {
    netherrack_value(x, y)
}

pub fn obsidian() -> Texture {
    Texture::from_fn(|x, y| {
        let n = hash2(x, y, 121);
        let streak = perlin::noise2(x as f32 * 0.4 + 5.0, y as f32 * 0.4) * 0.5 + 0.5;
        if n > 0.93 {
            col(0.36, 0.25, 0.50)
        } else if streak > 0.72 {
            col(0.20, 0.13, 0.30)
        } else {
            vary(col(0.08, 0.06, 0.12), n, 0.3)
        }
    })
}

pub fn portal() -> Texture {
    Texture::from_fn(|x, y| {
        // remolinos: seno de la distancia al centro + ruido
        let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
        let a = dy.atan2(dx);
        let r = (dx * dx + dy * dy).sqrt();
        let swirl = ((r * 0.9 + a * 2.0).sin() * 0.5 + 0.5) * 0.7 + hash2(x, y, 131) * 0.3;
        lerp3(col(0.36, 0.08, 0.70), col(0.85, 0.45, 1.0), swirl)
    })
}

pub fn magma() -> Texture {
    Texture::from_fn(|x, y| {
        // placas oscuras separadas por grietas brillantes
        let (edge, _) = cobble_cell(x, y);
        if edge < 0.7 {
            lerp3(col(1.0, 0.55, 0.12), col(1.0, 0.80, 0.30), hash2(x, y, 141))
        } else {
            vary(col(0.42, 0.14, 0.06), hash2(x, y, 142), 0.2)
        }
    })
}

pub fn glowstone() -> Texture {
    Texture::from_fn(|x, y| {
        let n = hash2(x / 2, y / 2, 151) * 0.7 + hash2(x, y, 152) * 0.3;
        if n > 0.7 {
            col(1.0, 0.93, 0.66)
        } else if n < 0.25 {
            col(0.62, 0.42, 0.20)
        } else {
            col(0.93, 0.72, 0.40)
        }
    })
}