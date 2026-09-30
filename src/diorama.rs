//! Construcción del diorama: isla del Overworld (terreno procedural con
//! Perlin), puente recto de piedra e isla del Nether.
//!
//! Ejes: X hacia la derecha (Overworld -> Nether), Y hacia arriba, Z hacia
//! el frente (donde está la cámara al inicio).

use crate::block::*;
use crate::light::PointLight;
use crate::math::{srgb, Vec3};
use crate::perlin::{fbm2, noise2};
use crate::world::World;

/// Tamaño de la grilla del mundo
pub const WORLD_SIZE: [i32; 3] = [60, 40, 32];
/// Altura de la superficie "plana" de las islas y del puente
pub const SURFACE_Y: i32 = 26;

// Isla del Overworld: centro y radio (en bloques)
const OW_CX: f32 = 12.5;
const OW_CZ: f32 = 13.5;
const OW_R: f32 = 10.5;

// Isla del Nether
const NE_CX: f32 = 45.5;
const NE_CZ: f32 = 13.5;
const NE_R: f32 = 8.3;

// Puente: carril central en Z = 12..15 (3 de ancho) + barandas en 11 y 15
const BRIDGE_Z0: i32 = 12;
const BRIDGE_Z1: i32 = 15; // exclusivo

/// Rectángulo [x0, x1) x [z0, z1) donde el terreno debe quedar plano
struct Rect(i32, i32, i32, i32);

impl Rect {
    fn contains(&self, x: i32, z: i32, margin: i32) -> bool {
        x >= self.0 - margin && x < self.1 + margin && z >= self.2 - margin && z < self.3 + margin
    }
}

/// Zonas del Overworld que se aplanan (casa, estanque, lava, oro, árbol, puente)
const OW_FLAT: [Rect; 6] = [
    Rect(8, 17, 4, 12),   // casa
    Rect(1, 10, 15, 25),  // estanque
    Rect(9, 16, 15, 21),  // lava
    Rect(15, 20, 16, 21), // oro
    Rect(3, 8, 6, 11),    // árbol
    Rect(17, 25, 10, 17), // llegada del puente
];

/// Construye todo y devuelve el mundo con sus luces
pub fn build() -> (World, Vec<PointLight>) {
    let mut w = World::new(WORLD_SIZE);
    let mut lights = Vec::new();

    overworld_island(&mut w);
    pond_and_waterfall(&mut w);
    lava_pool(&mut w, &mut lights);
    gold_cluster(&mut w);
    oak_tree(&mut w, 5, 8);
    house(&mut w);
    bridge(&mut w);
    nether_island(&mut w, &mut lights);

    // Broadphase: una caja por isla y otra para el puente
    w.add_region("overworld".into(), [0, 0, 0], [26, 40, 28]);
    w.add_region("puente".into(), [22, SURFACE_Y - 2, BRIDGE_Z0 - 1], [39, SURFACE_Y + 2, BRIDGE_Z1 + 1]);
    w.add_region("nether".into(), [36, 0, 3], [57, 40, 25]);

    (w, lights)
}

// ---------------------------------------------------------------------------
// Forma base de una isla flotante: dónde hay isla, a qué altura está la
// superficie y qué tan profunda es por debajo. Todo sale de ruido Perlin.
// ---------------------------------------------------------------------------

/// Distancia al centro y radio "ruidoso" del borde en esa dirección
fn island_edge(x: i32, z: i32, cx: f32, cz: f32, r: f32, seed: f32) -> (f32, f32) {
    let (fx, fz) = (x as f32 + 0.5 - cx, z as f32 + 0.5 - cz);
    let d = (fx * fx + fz * fz).sqrt();
    // el borde se deforma con ruido evaluado sobre la dirección (sin costuras)
    let (dx, dz) = if d > 0.0 { (fx / d, fz / d) } else { (1.0, 0.0) };
    let edge = r + noise2(dx * 1.7 + seed, dz * 1.7 + seed * 2.0) * 2.2;
    (d, edge)
}

/// Profundidad de la isla bajo la superficie: forma de cono invertido
/// (estalactita) con ruido fractal para que se vea rocosa.
fn island_depth(x: i32, z: i32, d: f32, edge: f32, seed: f32) -> i32 {
    let inner = (edge - d).max(0.0);
    let n = fbm2(x as f32 * 0.21 + seed, z as f32 * 0.21 - seed, 3) * 0.5 + 0.5;
    (2.0 + inner * 1.25 + n * 4.0 * (inner / edge).min(1.0) * 2.0) as i32
}

