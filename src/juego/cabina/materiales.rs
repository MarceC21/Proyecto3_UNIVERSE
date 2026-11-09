// Fragment shader de interiores: patrones procedurales (UV en metros) + luces puntuales.
// No hay texturas: todo se evalúa por píxel, como los demás shaders del proyecto.
use crate::math::*;
use crate::shader::{Contexto, Fragmento, Shader};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mat {
    Pared,
    Piso,
    Techo,
    Metal,
    MetalOscuro,
    Refuerzo,
    Tapizado,
    Radar,
    Telemetria,
    Monitor,
    Holo,
    Botonera,
    Cian,
    Ambar,
    Rojo,
    Luz,
    Compuerta,
    Peligro,
    Pintura,
    Casco,
    Objeto,
}

impl Mat {
    pub const N: usize = 21;
    pub const TODOS: [Mat; Mat::N] = [
        Mat::Pared,
        Mat::Piso,
        Mat::Techo,
        Mat::Metal,
        Mat::MetalOscuro,
        Mat::Refuerzo,
        Mat::Tapizado,
        Mat::Radar,
        Mat::Telemetria,
        Mat::Monitor,
        Mat::Holo,
        Mat::Botonera,
        Mat::Cian,
        Mat::Ambar,
        Mat::Rojo,
        Mat::Luz,
        Mat::Compuerta,
        Mat::Peligro,
        Mat::Pintura,
        Mat::Casco,
        Mat::Objeto,
    ];
}

#[derive(Clone, Copy)]
pub struct Luz {
    pub pos: V3,
    pub color: V3,
    pub inten: f32,
    pub radio: f32,
}

pub struct ShaderInterior {
    pub mat: Mat,
    pub luces: Arc<Vec<Luz>>,
    pub ambiente: V3,
}

impl ShaderInterior {
    pub fn nuevo(mat: Mat, luces: Arc<Vec<Luz>>, ambiente: V3) -> Self {
        ShaderInterior {
            mat,
            luces,
            ambiente,
        }
    }
}

fn frac(x: f32) -> f32 {
    x - x.floor()
}
fn hash(x: f32, y: f32) -> f32 {
    frac((x * 12.9898 + y * 78.233).sin() * 43758.545)
}
fn gris(a: f32) -> V3 {
    v3(a * 0.93, a * 0.97, a)
}
fn cian() -> V3 {
    rgb(60.0, 225.0, 255.0)
}

struct Sup {
    alb: V3,
    emi: V3,
    esp: f32,
    sin_luz: bool,
}

fn sup(alb: V3, esp: f32) -> Sup {
    Sup {
        alb,
        emi: V3::ZERO,
        esp,
        sin_luz: false,
    }
}
fn emisivo(c: V3) -> Sup {
    Sup {
        alb: V3::ZERO,
        emi: c,
        esp: 0.0,
        sin_luz: true,
    }
}

