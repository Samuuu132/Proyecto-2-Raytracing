//! Integrador: decide el color de cada rayo (trazado recursivo tipo Whitted).
//!
//! Por cada impacto:
//!  1. Se calculan UV, color de textura y normal (con normal map si aplica).
//!  2. Luz directa del sol y de las luces emisivas, con rayos de sombra.
//!  3. Luz ambiental del cielo multiplicada por oclusión ambiental (AO).
//!  4. Según el material: reflexión (oro/obsidiana), refracción con Fresnel
//!     (agua/vidrio) o emisión (lava/portal/glowstone), con rayos recursivos.

use crate::block::AIR;
use crate::material::{Material, MaterialKind};
use crate::math::{schlick, Vec3};
use crate::ray::Ray;
use crate::render::Shade;
use crate::scene::Scene;
use crate::voxel::face_uv;
use crate::world::Hit;

/// Rebotes máximos (reflexión/refracción)
const MAX_DEPTH: u32 = 4;
const EPS: f32 = 1e-3;

impl Shade for Scene {
    fn shade(&self, ray: &Ray) -> Vec3 {
        self.radiance(ray, 0)
    }
}

impl Scene {
    /// Color que llega por un rayo
    fn radiance(&self, ray: &Ray, depth: u32) -> Vec3 {
        match self.world.trace(ray, f32::INFINITY) {
            Some(hit) => self.shade_hit(ray, &hit, depth),
            // Ray miss -> skybox
            None => self.sky.background(ray.direction),
        }
    }

    fn shade_hit(&self, ray: &Ray, hit: &Hit, depth: u32) -> Vec3 {
        let m = self.materials.get(hit.block);
        let (u, v) = face_uv(hit.point, hit.cell, hit.axis, hit.normal);
        let albedo = self.materials.albedo(hit.block, hit.normal, u, v);
        let mut n = self.materials.shading_normal(hit.block, hit.axis, hit.normal, u, v);
        if n.dot(ray.direction) > 0.0 {
            n = hit.normal; // el normal map nunca debe voltear la cara
        }

        match m.kind {
            MaterialKind::Emissive => self.shade_emissive(ray, hit, m, albedo, depth),
            MaterialKind::Dielectric => self.shade_dielectric(ray, hit, m, albedo, n, depth),
            _ => self.shade_opaque(ray, hit, m, albedo, n, depth),
        }
    }

    /// Luz directa (Blinn-Phong) de sol + luces puntuales, con sombras
    fn direct_light(&self, p: Vec3, ng: Vec3, n: Vec3, view: Vec3, m: &Material, albedo: Vec3) -> Vec3 {
        let origin = p + ng * EPS;
        let mut c = Vec3::ZERO;

        // --- Sol ---
        let l = self.sun.direction;
        let ndl = n.dot(l);
        if ndl > 0.0 && ng.dot(l) > 0.0 {
            let vis = self.world.transmittance(&Ray::new(origin, l), f32::INFINITY);
            if vis > 0.0 {
                let h = (l + view).normalize();
                let spec = m.specular * n.dot(h).max(0.0).powf(m.shininess);
                c += self.sun.color * (albedo * (m.albedo * ndl) + Vec3::splat(spec)) * vis;
            }
        }

        // --- Luces de los bloques emisivos ---
        for light in &self.lights {
            let to_l = light.position - p;
            let dist = to_l.length();
            let att = light.attenuation(dist);
            if att <= 0.0 {
                continue;
            }
            let l = to_l / dist;
            let ndl = n.dot(l);
            if ndl <= 0.0 || ng.dot(l) <= 0.0 {
                continue;
            }
            let vis = self.world.transmittance(&Ray::new(origin, l), dist - EPS);
            if vis > 0.0 {
                let h = (l + view).normalize();
                let spec = m.specular * n.dot(h).max(0.0).powf(m.shininess);
                c += light.color * (albedo * (m.albedo * ndl) + Vec3::splat(spec)) * (att * vis);
            }
        }
        c
    }

    fn shade_opaque(&self, ray: &Ray, hit: &Hit, m: &Material, albedo: Vec3, n: Vec3, depth: u32) -> Vec3 {
        let view = -ray.direction;
        let direct = self.direct_light(hit.point, hit.normal, n, view, m, albedo);
        let ao = self.world.ambient_occlusion(hit);
        let ambient = self.sky.ambient(n) * albedo * (m.albedo * ao);
        let mut c = direct + ambient;

        // Reflexión: R = I - 2 (I·N) N, trazada recursivamente
        if m.reflectivity > 0.0 && depth < MAX_DEPTH {
            let r = ray.direction.reflect(n);
            let refl = self.radiance(&Ray::new(hit.point + hit.normal * EPS, r), depth + 1);
            // los metales tiñen el reflejo con su color (oro = reflejo dorado)
            let tint = if m.kind == MaterialKind::Metal { albedo } else { Vec3::ONE };
            c = c * (1.0 - m.reflectivity) + refl * tint * m.reflectivity;
        }
        c
    }

