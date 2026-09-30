//! Sistema de materiales.
//!
//! Cada bloque tiene su propia textura (arriba / lados / abajo) y sus propios
//! parámetros de albedo, specular, transparencia y reflectividad (lo que pide
//! la rúbrica para que un material cuente). Según `kind`, el integrador
//! (shading.rs) lo trata de forma distinta:
//!
//! | kind         | Bloques                          | Efecto                           |
//! |--------------|----------------------------------|----------------------------------|
//! | Lambert      | pasto, tierra, madera, hojas     | Difuso                           |
//! | NormalMapped | puente (stone bricks), cobble,   | Difuso + especular con la normal |
//! |              | netherrack                       | perturbada por un normal map     |
//! | Metal        | oro                              | Reflexión especular recursiva    |
//! | Dielectric   | agua, vidrio                     | Snell + Fresnel (Schlick) + TIR  |
//! | Emissive     | lava, magma, glowstone, portal   | Emite luz (Le > 0)               |
//! | BlinnPhong   | obsidiana, piedra                | Ambiente + difuso + especular    |

use crate::block::*;
use crate::math::Vec3;
use crate::texture::{self as tex, NormalMap, Texture};
use crate::voxel::{tangent_frame, Face};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialKind {
    Lambert,
    NormalMapped,
    Metal,
    Dielectric,
    Emissive,
    BlinnPhong,
}

#[derive(Clone, Debug)]
pub struct Material {
    pub name: &'static str,
    pub kind: MaterialKind,
    /// Peso de la componente difusa
    pub albedo: f32,
    /// Peso del brillo especular (Blinn-Phong)
    pub specular: f32,
    /// Exponente del brillo: más alto = brillo más pequeño y definido
    pub shininess: f32,
    /// Fracción de luz que se refleja (rayo recursivo)
    pub reflectivity: f32,
    /// Fracción de luz que atraviesa el material
    pub transparency: f32,
    /// Índice de refracción
    pub ior: f32,
    /// Radiancia emitida (multiplica el color de la textura)
    pub emission: f32,
    pub tex_top: usize,
    pub tex_side: usize,
    pub tex_bottom: usize,
    pub normal_map: Option<usize>,
}

impl Material {
    fn new(name: &'static str, kind: MaterialKind, tex: usize) -> Self {
        Self {
            name,
            kind,
            albedo: 1.0,
            specular: 0.04,
            shininess: 8.0,
            reflectivity: 0.0,
            transparency: 0.0,
            ior: 1.0,
            emission: 0.0,
            tex_top: tex,
            tex_side: tex,
            tex_bottom: tex,
            normal_map: None,
        }
    }

    fn faces(mut self, top: usize, side: usize, bottom: usize) -> Self {
        self.tex_top = top;
        self.tex_side = side;
        self.tex_bottom = bottom;
        self
    }

    /// albedo, specular, shininess, reflectividad, transparencia
    fn params(mut self, albedo: f32, specular: f32, shininess: f32, refl: f32, transp: f32) -> Self {
        self.albedo = albedo;
        self.specular = specular;
        self.shininess = shininess;
        self.reflectivity = refl;
        self.transparency = transp;
        self
    }

    fn ior(mut self, ior: f32) -> Self {
        self.ior = ior;
        self
    }

    fn emission(mut self, e: f32) -> Self {
        self.emission = e;
        self
    }

    fn normals(mut self, map: usize) -> Self {
        self.normal_map = Some(map);
        self
    }
}

pub struct MaterialLibrary {
    materials: Vec<Material>,
    textures: Vec<Texture>,
    normal_maps: Vec<NormalMap>,
}

