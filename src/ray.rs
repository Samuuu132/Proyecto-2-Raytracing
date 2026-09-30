
use crate::math::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub inv_dir: Vec3,
}

impl Ray {
    #[inline]
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        let d = direction.normalize();
        Self {
            origin,
            direction: d,
            inv_dir: Vec3::new(1.0 / d.x, 1.0 / d.y, 1.0 / d.z),
        }
    }

    /// Punto del rayo a distancia t: P(t) = O + tD
    #[inline]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn intersect(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<(f32, f32)> {
        let mut t0 = t_min;
        let mut t1 = t_max;
        for a in 0..3 {
            let inv = ray.inv_dir[a];
            let mut tn = (self.min[a] - ray.origin[a]) * inv;
            let mut tf = (self.max[a] - ray.origin[a]) * inv;
            if inv < 0.0 {
                std::mem::swap(&mut tn, &mut tf);
            }
            t0 = t0.max(tn);
            t1 = t1.min(tf);
            if t1 < t0 {
                return None;
            }
        }
        Some((t0, t1))
    }

    #[inline]
    pub fn intersect_face(&self, ray: &Ray) -> Option<(f32, usize)> {
        let mut t_near = f32::NEG_INFINITY;
        let mut t_far = f32::INFINITY;
        let mut axis = 1;
        for a in 0..3 {
            let inv = ray.inv_dir[a];
            let mut tn = (self.min[a] - ray.origin[a]) * inv;
            let mut tf = (self.max[a] - ray.origin[a]) * inv;
            if inv < 0.0 {
                std::mem::swap(&mut tn, &mut tf);
            }
            if tn > t_near {
                t_near = tn;
                axis = a;
            }
            t_far = t_far.min(tf);
        }
        if t_far < t_near.max(0.0) {
            None
        } else {
            Some((t_near, axis))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slab_hits_unit_box() {
        let b = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let r = Ray::new(Vec3::new(0.5, 0.5, -3.0), Vec3::new(0.0, 0.0, 1.0));
        let (t0, t1) = b.intersect(&r, 0.0, f32::INFINITY).unwrap();
        assert!((t0 - 3.0).abs() < 1e-5 && (t1 - 4.0).abs() < 1e-5);
        assert_eq!(b.intersect_face(&r).unwrap().1, 2);
    }

    #[test]
    fn slab_misses() {
        let b = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let r = Ray::new(Vec3::new(2.0, 2.0, -3.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(b.intersect(&r, 0.0, f32::INFINITY).is_none());
    }
}
