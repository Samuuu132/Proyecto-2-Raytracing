//! Luces de la escena.
//!
//! - `Sun`: luz direccional del atardecer (sombras duras).
//! - `PointLight`: luz puntual colocada sobre cada material emisivo (lava,
//!   portal, glowstone). Es la forma en que un bloque emisivo "ilumina" a los
//!   bloques que tiene cerca, con caída suave por distancia y sombras.

use crate::math::Vec3;

pub struct Sun {
    /// Dirección HACIA el sol (normalizada)
    pub direction: Vec3,
    pub color: Vec3,
}

pub struct PointLight {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    /// Más allá de esta distancia la luz ya no aporta (y no se lanza el rayo de sombra)
    pub range: f32,
}

impl PointLight {
    /// Atenuación física (1/d²) suavizada para que llegue a 0 exacto en `range`
    #[inline]
    pub fn attenuation(&self, dist: f32) -> f32 {
        if dist >= self.range {
            return 0.0;
        }
        let x = dist / self.range;
        let window = (1.0 - x * x * x * x).powi(2);
        self.intensity * window / (1.0 + dist * dist)
    }
}