impl ShaderInterior {
    fn superficie(&self, f: &Fragmento, t: f32) -> Sup {
        let (u, v) = (f.uv[0], f.uv[1]);
        match self.mat {
            Mat::Pared => {
                let cu = frac(u / 1.2);
                let d = cu.min(1.0 - cu) * 1.2;
                let mut a = 0.58 + (hash((u * 9.0).floor(), (v * 9.0).floor()) - 0.5) * 0.025;
                if d < 0.014 {
                    a *= 0.45
                } else if d < 0.04 {
                    a *= 1.1
                }
                let cv = frac(v / 1.2);
                if d > 0.14 && d < 0.17 && cv > 0.12 && cv < 0.88 {
                    a *= 0.8
                }
                let mut s = sup(gris(a), 0.08);
                if v < 0.28 {
                    s.alb = gris(0.30);
                } else if v < 0.33 {
                    s.alb = s.alb * 0.7;
                }
                if (v - 0.305).abs() < 0.012 {
                    s.emi = cian() * 0.9;
                }
                s
            }
            Mat::Piso => {
                let (cu, cv) = (frac(u / 1.2), frac(v / 1.2));
                let d = cu.min(1.0 - cu).min(cv).min(1.0 - cv) * 1.2;
                let mut a = 0.36 + (hash((u * 30.0).floor(), (v * 30.0).floor()) - 0.5) * 0.04;
                if d < 0.012 {
                    a *= 0.5
                } else if d < 0.03 {
                    a *= 1.18
                }
                if frac((u + v) * 2.5) < 0.12 {
                    a *= 1.12
                }
                sup(gris(a), 0.22)
            }
            Mat::Techo => {
                let (cu, cv) = (frac(u / 2.4), frac(v / 2.4));
                let d = cu.min(1.0 - cu).min(cv).min(1.0 - cv) * 2.4;
                sup(gris(if d < 0.02 { 0.35 } else { 0.64 }), 0.05)
            }
            Mat::Metal => {
                let s = 0.5 + 0.5 * hash((v * 70.0).floor(), 3.0);
                sup(gris(0.60 + s * 0.06), 0.6)
            }
            Mat::MetalOscuro => sup(gris(0.20 + 0.05 * hash((v * 70.0).floor(), 5.0)), 0.4),
            Mat::Refuerzo => sup(gris(0.40), 0.15),
            Mat::Tapizado => {
                let (a, b) = (frac((u + v) / 0.2), frac((u - v) / 0.2));
                let d = a.min(1.0 - a).min(b).min(1.0 - b);
                let k = if d < 0.05 { 0.45 } else { 0.85 + 0.3 * d };
                sup(v3(0.36, 0.43, 0.52) * k, 0.05)
            }
            Mat::Radar => {
                let (x, y) = ((u - 0.37) * 1.6, v - 0.5);
                let r = (x * x + y * y).sqrt();
                let mut c = rgb(4.0, 18.0, 26.0);
                let verde = rgb(75.0, 225.0, 140.0);
                if u > 0.78 {
                    let barra = ((v * 7.0).floor()) as f32;
                    let lleno = 0.3 + 0.7 * frac(hash(barra, 1.0) + t * 0.1 * (1.0 + barra * 0.2));
                    let dentro = frac(v * 7.0) > 0.35 && (u - 0.8) / 0.18 < lleno;
                    c = if dentro {
                        verde * 0.8
                    } else {
                        c + rgb(10.0, 40.0, 40.0) * 0.4
                    };
                } else if r < 0.43 {
                    for k in 1..5 {
                        if (r - k as f32 * 0.1).abs() < 0.004 {
                            c = verde * 0.55;
                        }
                    }
                    if x.abs() < 0.003 || y.abs() < 0.003 {
                        c = verde * 0.5;
                    }
                    let ang = y.atan2(x);
                    let barrido = (t * 1.3) % std::f32::consts::TAU;
                    let dif = (barrido - ang).rem_euclid(std::f32::consts::TAU);
                    c = c + verde * ((-dif * 2.2).exp() * 0.55);
                    for (bx, by) in [(0.15f32, 0.10f32), (-0.2, -0.12), (0.05, -0.3)] {
                        let (dx, dy) = (x - bx, y - by);
                        if dx * dx + dy * dy < 0.0004 {
                            let da = (barrido - by.atan2(bx)).rem_euclid(std::f32::consts::TAU);
                            c = c + rgb(200.0, 255.0, 180.0) * (0.3 + (-da * 1.2).exp());
                        }
                    }
                }
                if u < 0.02 || u > 0.98 || v < 0.03 || v > 0.97 {
                    c = cian() * 0.8;
                }
                emisivo(c)
            }
            Mat::Telemetria => {
                let mut c = rgb(5.0, 18.0, 32.0);
                if frac(u * 12.0) < 0.04 || frac(v * 8.0) < 0.05 {
                    c = rgb(18.0, 65.0, 85.0);
                }
                let yw = 0.5 + 0.22 * (u * 9.0 - t * 2.0).sin() + 0.08 * (u * 23.0 + t).sin();
                if u < 0.66 && (v - yw).abs() < 0.018 {
                    c = cian();
                }
                if u > 0.72 {
                    let col = ((u - 0.72) / 0.07).floor();
                    if frac((u - 0.72) / 0.07) < 0.75 {
                        let h =
                            0.25 + 0.7 * (0.5 + 0.5 * (t * (0.8 + col * 0.35) + col * 1.7).sin());
                        if 1.0 - v < h {
                            c = rgb(75.0, 215.0, 145.0);
                        } else {
                            c = rgb(18.0, 65.0, 85.0);
                        }
                    }
                }
                if u < 0.015 || u > 0.985 || v < 0.025 || v > 0.975 {
                    c = cian() * 0.9;
                }
                emisivo(c)
            }
            Mat::Monitor => {
                let (x, y) = ((u - 0.5) * 2.0, (v - 0.5) * 2.0);
                let mut c = rgb(4.0, 18.0, 31.0);
                if frac(u * 20.0) < 0.03 || frac(v * 10.0) < 0.05 {
                    c = rgb(15.0, 62.0, 83.0);
                }
                let ancho = if y < -0.1 {
                    0.08 + (y + 0.8).max(0.0) * 0.25
                } else {
                    0.26 - (y + 0.1) * 0.14
                };
                if y > -0.8
                    && y < 0.7
                    && ((x.abs() - ancho).abs() < 0.018 || (y + 0.8).abs() < 0.015)
                {
                    c = cian();
                }
                if x.abs() < 0.012 && y > -0.8 && y < 0.7 {
                    c = cian() * 0.8;
                }
                for (nx, ny) in [(-0.32f32, 0.15f32), (0.32, 0.15), (-0.2, 0.55), (0.2, 0.55)] {
                    let (dx, dy) = (x - nx, y - ny);
                    let r2 = dx * dx + dy * dy;
                    if r2 < 0.0016 {
                        c = rgb(80.0, 205.0, 152.0);
                    } else if (r2 - 0.006).abs() < 0.0008 {
                        c = cian();
                    }
                }
                let barrido = frac(t * 0.25) * 2.0 - 1.0;
                if (y - barrido).abs() < 0.012 {
                    c = c + cian() * 0.5;
                }
                if x > 0.62 && x < 0.92 {
                    let k = ((y + 1.0) * 5.0).floor();
                    if frac((y + 1.0) * 5.0) > 0.3
                        && (x - 0.62) / 0.3 < 0.25 + 0.7 * frac(hash(k, 2.0) + t * 0.08)
                    {
                        c = rgb(80.0, 205.0, 152.0);
                    }
                }
                if u < 0.012 || u > 0.988 || v < 0.02 || v > 0.98 {
                    c = cian() * 0.9;
                }
                emisivo(c)
            }
            Mat::Holo => {
                let (x, z) = ((u - 0.5) * 4.4, (v - 0.5) * 3.1);
                let r = (x * x + z * z).sqrt();
                let mut c = rgb(2.0, 10.0, 24.0) + rgb(6.0, 30.0, 60.0) * ((-r * 0.5).exp());
                if frac(x / 0.31) < 0.025 || frac(z / 0.31) < 0.03 {
                    c = c + rgb(10.0, 45.0, 75.0);
                }
                for orb in [0.55f32, 0.8, 1.1, 1.5] {
                    if (r - orb).abs() < 0.012 {
                        c = rgb(40.0, 190.0, 240.0) * 0.85;
                    }
                }
                if r < 0.3 {
                    c = c + rgb(255.0, 170.0, 60.0) * ((0.3 - r) * 2.0);
                }
                if u < 0.012 || u > 0.988 || v < 0.017 || v > 0.983 {
                    c = cian();
                }
                emisivo(c)
            }
            Mat::Botonera => {
                let (cu, cv) = (frac(u * 5.0), frac(v * 4.0));
                let (ci, ri) = ((u * 5.0).floor(), (v * 4.0).floor());
                let mut c = rgb(12.0, 15.0, 22.0);
                if cu > 0.14 && cu < 0.86 && cv > 0.2 && cv < 0.8 {
                    let pal = [
                        rgb(55.0, 180.0, 200.0),
                        rgb(210.0, 155.0, 55.0),
                        rgb(170.0, 60.0, 62.0),
                        rgb(74.0, 145.0, 110.0),
                        rgb(122.0, 120.0, 162.0),
                    ];
                    let mut b = pal[((ci + ri * 2.0) as usize) % 5];
                    if hash(ci, ri) > 0.65 && (t * 3.0 + ci).sin() > 0.0 {
                        b = b * 1.5
                    } else {
                        b = b * 0.8
                    }
                    c = b;
                }
                Sup {
                    alb: rgb(30.0, 34.0, 44.0),
                    emi: c * 0.9,
                    esp: 0.2,
                    sin_luz: false,
                }
            }
            Mat::Cian => emisivo(cian() * (0.9 + 0.1 * (t * 2.0).sin())),
            Mat::Ambar => emisivo(rgb(255.0, 160.0, 40.0) * (0.75 + 0.25 * (t * 2.5).sin())),
            Mat::Rojo => emisivo(rgb(255.0, 45.0, 30.0) * (0.55 + 0.45 * (t * 4.0).sin().max(0.0))),
            Mat::Luz => emisivo(rgb(255.0, 246.0, 225.0)),
            Mat::Compuerta => {
                let mut a = 0.46;
                let (bu, bv) = (u.abs(), v.abs());
                if bu > 0.84 || bv > 2.15 {
                    a = 0.28;
                }
                if (bu - 0.7).abs() < 0.015 || (bv - 1.95).abs() < 0.015 {
                    a = 0.25;
                }
                let r2 = u * u + (v - 0.3) * (v - 0.3);
                if (r2.sqrt() - 0.34).abs() < 0.03 {
                    a = 0.2;
                }
                if v < -1.65 && v > -2.1 && bu < 0.84 {
                    return sup(
                        if frac((u + v) / 0.3) < 0.5 {
                            rgb(215.0, 170.0, 50.0)
                        } else {
                            rgb(20.0, 22.0, 24.0)
                        },
                        0.1,
                    );
                }
                sup(gris(a), 0.2)
            }
            Mat::Peligro => sup(
                if frac((u + v) / 0.4) < 0.5 {
                    rgb(225.0, 180.0, 40.0)
                } else {
                    rgb(24.0, 24.0, 26.0)
                },
                0.1,
            ),
            Mat::Pintura => sup(f.tinte, 0.3),
            Mat::Casco => sup(f.tinte, 0.55),
            Mat::Objeto => {
                let pulso = 1.1 + 0.3 * (t * 4.0).sin();
                emisivo(f.tinte * pulso)
            }
        }
    }
}

