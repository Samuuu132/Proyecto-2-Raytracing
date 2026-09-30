//! Mundo voxel: grilla 3D densa + regiones (broadphase) + recorrido DDA.
//!
//! Estrategia de aceleración (de lo más barato a lo más caro):
//! 1. AABB de toda la grilla: si el rayo no la toca, va directo al skybox.
//! 2. AABB por región (cada isla y cada puente): el rayo solo se procesa en
//!    las regiones que cruza, ordenadas por distancia, y se corta en cuanto
//!    la siguiente región empieza más lejos que el mejor impacto encontrado.
//! 3. Dentro de cada región, recorrido voxel a voxel con el DDA de
//!    Amanatides & Woo: O(celdas cruzadas) en lugar de probar cada cubo.
//! 4. Al encontrar la celda ocupada, un único test de slabs contra el cubo
//!    da la distancia exacta, la cara, la normal y la UV.

use crate::block::{occludes, shadow_transmission, AIR};
use crate::math::Vec3;
use crate::ray::{Aabb, Ray};
use crate::voxel::Cube;

const MAX_REGIONS: usize = 16;

pub struct Region {
    pub name: String,
    pub bbox: Aabb,
    lo: [i32; 3],
    hi: [i32; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub t: f32,
    pub point: Vec3,
    /// Normal geométrica, apunta hacia el origen del rayo
    pub normal: Vec3,
    pub cell: [i32; 3],
    pub block: u8,
    pub axis: usize,
}

pub struct World {
    pub size: [i32; 3],
    cells: Vec<u8>,
    pub regions: Vec<Region>,
    bounds: Aabb,
}

impl World {
    pub fn new(size: [i32; 3]) -> Self {
        let n = (size[0] * size[1] * size[2]) as usize;
        Self {
            size,
            cells: vec![AIR; n],
            regions: Vec::new(),
            bounds: Aabb::new(
                Vec3::ZERO,
                Vec3::new(size[0] as f32, size[1] as f32, size[2] as f32),
            ),
        }
    }

    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32, z: i32) -> bool {
        x >= 0 && y >= 0 && z >= 0 && x < self.size[0] && y < self.size[1] && z < self.size[2]
    }

    #[inline]
    fn index(&self, x: i32, y: i32, z: i32) -> usize {
        ((y * self.size[2] + z) * self.size[0] + x) as usize
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if self.in_bounds(x, y, z) {
            self.cells[self.index(x, y, z)]
        } else {
            AIR
        }
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, z: i32, b: u8) {
        if self.in_bounds(x, y, z) {
            let i = self.index(x, y, z);
            self.cells[i] = b;
        }
    }

    pub fn solid_count(&self) -> usize {
        self.cells.iter().filter(|&&b| b != AIR).count()
    }

    pub fn add_region(&mut self, name: String, lo: [i32; 3], hi: [i32; 3]) {
        assert!(self.regions.len() < MAX_REGIONS, "demasiadas regiones");
        let lo = [lo[0].max(0), lo[1].max(0), lo[2].max(0)];
        let hi = [
            hi[0].min(self.size[0]),
            hi[1].min(self.size[1]),
            hi[2].min(self.size[2]),
        ];
        let bbox = Aabb::new(
            Vec3::new(lo[0] as f32, lo[1] as f32, lo[2] as f32),
            Vec3::new(hi[0] as f32, hi[1] as f32, hi[2] as f32),
        );
        self.regions.push(Region { name, bbox, lo, hi });
    }

    /// DDA 3D (Amanatides & Woo). Avanza celda por celda en [t0, t1] dentro
    /// de la caja [lo, hi) y se detiene cuando `stop(bloque)` devuelve true.
    #[inline]
    fn march<F: FnMut(u8) -> bool>(
        &self,
        ray: &Ray,
        t0: f32,
        t1: f32,
        lo: [i32; 3],
        hi: [i32; 3],
        mut stop: F,
    ) -> Option<[i32; 3]> {
        let p = ray.at(t0 + 1e-4);
        let d = ray.direction;
        let mut cell = [0i32; 3];
        let mut step = [0i32; 3];
        let mut t_next = [f32::INFINITY; 3];
        let mut t_delta = [f32::INFINITY; 3];
        for a in 0..3 {
            cell[a] = (p[a].floor() as i32).clamp(lo[a], hi[a] - 1);
            if d[a] > 0.0 {
                step[a] = 1;
                t_next[a] = ((cell[a] + 1) as f32 - ray.origin[a]) * ray.inv_dir[a];
                t_delta[a] = ray.inv_dir[a];
            } else if d[a] < 0.0 {
                step[a] = -1;
                t_next[a] = (cell[a] as f32 - ray.origin[a]) * ray.inv_dir[a];
                t_delta[a] = -ray.inv_dir[a];
            }
        }
        loop {
            if stop(self.cells[self.index(cell[0], cell[1], cell[2])]) {
                return Some(cell);
            }
            // eje cuyo próximo plano está más cerca
            let a = if t_next[0] < t_next[1] {
                if t_next[0] < t_next[2] {
                    0
                } else {
                    2
                }
            } else if t_next[1] < t_next[2] {
                1
            } else {
                2
            };
            if t_next[a] > t1 {
                return None;
            }
            cell[a] += step[a];
            if cell[a] < lo[a] || cell[a] >= hi[a] {
                return None;
            }
            t_next[a] += t_delta[a];
        }
    }

