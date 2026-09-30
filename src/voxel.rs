//! Primitiva cubo (voxel) basada en AABB: intersección, normal por cara,
//! coordenadas UV y marco tangente (para normal mapping).

use crate::math::Vec3;
use crate::ray::{Aabb, Ray};

/// Qué textura usar según la cara golpeada
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Top,
    Bottom,
    Side,
}

impl Face {
    #[inline]
    pub fn from_normal(n: Vec3) -> Face {
        if n.y > 0.5 {
            Face::Top
        } else if n.y < -0.5 {
            Face::Bottom
        } else {
            Face::Side
        }
    }
}

/// Un cubo unitario en la celda entera `cell` de la grilla.
pub struct Cube {
    pub aabb: Aabb,
}

pub struct CubeHit {
    pub t: f32,
    /// Eje de la cara impactada (0 = X, 1 = Y, 2 = Z)
    pub axis: usize,
    /// Normal geométrica, siempre apuntando hacia el origen del rayo
    pub normal: Vec3,
}

impl Cube {
    #[inline]
    pub fn at(cell: [i32; 3]) -> Self {
        let min = Vec3::new(cell[0] as f32, cell[1] as f32, cell[2] as f32);
        Self {
            aabb: Aabb::new(min, min + Vec3::ONE),
        }
    }

    #[inline]
    pub fn hit(&self, ray: &Ray) -> Option<CubeHit> {
        let (t, axis) = self.aabb.intersect_face(ray)?;
        let mut normal = Vec3::ZERO;
        let s = if ray.direction[axis] > 0.0 { -1.0 } else { 1.0 };
        match axis {
            0 => normal.x = s,
            1 => normal.y = s,
            _ => normal.z = s,
        }
        Some(CubeHit { t, axis, normal })
    }
}

/// Coordenadas UV (0..1) del punto `p` sobre la cara del cubo `cell`.
/// `v` crece hacia arriba en las caras laterales (importante para el pasto).
#[inline]
pub fn face_uv(p: Vec3, cell: [i32; 3], axis: usize, normal: Vec3) -> (f32, f32) {
    let lx = (p.x - cell[0] as f32).clamp(0.0, 0.9999);
    let ly = (p.y - cell[1] as f32).clamp(0.0, 0.9999);
    let lz = (p.z - cell[2] as f32).clamp(0.0, 0.9999);
    match axis {
        0 => {
            let u = if normal.x > 0.0 { 1.0 - lz } else { lz };
            (u, ly)
        }
        1 => (lx, lz),
        _ => {
            let u = if normal.z > 0.0 { lx } else { 1.0 - lx };
            (u, ly)
        }
    }
}

/// Marco tangente (T, B) consistente con `face_uv`: T apunta hacia +u y B hacia +v.
/// Con (T, B, N) se lleva una normal del espacio tangente al espacio mundo.
#[inline]
pub fn tangent_frame(axis: usize, normal: Vec3) -> (Vec3, Vec3) {
    match axis {
        0 => {
            let t = if normal.x > 0.0 {
                Vec3::new(0.0, 0.0, -1.0)
            } else {
                Vec3::new(0.0, 0.0, 1.0)
            };
            (t, Vec3::UP)
        }
        1 => (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0)),
        _ => {
            let t = if normal.z > 0.0 {
                Vec3::new(1.0, 0.0, 0.0)
            } else {
                Vec3::new(-1.0, 0.0, 0.0)
            };
            (t, Vec3::UP)
        }
    }
}