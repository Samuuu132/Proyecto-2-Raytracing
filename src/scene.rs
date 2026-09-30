//! La escena completa: mundo de voxels + materiales + luces + skybox.

use crate::camera::OrbitCamera;
use crate::diorama;
use crate::light::{PointLight, Sun};
use crate::material::MaterialLibrary;
use crate::math::{srgb, Vec3};
use crate::skybox::Skybox;
use crate::world::World;

pub struct Scene {
    pub world: World,
    pub materials: MaterialLibrary,
    pub sun: Sun,
    pub lights: Vec<PointLight>,
    pub sky: Skybox,
    /// Punto al que mira la cámara orbital (centro del diorama)
    pub center: Vec3,
}

impl Scene {
    pub fn new() -> Self {
        let (world, lights) = diorama::build();
        let sun = Sun {
            direction: Vec3::new(0.55, 0.42, 0.50).normalize(),
            color: srgb(1.0, 0.80, 0.60) * 2.6,
        };
        let sky = Skybox::new(256, sun.direction, sun.color * 0.35);
        Self {
            world,
            materials: MaterialLibrary::new(),
            sun,
            lights,
            sky,
            center: Vec3::new(29.0, diorama::SURFACE_Y as f32 - 2.0, 14.0),
        }
    }

    /// Cámara inicial: de frente, un poco arriba, viendo todo el diorama
    pub fn default_orbit(&self) -> OrbitCamera {
        OrbitCamera::new(self.center, 0.35, 0.42, 50.0)
    }
}