    /// Convierte la celda encontrada por el DDA en un impacto exacto
    #[inline]
    fn make_hit(&self, ray: &Ray, cell: [i32; 3], t_floor: f32) -> Hit {
        let block = self.get(cell[0], cell[1], cell[2]);
        let (t, axis, normal) = match Cube::at(cell).hit(ray) {
            Some(h) => (h.t.max(t_floor), h.axis, h.normal),
            None => (t_floor, 1, Vec3::UP),
        };
        Hit {
            t,
            point: ray.at(t),
            normal,
            cell,
            block,
            axis,
        }
    }

    /// Primer bloque no-aire que toca el rayo (incluye agua como superficie)
    pub fn trace(&self, ray: &Ray, t_max: f32) -> Option<Hit> {
        let (g0, g1) = self.bounds.intersect(ray, 1e-4, t_max)?;

        // Broadphase: regiones que el rayo cruza, ordenadas por entrada
        let mut cand = [(0.0f32, 0.0f32, 0usize); MAX_REGIONS];
        let mut n = 0;
        for (i, r) in self.regions.iter().enumerate() {
            if let Some((a, b)) = r.bbox.intersect(ray, g0, g1) {
                cand[n] = (a, b, i);
                n += 1;
            }
        }
        if n == 0 {
            return None;
        }
        let cand = &mut cand[..n];
        cand.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));

        let mut best: Option<Hit> = None;
        let mut best_t = g1;
        for &(a, b, i) in cand.iter() {
            if a > best_t {
                break; // todas las regiones que siguen están más lejos
            }
            let r = &self.regions[i];
            if let Some(cell) = self.march(ray, a, b.min(best_t), r.lo, r.hi, |blk| blk != AIR) {
                let h = self.make_hit(ray, cell, a);
                if h.t < best_t {
                    best_t = h.t;
                    best = Some(h);
                }
            }
        }
        best
    }

    /// Recorrido de un rayo que viaja DENTRO de un medio (agua): se salta las
    /// celdas del mismo medio y devuelve la primera celda distinta. Si es
    /// `AIR` significa que el rayo sale del agua por esa cara.
    pub fn trace_medium(&self, ray: &Ray, medium: u8, t_max: f32) -> Option<Hit> {
        let cell = self.march(ray, 0.0, t_max, [0, 0, 0], self.size, |blk| blk != medium)?;
        Some(self.make_hit(ray, cell, 0.0))
    }

    /// Rayo de sombra: 0 = bloqueado, 1 = libre. El agua, el vidrio y el
    /// portal dejan pasar parte de la luz (sombras más suaves y de color).
    pub fn transmittance(&self, ray: &Ray, t_max: f32) -> f32 {
        let Some((g0, g1)) = self.bounds.intersect(ray, 1e-4, t_max) else {
            return 1.0;
        };
        let mut tr = 1.0f32;
        for r in &self.regions {
            let Some((a, b)) = r.bbox.intersect(ray, g0, g1) else {
                continue;
            };
            self.march(ray, a, b, r.lo, r.hi, |blk| {
                if blk != AIR {
                    tr *= shadow_transmission(blk);
                }
                tr < 0.02
            });
            if tr < 0.02 {
                return 0.0;
            }
        }
        tr
    }

    /// Oclusión ambiental por voxel (la "smooth lighting" de Minecraft):
    /// se revisan los 8 vecinos de la cara y se interpola por esquinas.
    pub fn ambient_occlusion(&self, hit: &Hit) -> f32 {
        let a = hit.axis;
        let (ua, va) = ((a + 1) % 3, (a + 2) % 3);
        let mut c = hit.cell;
        c[a] += if hit.normal[a] > 0.0 { 1 } else { -1 };

        let occ = |du: i32, dv: i32| -> bool {
            let mut q = c;
            q[ua] += du;
            q[va] += dv;
            occludes(self.get(q[0], q[1], q[2]))
        };
        let corner = |du: i32, dv: i32| -> f32 {
            let s1 = occ(du, 0);
            let s2 = occ(0, dv);
            let cr = occ(du, dv);
            let o = if s1 && s2 {
                3
            } else {
                s1 as i32 + s2 as i32 + cr as i32
            };
            1.0 - o as f32 * 0.22
        };
        let fu = (hit.point[ua] - hit.cell[ua] as f32).clamp(0.0, 1.0);
        let fv = (hit.point[va] - hit.cell[va] as f32).clamp(0.0, 1.0);
        let c00 = corner(-1, -1);
        let c10 = corner(1, -1);
        let c01 = corner(-1, 1);
        let c11 = corner(1, 1);
        let bottom = c00 + (c10 - c00) * fu;
        let top = c01 + (c11 - c01) * fu;
        bottom + (top - bottom) * fv
    }
}