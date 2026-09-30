//! Escena del paso 5: el diorama completo con iluminación simple.
//! (En el paso 6 se reemplaza por la escena final con luces y skybox.)

use crate::camera::OrbitCamera;
use crate::diorama;
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
        // las luces de los emisivos se usan hasta el paso 6
        let (world, _lights) = diorama::build();
        Self {
            world,
            materials: MaterialLibrary::new(),
            sun: Vec3::new(0.55, 0.42, 0.50).normalize(),
            center: Vec3::new(29.0, diorama::SURFACE_Y as f32 - 2.0, 14.0),
        }
    }

    pub fn default_orbit(&self) -> OrbitCamera {
        OrbitCamera::new(self.center, 0.35, 0.42, 50.0)
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