fn overworld_island(w: &mut World) {
    for z in 0..WORLD_SIZE[2] {
        for x in 0..26 {
            let (d, edge) = island_edge(x, z, OW_CX, OW_CZ, OW_R, 3.1);
            if d > edge {
                continue;
            }
            // ---- elevación Y = f(X, Z) con Perlin 2D (fBm de 3 octavas) ----
            let flat = OW_FLAT.iter().any(|r| r.contains(x, z, 1));
            let mut hill = 0;
            if !flat {
                let n = fbm2(x as f32 * 0.13 + 7.3, z as f32 * 0.13 + 1.9, 3);
                hill = ((n + 0.12) * 5.0).round().clamp(0.0, 3.0) as i32;
                if d > edge - 1.5 {
                    hill = hill.min(1); // bordes bajitos, como en Minecraft
                }
            }
            let top = SURFACE_Y + hill;
            let bottom = top - island_depth(x, z, d, edge, 5.7);

            for y in bottom.max(1)..=top {
                let depth_from_top = top - y;
                let b = if depth_from_top == 0 {
                    GRASS
                } else if depth_from_top <= 3 {
                    DIRT
                } else if noise2(x as f32 * 0.4, y as f32 * 0.4 + z as f32 * 0.3) > 0.45 {
                    COBBLESTONE // vetas de cobble en la roca
                } else {
                    STONE
                };
                w.set(x, y, z, b);
            }
        }
    }
}

fn in_ellipse(x: i32, z: i32, cx: f32, cz: f32, rx: f32, rz: f32, seed: f32) -> bool {
    let (fx, fz) = ((x as f32 + 0.5 - cx) / rx, (z as f32 + 0.5 - cz) / rz);
    fx * fx + fz * fz < 1.0 + noise2(x as f32 * 0.6 + seed, z as f32 * 0.6) * 0.35
}

fn pond_and_waterfall(w: &mut World) {
    let y = SURFACE_Y;
    // Estanque: 2 bloques de profundidad, superficie al ras del pasto
    for z in 14..25 {
        for x in 1..11 {
            // solo donde el agua queda rodeada de tierra (no se sale por el borde)
            let enclosed = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .all(|(dx, dz)| w.get(x + dx, y, z + dz) != AIR);
            if in_ellipse(x, z, 5.6, 18.4, 3.2, 2.6, 1.3) && enclosed {
                w.set(x, y, z, WATER);
                w.set(x, y - 1, z, WATER);
                w.set(x, y - 2, z, DIRT);
            }
        }
    }
    // Canal hacia el borde frontal y cascada cayendo al vacío
    for x in 4..6 {
        let mut z = 20;
        while w.get(x, y, z) != AIR {
            if w.get(x, y, z) != WATER {
                w.set(x, y, z, WATER);
                w.set(x, y - 1, z, DIRT);
            }
            z += 1;
        }
        // la columna de agua pegada al costado de la isla
        for yy in 2..=y {
            w.set(x, yy, z, WATER);
        }
    }
}

fn lava_pool(w: &mut World, lights: &mut Vec<PointLight>) {
    let y = SURFACE_Y;
    for z in 14..22 {
        for x in 8..17 {
            if in_ellipse(x, z, 12.6, 18.0, 3.0, 2.2, 7.7) && w.get(x, y, z) == GRASS {
                w.set(x, y, z, LAVA);
                w.set(x, y - 1, z, STONE);
            }
        }
    }
    lights.push(PointLight {
        position: Vec3::new(12.6, y as f32 + 1.6, 18.0),
        color: srgb(1.0, 0.52, 0.18),
        intensity: 10.0,
        range: 10.0,
    });
}

fn gold_cluster(w: &mut World) {
    let y = SURFACE_Y + 1;
    for (x, z) in [(16, 17), (17, 17), (16, 18), (17, 18), (18, 18), (17, 19)] {
        w.set(x, y, z, GOLD);
    }
    w.set(16, y + 1, 18, GOLD);
    w.set(17, y + 1, 18, GOLD);
    w.set(17, y + 2, 18, GOLD);
}

