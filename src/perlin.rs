
const P: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69,
    142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219,
    203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122, 60, 211, 133, 230,
    220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73, 209, 76,
    132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173,
    186, 3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206,
    59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163,
    70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232,
    178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162,
    241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184, 84, 204,
    176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141,
    128, 195, 78, 66, 215, 61, 156, 180,
];

#[inline]
fn perm(i: usize) -> usize {
    P[i & 255] as usize
}

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}

#[inline]
fn grad2(hash: usize, x: f32, y: f32) -> f32 {
    match hash & 7 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x,
        5 => -x,
        6 => y,
        _ => -y,
    }
}

#[inline]
fn grad3(hash: usize, x: f32, y: f32, z: f32) -> f32 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

pub fn noise2(x: f32, y: f32) -> f32 {
    let xf0 = x.floor();
    let yf0 = y.floor();
    let xi = (xf0 as i32 & 255) as usize;
    let yi = (yf0 as i32 & 255) as usize;
    let xf = x - xf0;
    let yf = y - yf0;
    let u = fade(xf);
    let v = fade(yf);

    let aa = perm(perm(xi) + yi);
    let ab = perm(perm(xi) + yi + 1);
    let ba = perm(perm(xi + 1) + yi);
    let bb = perm(perm(xi + 1) + yi + 1);

    let x1 = lerp(u, grad2(aa, xf, yf), grad2(ba, xf - 1.0, yf));
    let x2 = lerp(u, grad2(ab, xf, yf - 1.0), grad2(bb, xf - 1.0, yf - 1.0));
    lerp(v, x1, x2) * 0.9
}

pub fn noise3(x: f32, y: f32, z: f32) -> f32 {
    let (xf0, yf0, zf0) = (x.floor(), y.floor(), z.floor());
    let xi = (xf0 as i32 & 255) as usize;
    let yi = (yf0 as i32 & 255) as usize;
    let zi = (zf0 as i32 & 255) as usize;
    let (xf, yf, zf) = (x - xf0, y - yf0, z - zf0);
    let (u, v, w) = (fade(xf), fade(yf), fade(zf));

    let a = perm(xi) + yi;
    let aa = perm(a) + zi;
    let ab = perm(a + 1) + zi;
    let b = perm(xi + 1) + yi;
    let ba = perm(b) + zi;
    let bb = perm(b + 1) + zi;

    lerp(
        w,
        lerp(
            v,
            lerp(u, grad3(perm(aa), xf, yf, zf), grad3(perm(ba), xf - 1.0, yf, zf)),
            lerp(
                u,
                grad3(perm(ab), xf, yf - 1.0, zf),
                grad3(perm(bb), xf - 1.0, yf - 1.0, zf),
            ),
        ),
        lerp(
            v,
            lerp(
                u,
                grad3(perm(aa + 1), xf, yf, zf - 1.0),
                grad3(perm(ba + 1), xf - 1.0, yf, zf - 1.0),
            ),
            lerp(
                u,
                grad3(perm(ab + 1), xf, yf - 1.0, zf - 1.0),
                grad3(perm(bb + 1), xf - 1.0, yf - 1.0, zf - 1.0),
            ),
        ),
    )
}

pub fn fbm2(x: f32, y: f32, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for _ in 0..octaves {
        sum += noise2(x * freq, y * freq) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

pub fn fbm3(x: f32, y: f32, z: f32, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for _ in 0..octaves {
        sum += noise3(x * freq, y * freq, z * freq) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_zero_on_integer_lattice() {
        assert!(noise2(3.0, 7.0).abs() < 1e-6);
        assert!(noise3(1.0, 2.0, 3.0).abs() < 1e-6);
    }

    #[test]
    fn noise_is_bounded() {
        for i in 0..2000 {
            let x = i as f32 * 0.137;
            let n = noise2(x, x * 0.71 + 3.3);
            assert!((-1.01..=1.01).contains(&n));
        }
    }
}