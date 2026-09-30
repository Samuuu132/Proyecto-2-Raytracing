
pub const AIR: u8 = 0;
pub const GRASS: u8 = 1;
pub const DIRT: u8 = 2;
pub const STONE: u8 = 3;
pub const COBBLESTONE: u8 = 4;
pub const STONE_BRICKS: u8 = 5;
pub const OAK_LOG: u8 = 6;
pub const OAK_LEAVES: u8 = 7;
pub const OAK_PLANKS: u8 = 8;
pub const OAK_DOOR: u8 = 9;
pub const GLASS: u8 = 10;
pub const WATER: u8 = 11;
pub const LAVA: u8 = 12;
pub const GOLD: u8 = 13;
pub const NETHERRACK: u8 = 14;
pub const OBSIDIAN: u8 = 15;
pub const PORTAL: u8 = 16;
pub const MAGMA: u8 = 17;
pub const GLOWSTONE: u8 = 18;

pub const BLOCK_COUNT: usize = 19;

#[inline]
pub fn occludes(b: u8) -> bool {
    !matches!(b, AIR | WATER | GLASS | PORTAL)
}

#[inline]
pub fn shadow_transmission(b: u8) -> f32 {
    match b {
        AIR => 1.0,
        WATER => 0.8,
        GLASS => 0.9,
        PORTAL => 0.7,
        _ => 0.0,
    }
}