impl MaterialLibrary {
    pub fn new() -> Self {
        let mut textures: Vec<Texture> = Vec::new();
        let mut add = |t: Texture| {
            textures.push(t);
            textures.len() - 1
        };

        // ---------------- Texturas (una o más por material) ----------------
        let t_grass_top = add(tex::grass_top());
        let t_grass_side = add(tex::grass_side());
        let t_dirt = add(tex::dirt());
        let t_stone = add(tex::stone());
        let t_cobble = add(tex::cobblestone());
        let t_bricks = add(tex::stone_bricks());
        let t_log_side = add(tex::oak_log_side());
        let t_log_top = add(tex::oak_log_top());
        let t_leaves = add(tex::oak_leaves());
        let t_planks = add(tex::oak_planks());
        let t_door = add(tex::oak_door());
        let t_glass = add(tex::glass());
        let t_water = add(tex::water());
        let t_lava = add(tex::lava());
        let t_gold = add(tex::gold());
        let t_netherrack = add(tex::netherrack());
        let t_obsidian = add(tex::obsidian());
        let t_portal = add(tex::portal());
        let t_magma = add(tex::magma());
        let t_glowstone = add(tex::glowstone());

        // ---------------- Mapas de normales (desde mapas de altura) ----------------
        let normal_maps = vec![
            NormalMap::from_height(tex::stone_bricks_height, 2.6), // 0
            NormalMap::from_height(tex::cobblestone_height, 2.2),  // 1
            NormalMap::from_height(tex::stone_height, 1.3),        // 2
            NormalMap::from_height(tex::water_height, 0.35),       // 3
            NormalMap::from_height(tex::netherrack_height, 1.8),   // 4
        ];

        use MaterialKind::*;
        let mut m = vec![Material::new("air", Lambert, 0); BLOCK_COUNT];

        //                                                         albedo spec  shin  refl  transp
        m[GRASS as usize] = Material::new("grass", Lambert, t_grass_side)
            .faces(t_grass_top, t_grass_side, t_dirt)
            .params(1.0, 0.03, 8.0, 0.0, 0.0);
        m[DIRT as usize] = Material::new("dirt", Lambert, t_dirt).params(0.95, 0.02, 8.0, 0.0, 0.0);
        m[STONE as usize] = Material::new("stone", NormalMapped, t_stone)
            .params(0.90, 0.10, 16.0, 0.0, 0.0)
            .normals(2);
        m[COBBLESTONE as usize] = Material::new("cobblestone", NormalMapped, t_cobble)
            .params(0.95, 0.15, 16.0, 0.0, 0.0)
            .normals(1);
        m[STONE_BRICKS as usize] = Material::new("stone_bricks", NormalMapped, t_bricks)
            .params(0.95, 0.20, 24.0, 0.0, 0.0)
            .normals(0);
        m[OAK_LOG as usize] = Material::new("oak_log", Lambert, t_log_side)
            .faces(t_log_top, t_log_side, t_log_top)
            .params(1.0, 0.04, 8.0, 0.0, 0.0);
        m[OAK_LEAVES as usize] =
            Material::new("oak_leaves", Lambert, t_leaves).params(0.95, 0.06, 12.0, 0.0, 0.0);
        m[OAK_PLANKS as usize] =
            Material::new("oak_planks", BlinnPhong, t_planks).params(0.95, 0.12, 20.0, 0.0, 0.0);
        m[OAK_DOOR as usize] =
            Material::new("oak_door", BlinnPhong, t_door).params(0.95, 0.15, 24.0, 0.0, 0.0);
        m[GLASS as usize] = Material::new("glass", Dielectric, t_glass)
            .params(0.10, 0.9, 200.0, 0.06, 0.85)
            .ior(1.5);
        m[WATER as usize] = Material::new("water", Dielectric, t_water)
            .params(0.25, 0.9, 180.0, 0.05, 0.80)
            .ior(1.333)
            .normals(3);
        m[LAVA as usize] = Material::new("lava", Emissive, t_lava)
            .params(1.0, 0.0, 1.0, 0.0, 0.0)
            .emission(3.0);
        m[GOLD as usize] = Material::new("gold", Metal, t_gold)
            .params(0.85, 1.0, 320.0, 0.45, 0.0);
        m[NETHERRACK as usize] = Material::new("netherrack", NormalMapped, t_netherrack)
            .params(0.95, 0.10, 12.0, 0.0, 0.0)
            .normals(4);
        m[OBSIDIAN as usize] = Material::new("obsidian", BlinnPhong, t_obsidian)
            .params(0.85, 0.80, 96.0, 0.05, 0.0);
        m[PORTAL as usize] = Material::new("portal", Emissive, t_portal)
            .params(1.0, 0.0, 1.0, 0.0, 0.35)
            .emission(1.8);
        m[MAGMA as usize] = Material::new("magma", Emissive, t_magma)
            .params(1.0, 0.1, 8.0, 0.0, 0.0)
            .emission(1.4);
        m[GLOWSTONE as usize] = Material::new("glowstone", Emissive, t_glowstone)
            .params(1.0, 0.0, 1.0, 0.0, 0.0)
            .emission(2.2);

        Self {
            materials: m,
            textures,
            normal_maps,
        }
    }

    /// Nombres de todos los materiales (se imprimen al iniciar)
    pub fn names(&self) -> Vec<&'static str> {
        self.materials.iter().skip(1).map(|m| m.name).collect()
    }

    #[inline]
    pub fn get(&self, block: u8) -> &Material {
        &self.materials[block as usize]
    }

    /// Color de la textura (espacio lineal) según la cara y la UV
    #[inline]
    pub fn albedo(&self, block: u8, normal: Vec3, u: f32, v: f32) -> Vec3 {
        let m = self.get(block);
        let t = match Face::from_normal(normal) {
            Face::Top => m.tex_top,
            Face::Bottom => m.tex_bottom,
            Face::Side => m.tex_side,
        };
        self.textures[t].sample(u, v)
    }

    /// Normal de sombreado. Con normal map: N' = T*n.x + B*n.y + N*n.z
    #[inline]
    pub fn shading_normal(&self, block: u8, axis: usize, normal: Vec3, u: f32, v: f32) -> Vec3 {
        match self.get(block).normal_map {
            None => normal,
            Some(i) => {
                let (t, b) = tangent_frame(axis, normal);
                let n = self.normal_maps[i].sample(u, v);
                (t * n.x + b * n.y + normal * n.z).normalize()
            }
        }
    }
}