fn oak_tree(w: &mut World, tx: i32, tz: i32) {
    let base = SURFACE_Y + 1;
    let trunk_h = 5;
    for y in base..base + trunk_h {
        w.set(tx, y, tz, OAK_LOG);
    }
    // Copa: esfera achatada con huecos por ruido
    let cy = (base + trunk_h - 1) as f32 + 0.5;
    for y in base + 2..base + trunk_h + 3 {
        for z in tz - 3..=tz + 3 {
            for x in tx - 3..=tx + 3 {
                let (dx, dy, dz) = (
                    x as f32 - tx as f32,
                    (y as f32 + 0.5 - cy) * 1.3,
                    z as f32 - tz as f32,
                );
                let r = (dx * dx + dy * dy + dz * dz).sqrt();
                let jitter = noise2(x as f32 * 0.9 + y as f32 * 0.5, z as f32 * 0.9) * 0.6;
                if r < 3.1 + jitter && w.get(x, y, z) == AIR {
                    w.set(x, y, z, OAK_LEAVES);
                }
            }
        }
    }
}

fn house(w: &mut World) {
    let (x0, x1, z0, z1) = (9, 15, 5, 10); // paredes en x0..=x1, z0..=z1
    let yb = SURFACE_Y + 1;
    let wall_top = yb + 3;

    for y in yb..=wall_top {
        for z in z0..=z1 {
            for x in x0..=x1 {
                let edge_x = x == x0 || x == x1;
                let edge_z = z == z0 || z == z1;
                if !(edge_x || edge_z) {
                    w.set(x, y, z, AIR); // interior hueco
                    continue;
                }
                let b = if edge_x && edge_z {
                    OAK_LOG // esquinas de tronco
                } else if y == yb {
                    COBBLESTONE // zócalo de piedra
                } else {
                    OAK_PLANKS
                };
                w.set(x, y, z, b);
            }
        }
    }
    // piso de tablas
    for z in z0 + 1..z1 {
        for x in x0 + 1..x1 {
            w.set(x, SURFACE_Y, z, OAK_PLANKS);
        }
    }
    // puerta al frente y ventanas de vidrio
    let door_x = 12;
    w.set(door_x, yb, z1, OAK_DOOR);
    w.set(door_x, yb + 1, z1, OAK_DOOR);
    for (x, z) in [(10, z1), (11, z1), (13, z1), (14, z1)] {
        w.set(x, yb + 2, z, GLASS);
    }
    for z in [7, 8] {
        w.set(x0, yb + 2, z, GLASS);
        w.set(x1, yb + 2, z, GLASS);
    }
    // escalón de piedra frente a la puerta
    w.set(door_x, SURFACE_Y, z1 + 1, COBBLESTONE);

    // Techo a dos aguas con alero: cada capa sube 1 y se mete 1 en Z
    for k in 0..4 {
        let y = wall_top + 1 + k;
        let (za, zb) = (z0 - 1 + k, z1 + 1 - k);
        for x in x0 - 1..=x1 + 1 {
            w.set(x, y, za, OAK_PLANKS);
            w.set(x, y, zb, OAK_PLANKS);
        }
        // hastiales (triángulos de las paredes laterales)
        for z in za + 1..zb {
            w.set(x0, y, z, OAK_PLANKS);
            w.set(x1, y, z, OAK_PLANKS);
        }
    }
    // cumbrera
    for x in x0 - 1..=x1 + 1 {
        w.set(x, wall_top + 5, 7, OAK_LOG);
        w.set(x, wall_top + 5, 8, OAK_LOG);
    }
}

fn bridge(w: &mut World) {
    let y = SURFACE_Y;
    let zc = (BRIDGE_Z0 + BRIDGE_Z1) / 2;
    // borde este del Overworld y borde oeste del Nether sobre el eje del puente
    let x_lo = (0..30).rev().find(|&x| w.get(x, y, zc) != AIR).unwrap_or(22);
    let x_hi = (30..WORLD_SIZE[0]).find(|&x| w.get(x, y, zc) != AIR).unwrap_or(38);

    for x in x_lo - 2..=x_hi + 2 {
        for z in BRIDGE_Z0 - 1..=BRIDGE_Z1 {
            if w.get(x, y, z) != AIR {
                continue; // ahí ya está la isla
            }
            let rail = z == BRIDGE_Z0 - 1 || z == BRIDGE_Z1;
            w.set(x, y, z, STONE_BRICKS); // piso con normal map
            w.set(x, y - 1, z, COBBLESTONE); // viga inferior
            if rail && x > x_lo && x < x_hi {
                w.set(x, y + 1, z, COBBLESTONE); // baranda recta
            }
        }
    }
}

