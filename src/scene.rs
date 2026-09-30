
use crate::math::Vec3;
use crate::ray::{Aabb, Ray};
use crate::render::Shade;

pub struct TestScene {
    cubes: Vec<(Aabb, Vec3)>,
    sun: Vec3,
}

impl TestScene {
    pub fn new() -> Self {
        let mut cubes = Vec::new();
        for z in -4i32..4 {
            for x in -4i32..4 {
                let hgt = ((x + z).rem_euclid(3)) as f32 * 0.25;
                let min = Vec3::new(x as f32, -1.0, z as f32);
                let color = if (x + z) % 2 == 0 {
                    Vec3::new(0.35, 0.65, 0.25)
                } else {
                    Vec3::new(0.30, 0.55, 0.20)
                };
                cubes.push((Aabb::new(min, min + Vec3::new(1.0, 1.0 + hgt, 1.0)), color));
            }
        }
        cubes.push((
            Aabb::new(Vec3::new(-3.0, 0.0, -3.0), Vec3::new(-1.0, 2.0, -1.0)),
            Vec3::new(0.70, 0.52, 0.30),
        ));
        cubes.push((
            Aabb::new(Vec3::new(1.0, 0.5, 1.0), Vec3::new(2.0, 1.5, 2.0)),
            Vec3::new(1.0, 0.8, 0.2),
        ));
        Self {
            cubes,
            sun: Vec3::new(0.5, 0.9, 0.35).normalize(),
        }
    }

    fn hit(&self, ray: &Ray) -> Option<(f32, usize, usize)> {
        let mut best: Option<(f32, usize, usize)> = None;
        for (i, (b, _)) in self.cubes.iter().enumerate() {
            if let Some((t, axis)) = b.intersect_face(ray) {
                if t > 1e-4 && best.map_or(true, |(bt, _, _)| t < bt) {
                    best = Some((t, axis, i));
                }
            }
        }
        best
    }
}

impl Shade for TestScene {
    fn shade(&self, ray: &Ray) -> Vec3 {
        match self.hit(ray) {
            Some((t, axis, i)) => {
                let mut n = Vec3::ZERO;
                let s = if ray.direction[axis] > 0.0 { -1.0 } else { 1.0 };
                match axis {
                    0 => n.x = s,
                    1 => n.y = s,
                    _ => n.z = s,
                }
                let p = ray.at(t) + n * 1e-3;
                let lit = self.hit(&Ray::new(p, self.sun)).is_none();
                let diff = if lit { n.dot(self.sun).max(0.0) } else { 0.0 };
                self.cubes[i].1 * (0.25 + 0.9 * diff)
            }
            None => {
                let t = 0.5 * (ray.direction.y + 1.0);
                Vec3::lerp(Vec3::new(1.0, 0.62, 0.45), Vec3::new(0.18, 0.25, 0.55), t)
            }
        }
    }
}