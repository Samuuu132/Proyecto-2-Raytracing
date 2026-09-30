
use crate::math::Vec3;
use crate::ray::Ray;
use std::f32::consts::{FRAC_PI_2, TAU};

pub struct Camera {
    pub position: Vec3,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
    tan_half_fov: f32,
    aspect: f32,
}

impl Camera {
    pub fn look_at(position: Vec3, target: Vec3, fov_deg: f32, aspect: f32) -> Self {
        let forward = (target - position).normalize();
        let right = forward.cross(Vec3::UP).normalize();
        let up = right.cross(forward);
        Self {
            position,
            forward,
            right,
            up,
            tan_half_fov: (fov_deg.to_radians() * 0.5).tan(),
            aspect,
        }
    }

    #[inline]
    pub fn ray(&self, sx: f32, sy: f32) -> Ray {
        let d = self.forward
            + self.right * (sx * self.tan_half_fov * self.aspect)
            + self.up * (sy * self.tan_half_fov);
        Ray::new(self.position, d)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitCamera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub fov: f32,
}

impl OrbitCamera {
    pub fn new(target: Vec3, yaw: f32, pitch: f32, distance: f32) -> Self {
        Self {
            target,
            yaw,
            pitch,
            distance,
            min_distance: distance * 0.35,
            max_distance: distance * 2.5,
            fov: 45.0,
        }
    }

    pub fn position(&self) -> Vec3 {
        let offset = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        self.target + offset * self.distance
    }

    pub fn camera(&self, aspect: f32) -> Camera {
        Camera::look_at(self.position(), self.target, self.fov, aspect)
    }

    pub fn rotate(&mut self, d_yaw: f32, d_pitch: f32) {
        self.yaw = (self.yaw + d_yaw).rem_euclid(TAU);
        self.pitch = (self.pitch + d_pitch).clamp(-FRAC_PI_2 + 0.05, FRAC_PI_2 - 0.05);
    }

    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(self.min_distance, self.max_distance);
    }
}