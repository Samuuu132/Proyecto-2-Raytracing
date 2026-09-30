//! Paso 1: prueba de la base matemática.
//! Lanza un rayo por píxel contra un cubo (AABB) y colorea según la normal
//! de la cara golpeada. Si no golpea nada, pinta un degradado de cielo.

// Hay funciones que usaremos en pasos siguientes; por ahora no avisar
#![allow(dead_code)]

mod image;
mod math;
mod ray;

use math::Vec3;
use ray::{Aabb, Ray};
use std::path::Path;

fn main() {
    let (w, h) = (640usize, 360usize);
    let aspect = w as f32 / h as f32;

    // Cubo girado "a mano": lo vemos desde una esquina para ver 3 caras
    let cube = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
    let eye = Vec3::new(3.5, 2.8, 4.5);
    let forward = (Vec3::ZERO - eye).normalize();
    let right = forward.cross(Vec3::UP).normalize();
    let up = right.cross(forward);
    let tan_half = (45.0f32.to_radians() * 0.5).tan();

    let mut rgb = vec![0u8; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            let sx = (2.0 * (x as f32 + 0.5) / w as f32 - 1.0) * aspect * tan_half;
            let sy = (1.0 - 2.0 * (y as f32 + 0.5) / h as f32) * tan_half;
            let ray = Ray::new(eye, forward + right * sx + up * sy);

            let color = match cube.intersect_face(&ray) {
                Some((_t, axis)) => {
                    // Normal de la cara: el eje por donde entró el rayo
                    let mut n = Vec3::ZERO;
                    let s = if ray.direction[axis] > 0.0 { -1.0 } else { 1.0 };
                    match axis {
                        0 => n.x = s,
                        1 => n.y = s,
                        _ => n.z = s,
                    }
                    // Luz direccional simple (Lambert) para comprobar dot()
                    let light = Vec3::new(0.4, 1.0, 0.6).normalize();
                    let diff = n.dot(light).max(0.0);
                    (n * 0.5 + Vec3::splat(0.5)) * (0.3 + 0.7 * diff)
                }
                None => {
                    let t = 0.5 * (ray.direction.y + 1.0);
                    Vec3::lerp(Vec3::new(1.0, 0.75, 0.6), Vec3::new(0.3, 0.45, 0.85), t)
                }
            };
            let i = (y * w + x) * 3;
            rgb[i] = (color.x.clamp(0.0, 1.0) * 255.0) as u8;
            rgb[i + 1] = (color.y.clamp(0.0, 1.0) * 255.0) as u8;
            rgb[i + 2] = (color.z.clamp(0.0, 1.0) * 255.0) as u8;
        }
    }
    image::save(Path::new("paso1.bmp"), w, h, &rgb).expect("no se pudo guardar la imagen");
    println!("Listo: paso1.bmp ({}x{})", w, h);
}
