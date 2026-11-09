// Jugador en primera persona para interiores: caminar, mirar y colisionar en el plano XZ.
use crate::camara::Camara;
use crate::math::*;

#[derive(Clone)]
pub enum Zona {
    Rect { x0: f32, x1: f32, z0: f32, z1: f32 },
    Disco { cx: f32, cz: f32, r: f32 },
    Anillo { cx: f32, cz: f32, r0: f32, r1: f32 },
    Poligono(Vec<(f32, f32)>), // convexo, cualquier sentido
}

impl Zona {
    pub fn dentro(&self, x: f32, z: f32) -> bool {
        match self {
            Zona::Rect { x0, x1, z0, z1 } => x >= *x0 && x <= *x1 && z >= *z0 && z <= *z1,
            Zona::Disco { cx, cz, r } => (x - cx).powi(2) + (z - cz).powi(2) <= r * r,
            Zona::Anillo { cx, cz, r0, r1 } => {
                let d2 = (x - cx).powi(2) + (z - cz).powi(2);
                d2 >= r0 * r0 && d2 <= r1 * r1
            }
            Zona::Poligono(p) => {
                let (mut pos, mut neg) = (false, false);
                for i in 0..p.len() {
                    let (a, b) = (p[i], p[(i + 1) % p.len()]);
                    let c = (b.0 - a.0) * (z - a.1) - (b.1 - a.1) * (x - a.0);
                    if c > 0.0 {
                        pos = true
                    } else if c < 0.0 {
                        neg = true
                    }
                }
                !(pos && neg)
            }
        }
    }
}

// Caja orientada (mismo giro que Cubo::rotado_y / Rot::rot_y).
#[derive(Clone, Copy)]
pub struct Obst {
    pub cx: f32,
    pub cz: f32,
    pub hx: f32,
    pub hz: f32,
    pub yaw: f32,
}

impl Obst {
    pub fn caja(cx: f32, cz: f32, ancho: f32, fondo: f32) -> Obst {
        Obst {
            cx,
            cz,
            hx: ancho / 2.0,
            hz: fondo / 2.0,
            yaw: 0.0,
        }
    }
    fn toca(&self, x: f32, z: f32, r: f32) -> bool {
        let (s, c) = self.yaw.sin_cos();
        let (dx, dz) = (x - self.cx, z - self.cz);
        let (lx, lz) = (c * dx - s * dz, s * dx + c * dz);
        let ex = lx - lx.clamp(-self.hx, self.hx);
        let ez = lz - lz.clamp(-self.hz, self.hz);
        ex * ex + ez * ez < r * r
    }
}

#[derive(Default, Clone)]
pub struct Mundo {
    pub zonas: Vec<Zona>,
    pub obst: Vec<Obst>,
    pub dinamicos: Vec<Obst>,
}

impl Mundo {
    fn dentro(&self, x: f32, z: f32) -> bool {
        self.zonas.iter().any(|zn| zn.dentro(x, z))
    }
    // El círculo del jugador debe caber en la unión de zonas y no tocar obstáculos.
    pub fn libre(&self, x: f32, z: f32, r: f32) -> bool {
        if !self.dentro(x, z) {
            return false;
        }
        for k in 0..8 {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            if !self.dentro(x + r * a.cos(), z + r * a.sin()) {
                return false;
            }
        }
        !self
            .obst
            .iter()
            .chain(self.dinamicos.iter())
            .any(|o| o.toca(x, z, r))
    }
}

#[derive(Default)]
pub struct EntradaJ {
    pub adelante: f32,
    pub lateral: f32,
    pub giro_x: f32, // píxeles de ratón o equivalente
    pub giro_y: f32,
    pub correr: bool,
}

pub struct Jugador {
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub altura_ojos: f32,
    vx: f32,
    vz: f32,
    pub paso: f32, // distancia recorrida: para el balanceo de la cámara
}

pub const RADIO: f32 = 0.35;

impl Jugador {
    pub fn nuevo(x: f32, z: f32, yaw: f32) -> Jugador {
        Jugador {
            x,
            z,
            yaw,
            pitch: 0.0,
            altura_ojos: 1.65,
            vx: 0.0,
            vz: 0.0,
            paso: 0.0,
        }
    }

    pub fn colocar(&mut self, x: f32, z: f32, yaw: f32) {
        self.x = x;
        self.z = z;
        self.yaw = yaw;
        self.pitch = 0.0;
        self.vx = 0.0;
        self.vz = 0.0;
    }

    pub fn adelante(&self) -> (f32, f32) {
        (self.yaw.sin(), -self.yaw.cos())
    }

    pub fn actualizar(&mut self, e: &EntradaJ, dt: f32, mundo: &Mundo) {
        self.yaw += e.giro_x * 0.0022;
        self.pitch = (self.pitch - e.giro_y * 0.0022).clamp(-1.4, 1.4);
        self.yaw %= std::f32::consts::TAU;
        let (fx, fz) = (self.yaw.sin(), -self.yaw.cos());
        let (rx, rz) = (self.yaw.cos(), self.yaw.sin());
        let (mut wx, mut wz) = (
            fx * e.adelante + rx * e.lateral,
            fz * e.adelante + rz * e.lateral,
        );
        let l = (wx * wx + wz * wz).sqrt();
        if l > 1.0 {
            wx /= l;
            wz /= l;
        }
        let vel = if e.correr { 5.5 } else { 3.2 };
        let t = 1.0 - (-14.0 * dt).exp();
        self.vx += (wx * vel - self.vx) * t;
        self.vz += (wz * vel - self.vz) * t;
        // Subpasos: no se atraviesan muebles aunque caiga el FPS.
        let (dx, dz) = (self.vx * dt, self.vz * dt);
        let pasos = ((dx * dx + dz * dz).sqrt() / (RADIO * 0.5)).ceil().max(1.0) as usize;
        let (sx, sz) = (dx / pasos as f32, dz / pasos as f32);
        for _ in 0..pasos {
            if mundo.libre(self.x + sx, self.z, RADIO) {
                self.x += sx
            } else {
                self.vx = 0.0
            }
            if mundo.libre(self.x, self.z + sz, RADIO) {
                self.z += sz
            } else {
                self.vz = 0.0
            }
        }
        self.paso += (self.vx * self.vx + self.vz * self.vz).sqrt() * dt;
    }

    pub fn camara(&self, cam: &mut Camara) {
        let bob = (self.paso * 5.5).sin() * 0.025;
        cam.pos = v3(self.x, self.altura_ojos + bob, self.z);
        cam.yaw = self.yaw;
        cam.pitch = self.pitch;
        cam.base_ext = None;
        cam.fov = 72f32.to_radians();
    }
}