fn nether_island(w: &mut World, lights: &mut Vec<PointLight>) {
    let flat = [
        Rect(42, 50, 6, 10),  // portal
        Rect(36, 41, 10, 17), // llegada del puente
        Rect(46, 53, 14, 21), // lago de lava
    ];
    for z in 0..WORLD_SIZE[2] {
        for x in 36..57 {
            let (d, edge) = island_edge(x, z, NE_CX, NE_CZ, NE_R, 8.4);
            if d > edge {
                continue;
            }
            let is_flat = flat.iter().any(|r| r.contains(x, z, 0));
            let mut bump = 0;
            if !is_flat {
                let n = fbm2(x as f32 * 0.2 + 2.2, z as f32 * 0.2 + 9.1, 2);
                bump = ((n + 0.05) * 3.0).round().clamp(0.0, 1.0) as i32;
                if d > edge - 1.5 {
                    bump = 0;
                }
            }
            let top = SURFACE_Y + bump;
            let bottom = top - island_depth(x, z, d, edge, 11.3);
            for y in bottom.max(1)..=top {
                // parches de magma en la superficie y vetas en los costados
                let n = noise2(x as f32 * 0.45 + y as f32 * 0.2, z as f32 * 0.45 + 4.0);
                let magma = if y == top { !is_flat && n > 0.35 } else { n > 0.55 };
                let b = if magma { MAGMA } else { NETHERRACK };
                w.set(x, y, z, b);
            }
        }
    }

    // --- Portal del Nether: marco de obsidiana 4x5 con portal emisivo 2x3 ---
    let (px, pz, py) = (44, 8, SURFACE_Y + 1);
    for y in py..py + 5 {
        for x in px..px + 4 {
            let frame = x == px || x == px + 3 || y == py || y == py + 4;
            w.set(x, y, pz, if frame { OBSIDIAN } else { PORTAL });
        }
    }
    lights.push(PointLight {
        position: Vec3::new(46.0, py as f32 + 2.5, pz as f32 + 1.6),
        color: srgb(0.72, 0.35, 1.0),
        intensity: 5.0,
        range: 8.0,
    });

    // --- Lago de lava + cascada de lava por el costado ---
    for z in 13..22 {
        for x in 45..54 {
            if in_ellipse(x, z, 49.3, 17.4, 2.4, 2.1, 3.3) && w.get(x, SURFACE_Y, z) != AIR {
                w.set(x, SURFACE_Y, z, LAVA);
            }
        }
    }
    let lx = 50;
    let mut lz = 18;
    while w.get(lx, SURFACE_Y, lz) != AIR {
        w.set(lx, SURFACE_Y, lz, LAVA);
        lz += 1;
    }
    for y in SURFACE_Y - 11..=SURFACE_Y {
        w.set(lx, y, lz, LAVA);
    }
    lights.push(PointLight {
        position: Vec3::new(49.3, SURFACE_Y as f32 + 1.6, 17.4),
        color: srgb(1.0, 0.52, 0.18),
        intensity: 10.0,
        range: 10.0,
    });
    lights.push(PointLight {
        position: Vec3::new(lx as f32 + 0.5, SURFACE_Y as f32 - 5.0, lz as f32 + 1.8),
        color: srgb(1.0, 0.5, 0.15),
        intensity: 5.0,
        range: 8.0,
    });

    // --- Postes de obsidiana con glowstone (antorchas del Nether) ---
    for (x, z) in [(40, 10), (40, 16), (52, 10)] {
        let top = (SURFACE_Y..SURFACE_Y + 3).rev().find(|&y| w.get(x, y, z) != AIR).unwrap_or(SURFACE_Y);
        w.set(x, top + 1, z, OBSIDIAN);
        w.set(x, top + 2, z, GLOWSTONE);
        lights.push(PointLight {
            position: Vec3::new(x as f32 + 0.5, top as f32 + 3.6, z as f32 + 0.5),
            color: srgb(1.0, 0.82, 0.5),
            intensity: 2.5,
            range: 6.0,
        });
    }

    // --- Racimos de glowstone colgando bajo la isla ---
    for (x, z) in [(43, 12), (48, 15)] {
        let Some(bottom) = (1..SURFACE_Y).find(|&y| w.get(x, y, z) != AIR) else {
            continue;
        };
        for dy in 1..=2 {
            w.set(x, bottom - dy, z, GLOWSTONE);
        }
        w.set(x + 1, bottom - 1, z, GLOWSTONE);
        w.set(x, bottom - 1, z + 1, GLOWSTONE);
        lights.push(PointLight {
            position: Vec3::new(x as f32 + 0.8, bottom as f32 - 3.2, z as f32 + 0.8),
            color: srgb(1.0, 0.82, 0.5),
            intensity: 3.0,
            range: 7.0,
        });
    }
}