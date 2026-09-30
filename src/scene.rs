//! Escena del paso 4: un "muestrario" con todos los bloques texturizados
//! sobre una plataforma, para revisar texturas, UVs y normal maps.
//! (En el paso 5 se cambia por el diorama y en el paso 6 por la escena final.)

use crate::block::*;
use crate::camera::OrbitCamera;
use crate::material::{MaterialKind, MaterialLibrary};
use crate::math::Vec3;
use crate::ray::Ray;
use crate::render::Shade;
use crate::voxel::face_uv;
use crate::world::World;

pub struct Scene {
    pub world: World,
    pub materials: MaterialLibrary,
    sun: Vec3,
    center: Vec3,
}

impl Scene {
    pub fn new() -> Self {
        let mut world = World::new([24, 8, 12]);
        // plataforma de ladrillos de piedra
        for z in 1..11 {
            for x in 1..23 {
                world.set(x, 0, z, STONE_BRICKS);
            }
        }
        // dos filas con cada tipo de bloque (del 1 al 18)
        for id in 1..BLOCK_COUNT as u8 {
            let i = (id - 1) as i32;
            let (x, z) = (2 + (i % 9) * 2 + 1, if i < 9 { 3 } else { 7 });
            world.set(x, 1, z, id);
        }
        world.add_region("muestrario".into(), [0, 0, 0], [24, 8, 12]);
        Self {
            world,
            materials: MaterialLibrary::new(),
            sun: Vec3::new(0.55, 0.75, 0.45).normalize(),
            center: Vec3::new(12.0, 1.0, 6.0),
        }
    }

    pub fn default_orbit(&self) -> OrbitCamera {
        OrbitCamera::new(self.center, 0.35, 0.6, 22.0)
    }
}

impl Shade for Scene {
    fn shade(&self, ray: &Ray) -> Vec3 {
        let Some(hit) = self.world.trace(ray, f32::INFINITY) else {
            // cielo provisional (el skybox llega en el paso 6)
            let t = 0.5 * (ray.direction.y + 1.0);
            return Vec3::lerp(Vec3::new(1.0, 0.62, 0.45), Vec3::new(0.18, 0.25, 0.55), t);
        };
        let m = self.materials.get(hit.block);
        let (u, v) = face_uv(hit.point, hit.cell, hit.axis, hit.normal);
        let albedo = self.materials.albedo(hit.block, hit.normal, u, v);
        if m.kind == MaterialKind::Emissive {
            return albedo * m.emission;
        }
        // normal con normal map (se nota en los ladrillos del piso)
        let n = self.materials.shading_normal(hit.block, hit.axis, hit.normal, u, v);
        albedo * (0.3 + 0.9 * n.dot(self.sun).max(0.0))
    }
}