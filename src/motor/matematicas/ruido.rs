// Aleatoriedad reproducible y ruido procedural (Perlin 3D, fBm y Worley).
// Todo es determinista: misma semilla => mismo resultado (lo pide la rúbrica).
use crate::math::*;

// Generador pseudoaleatorio SplitMix64.
pub struct Rng(u64);

impl Rng {
    pub fn new(semilla: u64) -> Rng { Rng(semilla.wrapping_add(0x9E37_79B9_7F4A_7C15)) }
    pub fn siguiente(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    // Número en [0, 1)
    pub fn f(&mut self) -> f32 { (self.siguiente() >> 40) as f32 / (1u64 << 24) as f32 }
    pub fn rango(&mut self, a: f32, b: f32) -> f32 { a + (b - a) * self.f() }
    // Dirección uniforme sobre la esfera unidad.
    pub fn direccion(&mut self) -> V3 {
        let z = self.rango(-1.0, 1.0);
        let a = self.rango(0.0, std::f32::consts::TAU);
        let r = (1.0 - z * z).sqrt();
        v3(r * a.cos(), z, r * a.sin())
    }
}

fn hash3(x: i32, y: i32, z: i32, semilla: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ (z as u32).wrapping_mul(0xCB1A_B31F)
        ^ semilla.wrapping_mul(0x1656_67B1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^ (h >> 15)
}

fn grad(h: u32, x: f32, y: f32, z: f32) -> f32 {
    match h & 15 {
        0 | 12 => x + y,
        1 | 13 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 | 14 => -y + z,
        10 => y - z,
        _ => -y - z,
    }
}

// Ruido de Perlin 3D, aproximadamente en [-1, 1].
pub fn ruido(p: V3, semilla: u32) -> f32 {
    let (fx, fy, fz) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (x, y, z) = (p.x - fx, p.y - fy, p.z - fz);
    let (xi, yi, zi) = (fx as i32, fy as i32, fz as i32);
    let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let (u, v, w) = (fade(x), fade(y), fade(z));
    let g = |dx: i32, dy: i32, dz: i32| {
        grad(hash3(xi + dx, yi + dy, zi + dz, semilla), x - dx as f32, y - dy as f32, z - dz as f32)
    };
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = l(g(0, 0, 0), g(1, 0, 0), u);
    let x10 = l(g(0, 1, 0), g(1, 1, 0), u);
    let x01 = l(g(0, 0, 1), g(1, 0, 1), u);
    let x11 = l(g(0, 1, 1), g(1, 1, 1), u);
    l(l(x00, x10, v), l(x01, x11, v), w)
}

// Suma de octavas (fBm). Devuelve aprox. [-1, 1] con valores típicos en ±0.5.
pub fn fbm(p: V3, octavas: u32, semilla: u32) -> f32 {
    let (mut suma, mut amp, mut frec, mut norma) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octavas {
        suma += ruido(p * frec, semilla.wrapping_add(o * 31)) * amp;
        norma += amp;
        amp *= 0.5;
        frec *= 2.0;
    }
    suma / norma * 1.35
}

// Ruido celular (Worley): distancia al punto más cercano (f1), al segundo (f2)
// y un número aleatorio 0..1 propio de la celda. Sirve para cráteres y granos.
pub fn celdas(p: V3, semilla: u32) -> (f32, f32, f32) {
    let (ix, iy, iz) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
    let (mut f1, mut f2, mut id) = (9.0f32, 9.0f32, 0.0f32);
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (cx, cy, cz) = (ix + dx, iy + dy, iz + dz);
                let h = hash3(cx, cy, cz, semilla);
                let px = cx as f32 + (h & 255) as f32 / 255.0 - p.x;
                let py = cy as f32 + ((h >> 8) & 255) as f32 / 255.0 - p.y;
                let pz = cz as f32 + ((h >> 16) & 255) as f32 / 255.0 - p.z;
                let d = (px * px + py * py + pz * pz).sqrt();
                if d < f1 {
                    f2 = f1;
                    f1 = d;
                    id = (h >> 24) as f32 / 255.0;
                } else if d < f2 {
                    f2 = d;
                }
            }
        }
    }
    (f1, f2, id)
}