impl Shader for ShaderInterior {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let s = self.superficie(f, c.tiempo);
        if s.sin_luz {
            let col = s.emi;
            return [col.x, col.y, col.z, 1.0];
        }
        let vdir = (c.cam - f.pos).norm();
        let mut n = f.normal.norm();
        if n.dot(vdir) < 0.0 {
            n = -n;
        }
        let mut luz = self.ambiente;
        let mut esp = V3::ZERO;
        for l in self.luces.iter() {
            let d = l.pos - f.pos;
            let d2 = d.dot(d);
            let r2 = l.radio * l.radio;
            if d2 >= r2 {
                continue;
            }
            let dist = d2.sqrt().max(1e-4);
            let ld = d * (1.0 / dist);
            let k = 1.0 - d2 / r2;
            let att = k * k * l.inten;
            luz += l.color * (att * (0.3 + 0.7 * n.dot(ld).max(0.0)));
            if s.esp > 0.0 {
                let sp = n.dot((ld + vdir).norm()).max(0.0);
                let s2 = sp * sp;
                let s4 = s2 * s2;
                let s8 = s4 * s4;
                esp += l.color * (att * s8 * s8);
            }
        }
        let ao = 0.74 + 0.26 * smoothstep(0.0, 0.8, f.pos.y);
        let col = s.alb * luz * ao + esp * s.esp + s.emi;
        [col.x, col.y, col.z, 1.0]
    }
}