    /// Lava, magma, glowstone y portal. El portal además es semitransparente.
    fn shade_emissive(&self, ray: &Ray, hit: &Hit, m: &Material, albedo: Vec3, depth: u32) -> Vec3 {
        let glow = albedo * m.emission;
        if m.transparency > 0.0 && depth < MAX_DEPTH {
            let behind = self.pass_through(ray, hit, depth);
            glow * (1.0 - m.transparency) + (behind + glow * 0.5) * m.transparency
        } else {
            glow
        }
    }

    /// Sigue un rayo que atraviesa un bloque sin desviarse (portal)
    fn pass_through(&self, ray: &Ray, hit: &Hit, depth: u32) -> Vec3 {
        let inside = Ray::new(hit.point + ray.direction * EPS, ray.direction);
        match self.world.trace_medium(&inside, hit.block, f32::INFINITY) {
            None => self.sky.background(ray.direction),
            Some(exit) if exit.block == AIR => {
                self.radiance(&Ray::new(exit.point + ray.direction * EPS, ray.direction), depth + 1)
            }
            Some(other) => self.shade_hit(&inside, &other, depth + 1),
        }
    }

    /// Agua y vidrio: Fresnel (Schlick) reparte entre reflejo y refracción
    /// (Snell). Dentro del medio se recorre hasta salir y se vuelve a refractar.
    fn shade_dielectric(&self, ray: &Ray, hit: &Hit, m: &Material, albedo: Vec3, n: Vec3, depth: u32) -> Vec3 {
        let view = -ray.direction;
        // Parte "sólida" de la superficie (color del agua, brillo del sol)
        let surface = self.direct_light(hit.point, hit.normal, n, view, m, albedo)
            + self.sky.ambient(n) * albedo * m.albedo;
        if depth >= MAX_DEPTH {
            return surface;
        }

        let cos_i = (-ray.direction.dot(n)).clamp(0.0, 1.0);
        let fresnel = schlick(cos_i, 1.0, m.ior).max(m.reflectivity);

        // Rayo reflejado
        let r = ray.direction.reflect(n);
        let reflected = self.radiance(&Ray::new(hit.point + hit.normal * EPS, r), depth + 1);

        // Rayo refractado (aire -> medio, nunca hay TIR al entrar)
        let transmitted = match ray.direction.refract(n, 1.0 / m.ior) {
            Some(t) => self.through_medium(hit, t, m, albedo, depth),
            None => reflected,
        };

        let optics = reflected * fresnel + transmitted * (1.0 - fresnel);
        surface * (1.0 - m.transparency) + optics * m.transparency
    }

    /// Recorre el interior del agua/vidrio, aplica absorción (Beer-Lambert) y
    /// al salir refracta otra vez (medio -> aire), detectando reflexión interna total.
    fn through_medium(&self, hit: &Hit, dir: Vec3, m: &Material, albedo: Vec3, depth: u32) -> Vec3 {
        let mut ray = Ray::new(hit.point - hit.normal * EPS, dir);
        // absorción: el agua se "come" primero el rojo, por eso se ve azul
        let sigma = (Vec3::ONE - albedo) * if m.ior > 1.4 { 0.08 } else { 1.1 };

        for _ in 0..3 {
            let Some(exit) = self.world.trace_medium(&ray, hit.block, 64.0) else {
                return self.sky.background(ray.direction);
            };
            let dist = (exit.point - ray.origin).length();
            let absorb = (sigma * -dist).exp();

            if exit.block != AIR {
                // fondo del estanque u otro bloque visto a través del agua
                return self.shade_hit(&ray, &exit, depth + 1) * absorb;
            }
            // Salida al aire: exit.normal apunta hacia adentro del medio
            match ray.direction.refract(exit.normal, m.ior) {
                Some(out) => {
                    let out_ray = Ray::new(exit.point + ray.direction * EPS, out);
                    return self.radiance(&out_ray, depth + 1) * absorb;
                }
                None => {
                    // Reflexión interna total: rebota dentro del medio
                    let r = ray.direction.reflect(exit.normal);
                    ray = Ray::new(exit.point + exit.normal * EPS, r);
                }
            }
        }
        albedo * 0.2
    }
}