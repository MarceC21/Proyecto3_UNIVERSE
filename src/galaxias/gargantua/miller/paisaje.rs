// Escena visual del planeta Miller: mar poco profundo, la gran ola convertida en
// una pared que rodea al jugador, cielo blanco y cajas flotando a la deriva.
// La misión controla el avance de la ola y las marcas de inspección. Usa el rasterizador del motor
// (recorte, z-buffer, interpolación) igual que paisaje_marte.rs.
//
// La pared y el suelo usan EL MISMO shader (ShaderAgua) y el mismo ruido en
// coordenadas de mundo, por eso la ola se ve del mismo "material" que el agua.
use crate::camara::Camara;
use crate::dibujo::{Item, Mezcla};
use crate::malla::{self, Malla};
use crate::math::*;
use crate::raster::{preparar_item, rasterizar_banda, Framebuffer, Vista};
use crate::ruido::{fbm, ruido, Rng};
use crate::shader::{Contexto, Fragmento, Shader};
use crate::textura::nucleos;
use std::f32::consts::TAU;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Parámetros para ajustar sin tocar el resto del código
// ---------------------------------------------------------------------------
const SOL: V3 = v3(-260.0, 230.0, -460.0); // luz detrás de la ola (contraluz)
const RADIO_PARED: f32 = 200.0; // distancia de la pared al jugador
const ALTO_FRENTE: f32 = 80.0; // altura de la ola frente a la cámara (-Z)
const ALTO_ATRAS: f32 = 45.0; // altura en la parte de atrás del anillo
const INCLINACION: f32 = 32.0; // cuánto se "dobla" la cresta hacia el jugador
const NUM_CAJAS: usize = 3;
const SEMILLA_CAJAS: u64 = 3141;
const DISTANCIA_NIEBLA: f32 = 380.0;

fn sol_dir() -> V3 {
    SOL.norm()
}
fn horizonte() -> V3 {
    rgb(250.0, 252.0, 252.0)
}

// Cielo blanco con un cenit apenas azulado y un halo suave hacia el sol.
fn cielo_color(d: V3) -> V3 {
    let h = clamp01(d.y);
    let base = mezcla(
        horizonte(),
        rgb(205.0, 224.0, 232.0),
        smoothstep(0.0, 0.75, h).powf(0.8),
    );
    let s = d.dot(sol_dir()).max(0.0);
    let c = base + rgb(255.0, 248.0, 225.0) * (s.powi(64) * 0.8 + s.powi(6) * 0.12);
    v3(c.x.min(1.0), c.y.min(1.0), c.z.min(1.0))
}

fn niebla(col: V3, dist: f32) -> V3 {
    let k = 1.0 - (-(dist / DISTANCIA_NIEBLA).powi(2)).exp();
    mezcla(col, horizonte(), k)
}

// ---------------------------------------------------------------------------
// Shaders
// ---------------------------------------------------------------------------

// Agua y pared de agua. Todo el patrón sale de la posición en el MUNDO, así que
// donde la pared nace del suelo la textura continúa sin costura.
// uv[1] vale 0 en el suelo y 0..1 (altura relativa) en la pared.
pub struct ShaderAgua;

impl Shader for ShaderAgua {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let p = f.pos;
        let t = c.tiempo;
        let dv = c.cam - p;
        let dist = dv.len().max(1e-3);
        let v = dv * (1.0 / dist);
        let mut n = f.normal.norm();
        if n.dot(v) < 0.0 {
            n = -n;
        } // superficies a doble cara
          // El detalle fino se apaga con la distancia (evita parpadeo).
        let detalle = 1.0 - smoothstep(25.0, 140.0, dist);

        // Rizado: el gradiente del ruido 3D inclina un poco la normal.
        let s = v3(p.x * 0.8, p.y * 0.8 + t * 0.35, p.z * 0.8);
        let e = 0.12;
        let h0 = ruido(s, 305);
        let g = v3(
            ruido(s + v3(e, 0.0, 0.0), 305) - h0,
            ruido(s + v3(0.0, e, 0.0), 305) - h0,
            ruido(s + v3(0.0, 0.0, e), 305) - h0,
        ) * (1.0 / e);
        let gt = g - n * g.dot(n);
        n = (n + gt * (0.16 * detalle + 0.02)).norm();

        // Color: mismas ideas que la textura de Miller (agua turquesa + líneas de espuma).
        let q = v3(p.x * 0.22, p.y * 0.22 + t * 0.08, p.z * 0.22);
        let n0 = fbm(q, 4, 301);
        let olas = ruido(q * 3.2 + v3(n0 * 2.3, n0 * 1.4, -n0 * 1.8), 302).abs();
        let linea = 1.0 - smoothstep(0.015, 0.075, olas);
        let espuma = 0.18 + (linea - 0.18) * detalle;

        let mut col = mezcla(
            rgb(17.0, 73.0, 110.0),
            rgb(60.0, 157.0, 176.0),
            clamp01(n0 * 0.45 + 0.5),
        );
        // La parte alta de la ola deja pasar la luz (contraluz turquesa).
        let uvy = f.uv[1];
        col = mezcla(
            col,
            rgb(120.0, 208.0, 206.0),
            smoothstep(0.45, 1.0, uvy) * 0.65,
        );

        let l = (c.sol - p).norm();
        col = col * (0.62 + 0.38 * smoothstep(-0.2, 0.6, n.dot(l)));
        col = mezcla(col, rgb(226.0, 244.0, 240.0), espuma * 0.85);
        // Cresta espumosa en el borde superior de la pared.
        col = mezcla(
            col,
            rgb(240.0, 252.0, 250.0),
            smoothstep(0.93, 1.0, uvy) * (0.5 + 0.5 * espuma),
        );

        // Reflejo del cielo (Fresnel): lejos y rasante el agua se vuelve blanca.
        let r = n * (2.0 * n.dot(v)) - v;
        let fres = 0.03 + 0.97 * (1.0 - n.dot(v).max(0.0)).powi(5);
        let cielo = cielo_color(v3(r.x, r.y.abs(), r.z));
        col = mezcla(col, cielo, fres * (1.0 - espuma * 0.7));

        // Destello del sol sobre las ondas.
        let hv = (l + v).norm();
        col += v3(1.0, 0.97, 0.88) * (n.dot(hv).max(0.0).powf(220.0) * 1.4 * detalle.max(0.3));

        let col = niebla(col, dist);
        [col.x, col.y, col.z, 1.0]
    }
}

// Cajas flotantes. tipo 0 = módulo técnico gris con pantalla cian (la "correcta"),
// 1 = caja militar verde con correa, 2 = caja de madera con herrajes.
pub struct ShaderCaja {
    pub tipo: u8,
}

impl Shader for ShaderCaja {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let p = f.pos;
        let dv = c.cam - p;
        let dist = dv.len().max(1e-3);
        let v = dv * (1.0 / dist);
        let mut n = f.normal.norm();
        if n.dot(v) < 0.0 {
            n = -n;
        }
        let (u, w) = (f.uv[0], f.uv[1]);
        let borde = u.min(1.0 - u).min(w.min(1.0 - w));
        let mut emisivo: Option<V3> = None;

        let col = match self.tipo {
            0 => {
                let mut k = 1.0;
                if borde < 0.07 {
                    k = 0.62;
                } else if borde < 0.10 {
                    k = 1.15;
                }
                if (w - 0.5).abs() < 0.012 {
                    k *= 0.75;
                }
                if u > 0.28 && u < 0.72 && w > 0.36 && w < 0.64 {
                    emisivo =
                        Some(rgb(80.0, 225.0, 240.0) * (0.85 + 0.15 * (c.tiempo * 3.0).sin()));
                }
                if w > 0.80 && w < 0.87 {
                    if u > 0.10 && u < 0.17 {
                        emisivo = Some(rgb(230.0, 60.0, 50.0));
                    }
                    if u > 0.20 && u < 0.27 {
                        emisivo = Some(rgb(70.0, 230.0, 120.0));
                    }
                }
                rgb(112.0, 122.0, 128.0) * k
            }
            1 => {
                let mut k = 1.0;
                if borde < 0.06 {
                    k = 0.6;
                } else if borde < 0.09 {
                    k = 1.12;
                }
                let mut col = rgb(88.0, 100.0, 68.0) * k;
                if (u - 0.5).abs() < 0.05 {
                    col = rgb(52.0, 58.0, 46.0);
                } // correa
                if w > 0.62 && w < 0.72 && u > 0.38 && u < 0.62 {
                    col = rgb(40.0, 44.0, 40.0);
                } // asa
                col
            }
            _ => {
                let tabla = (w * 5.0).fract();
                let mut k = 1.0 + 0.05 * (u * 37.0 + w * 5.0).sin();
                if tabla < 0.06 {
                    k *= 0.55;
                }
                let mut col = rgb(128.0, 98.0, 64.0) * k;
                if u < 0.09 || u > 0.91 {
                    // herrajes metálicos
                    col = rgb(112.0, 114.0, 110.0) * if borde < 0.015 { 0.7 } else { 1.0 };
                }
                col
            }
        } * f.tinte;

        let l = (c.sol - p).norm();
        let luz = 0.50 + 0.50 * smoothstep(-0.1, 0.5, n.dot(l));
        let ambiente = mezcla(
            rgb(150.0, 175.0, 185.0),
            rgb(235.0, 245.0, 248.0),
            n.y * 0.5 + 0.5,
        );
        let mut out = col * ambiente * luz * 1.15;
        let hv = (l + v).norm();
        let espec = if self.tipo == 0 { 0.35 } else { 0.10 };
        out += v3(1.0, 1.0, 1.0) * (n.dot(hv).max(0.0).powf(40.0) * espec);
        out += rgb(200.0, 235.0, 245.0) * ((1.0 - n.dot(v).max(0.0)).powi(3) * 0.12);
        // Parte mojada junto a la línea de agua.
        out = out * (1.0 - 0.35 * (1.0 - smoothstep(0.0, 0.35, p.y)));
        if let Some(e) = emisivo {
            out = e;
        }
        let out = niebla(out, dist);
        [out.x, out.y, out.z, 1.0]
    }
}

// Aro de espuma que rodea a cada caja en la superficie del agua.
pub struct ShaderEspumaCaja;

impl Shader for ShaderEspumaCaja {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let borde = 1.0 - (f.uv[1] - 0.5).abs() * 2.0;
        let ang = (f.pos.z - f.centro.z).atan2(f.pos.x - f.centro.x);
        let trazo = 0.5 + 0.5 * (ang * 7.0 + c.tiempo * 1.4).sin();
        let a = borde.max(0.0).powf(1.6) * (0.25 + 0.5 * trazo) * 0.7;
        [0.92, 0.98, 0.97, a]
    }
}

// ---------------------------------------------------------------------------
// Mallas
// ---------------------------------------------------------------------------

// Suelo de agua: malla polar (anillos concéntricos) para que los triángulos
// sean pequeños cerca de la cámara y no cubran media pantalla.
fn malla_agua() -> Malla {
    const SECTORES: usize = 72;
    let radios = [
        0.0f32, 4.0, 9.0, 16.0, 26.0, 40.0, 60.0, 90.0, 130.0, 190.0, 280.0, 420.0,
    ];
    let n = SECTORES;
    let mut pos = vec![v3(0.0, 0.0, 0.0)];
    let mut idx: Vec<[u32; 3]> = vec![];
    for &r in &radios[1..] {
        for s in 0..n {
            let a = s as f32 / n as f32 * TAU;
            pos.push(v3(r * a.cos(), 0.0, r * a.sin()));
        }
    }
    for s in 0..n {
        idx.push([0, (1 + s) as u32, (1 + (s + 1) % n) as u32]);
    }
    for k in 1..radios.len() - 1 {
        let (r0, r1) = (1 + (k - 1) * n, 1 + k * n);
        for s in 0..n {
            let s1 = (s + 1) % n;
            let (a, b, c, d) = (
                (r0 + s) as u32,
                (r0 + s1) as u32,
                (r1 + s1) as u32,
                (r1 + s) as u32,
            );
            idx.push([a, b, c]);
            idx.push([a, c, d]);
        }
    }
    let nrm = vec![v3(0.0, 1.0, 0.0); pos.len()];
    let uv = vec![[0.0, 0.0]; pos.len()];
    Malla {
        pos,
        nrm,
        uv,
        idx,
        radio: 420.0,
    }
}

// La ola: un anillo completo alrededor del jugador, más alto al frente (-Z),
// con radio y altura irregulares (ruido) y la cresta inclinada hacia adentro.
fn malla_pared() -> Malla {
    const SEG: usize = 192;
    const FILAS: usize = 28;
    let radio_en = |_a: f32| RADIO_PARED;
    let alto_en = |a: f32| {
        ALTO_ATRAS
            + (ALTO_FRENTE - ALTO_ATRAS) * smoothstep(-0.2, 0.9, a.cos())
            + 14.0 * fbm(v3(a.cos() * 2.0, a.sin() * 2.0, 7.0), 3, 912)
    };
    // a = 0 apunta hacia -Z (donde mira la cámara al inicio).
    let punto = |i: usize, j: usize| -> V3 {
        let a = (i % SEG) as f32 / SEG as f32 * TAU;
        let t = j as f32 / FILAS as f32;
        let alto = alto_en(a);
        let r = radio_en(a) - INCLINACION * (alto / ALTO_FRENTE) * t.powf(2.2);
        // La base arranca bajo el agua para que no se vea ninguna junta.
        v3(r * a.sin(), -4.0 + (alto + 4.0) * t, -r * a.cos())
    };
    let normal = |i: usize, j: usize| -> V3 {
        let du = punto(i + 1, j) - punto(i + SEG - 1, j);
        let dv = punto(i, (j + 1).min(FILAS)) - punto(i, j.saturating_sub(1));
        du.cross(dv).norm()
    };
    let (mut pos, mut nrm, mut uv, mut idx) = (vec![], vec![], vec![], vec![]);
    for i in 0..=SEG {
        for j in 0..=FILAS {
            pos.push(punto(i, j));
            nrm.push(normal(i % SEG, j));
            uv.push([i as f32 / SEG as f32, j as f32 / FILAS as f32]);
        }
    }
    let id = |i: usize, j: usize| (i * (FILAS + 1) + j) as u32;
    for i in 0..SEG {
        for j in 0..FILAS {
            let (a, b, c, d) = (id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1));
            idx.push([a, b, c]);
            idx.push([a, c, d]);
        }
    }
    Malla {
        pos,
        nrm,
        uv,
        idx,
        radio: 400.0,
    }
}

// Cubo de lado 2 con normal y uv por cara (cada cara pinta su panel completo).
fn malla_caja() -> Malla {
    let caras = [
        (v3(1.0, 0.0, 0.0), v3(0.0, 0.0, -1.0), v3(0.0, 1.0, 0.0)),
        (v3(-1.0, 0.0, 0.0), v3(0.0, 0.0, 1.0), v3(0.0, 1.0, 0.0)),
        (v3(0.0, 1.0, 0.0), v3(1.0, 0.0, 0.0), v3(0.0, 0.0, -1.0)),
        (v3(0.0, -1.0, 0.0), v3(1.0, 0.0, 0.0), v3(0.0, 0.0, 1.0)),
        (v3(0.0, 0.0, 1.0), v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0)),
        (v3(0.0, 0.0, -1.0), v3(-1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0)),
    ];
    let (mut pos, mut nrm, mut uv, mut idx) = (vec![], vec![], vec![], vec![]);
    for (n, u, v) in caras {
        let base = pos.len() as u32;
        for (su, sv) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            pos.push(n + u * su + v * sv);
            nrm.push(n);
            uv.push([(su + 1.0) * 0.5, (1.0 - sv) * 0.5]);
        }
        idx.push([base, base + 1, base + 2]);
        idx.push([base, base + 2, base + 3]);
    }
    Malla {
        pos,
        nrm,
        uv,
        idx,
        radio: 1.8,
    }
}

// ---------------------------------------------------------------------------
// Cajas flotantes
// ---------------------------------------------------------------------------
struct Caja {
    x: f32,
    z: f32,
    mitad: V3, // semidimensiones
    yaw: f32,
    tipo: u8,
    fase: f32,
    vel: f32,
    tinte: f32,
}

impl Caja {
    fn nueva(rng: &mut Rng, tipo: u8, x: f32, z: f32, escala: f32) -> Caja {
        let base = match tipo {
            0 => v3(1.15, 0.70, 0.80),
            1 => v3(0.95, 0.62, 0.62),
            _ => v3(0.75, 0.58, 0.75),
        };
        let k = escala * rng.rango(0.85, 1.25);
        Caja {
            x,
            z,
            mitad: base * k,
            yaw: rng.rango(0.0, TAU),
            tipo,
            fase: rng.rango(0.0, TAU),
            vel: rng.rango(0.6, 1.2),
            tinte: rng.rango(0.88, 1.08),
        }
    }
}

fn generar_cajas() -> Vec<Caja> {
    let mut rng = Rng::new(SEMILLA_CAJAS);
    let cajas = crate::mision_miller::CAJAS
        .iter()
        .enumerate()
        .map(|(i, p)| Caja::nueva(&mut rng, i as u8, p.x, p.z, 1.0))
        .collect();
    cajas
}

// ---------------------------------------------------------------------------
// Escena
// ---------------------------------------------------------------------------
// La escala solo afecta la nave de superficie, conservando sus mallas y shaders.
pub const ESCALA_NAVE_SUPERFICIE: f32 = 5.0;

pub fn pose_aterrizaje(t: f32) -> crate::nave::PoseNave {
    let t = t.clamp(0.0, 1.0);
    let avance = smoothstep(0.0, 0.85, t);
    let descenso = smoothstep(0.1, 0.92, t);
    let inclinacion = -0.15 * (1.0 - descenso);
    crate::nave::PoseNave {
        pos: v3(
            -9.0 * (1.0 - avance),
            3.0 + 24.0 * (1.0 - descenso),
            8.0 + 38.0 * (1.0 - avance),
        ),
        fwd: v3(0.0, inclinacion, -1.0).norm(),
        up: v3(0.0, 1.0, inclinacion).norm(),
        empuje: 1.1 * (1.0 - smoothstep(0.72, 1.0, t)),
    }
}

pub struct EscenaMiller {
    m_agua: Malla,
    m_pared: Malla,
    m_caja: Malla,
    m_aro: Malla,
    s_agua: ShaderAgua,
    s_cajas: [ShaderCaja; 3],
    s_espuma: ShaderEspumaCaja,
    cajas: Vec<Caja>,
    pub radio_ola: f32,
    pub inspeccionadas: [bool; NUM_CAJAS],
}

impl EscenaMiller {
    pub fn nueva() -> EscenaMiller {
        EscenaMiller {
            m_agua: malla_agua(),
            m_pared: malla_pared(),
            m_caja: malla_caja(),
            m_aro: malla::anillo(0.9, 1.9, 48, 6.0),
            s_agua: ShaderAgua,
            s_cajas: [
                ShaderCaja { tipo: 0 },
                ShaderCaja { tipo: 1 },
                ShaderCaja { tipo: 2 },
            ],
            s_espuma: ShaderEspumaCaja,
            cajas: generar_cajas(),
            radio_ola: RADIO_PARED,
            inspeccionadas: [false; NUM_CAJAS],
        }
    }

    // Cámara a la altura de los ojos, mirando un poco hacia arriba para ver
    // la ola, el cielo y el agua a la vez.
    pub fn camara(ancho: usize, alto: usize) -> Camara {
        let mut cam = Camara::nueva(ancho as f32, alto as f32);
        cam.pos = crate::mision_miller::INICIO;
        cam.fov = 62f32.to_radians();
        cam.mirar_a(v3(0.0, 15.0, -100.0));
        cam
    }

    fn items<'a>(&'a self, cam: &Camara, t: f32) -> Vec<Item<'a>> {
        let unif = |s: f32| v3(s, s, s);
        let mut items = vec![
            Item {
                malla: &self.m_agua,
                shader: &self.s_agua,
                pos: V3::ZERO,
                rot: Rot::identidad(),
                escala: unif(1.0),
                mezcla: Mezcla::Opaco,
                doble_cara: true,
                tinte: unif(1.0),
                mip: 0.0,
                dist: 1e6,
            },
            Item {
                malla: &self.m_pared,
                shader: &self.s_agua,
                pos: V3::ZERO,
                rot: Rot::identidad(),
                escala: v3(
                    self.radio_ola / RADIO_PARED,
                    1.0,
                    self.radio_ola / RADIO_PARED,
                ),
                mezcla: Mezcla::Opaco,
                doble_cara: true,
                tinte: unif(1.0),
                mip: 0.0,
                dist: 1e6,
            },
        ];
        let mut aros: Vec<Item<'a>> = vec![];
        for (i, c) in self.cajas.iter().enumerate() {
            // Se mecen con el oleaje: subida/bajada y un leve cabeceo y alabeo.
            let bob = 0.07 * (t * c.vel + c.fase).sin();
            let cab = 0.05 * (t * 0.7 * c.vel + c.fase * 1.7).sin();
            let alab = 0.07 * (t * 0.9 * c.vel + c.fase * 0.6).sin();
            let rot = Rot::rot_y(c.yaw)
                .mul(&Rot::rot_x(cab))
                .mul(&Rot::rot_z(alab));
            // Flotan con ~1/3 de su volumen bajo el agua.
            let pos = v3(c.x, c.mitad.y * 0.35 + bob, c.z);
            let dist = (pos - cam.pos).len();
            items.push(Item {
                malla: &self.m_caja,
                shader: &self.s_cajas[c.tipo as usize],
                pos,
                rot,
                escala: c.mitad,
                mezcla: Mezcla::Opaco,
                doble_cara: true,
                tinte: if self.inspeccionadas[i] {
                    v3(0.3, 0.95, 0.65)
                } else {
                    unif(c.tinte)
                },
                mip: 0.0,
                dist,
            });
            let diag = (c.mitad.x * c.mitad.x + c.mitad.z * c.mitad.z).sqrt();
            aros.push(Item {
                malla: &self.m_aro,
                shader: &self.s_espuma,
                pos: v3(c.x, 0.03, c.z),
                rot: Rot::identidad(),
                escala: unif(diag),
                mezcla: Mezcla::Alfa,
                doble_cara: true,
                tinte: unif(1.0),
                mip: 0.0,
                dist,
            });
        }
        // Transparentes al final, de lejos a cerca.
        aros.sort_by(|a, b| b.dist.total_cmp(&a.dist));
        items.extend(aros);
        items
    }

    pub fn colisionadores(&self) -> Vec<(V3, f32)> {
        self.cajas
            .iter()
            .map(|c| {
                (
                    v3(c.x, 1.7, c.z),
                    (c.mitad.x * c.mitad.x + c.mitad.z * c.mitad.z).sqrt(),
                )
            })
            .collect()
    }

    pub fn dibujar(&self, fb: &mut Framebuffer, cam: &Camara, tiempo: f32) {
        self.dibujar_con_nave(fb, cam, tiempo, None);
    }

    pub fn dibujar_con_nave(
        &self,
        fb: &mut Framebuffer,
        cam: &Camara,
        tiempo: f32,
        nave: Option<&crate::nave::Nave>,
    ) {
        self.dibujar_pose(fb, cam, tiempo, nave.map(|n| (n, pose_aterrizaje(1.0))));
    }

    pub fn dibujar_descenso(&self, fb: &mut Framebuffer, nave: &crate::nave::Nave, t: f32) {
        let mut cam = Camara::nueva(fb.width as f32, fb.height as f32);
        let acercamiento = smoothstep(0.15, 1.0, t);
        cam.pos = mezcla(v3(42.0, 18.0, 65.0), v3(18.0, 8.0, 28.0), acercamiento);
        cam.mirar_a(mezcla(v3(0.0, 9.0, 8.0), v3(0.0, 3.0, 8.0), acercamiento));
        self.dibujar_pose(fb, &cam, t * 5.0, Some((nave, pose_aterrizaje(t))));
    }

    fn dibujar_pose(
        &self,
        fb: &mut Framebuffer,
        cam: &Camara,
        tiempo: f32,
        nave: Option<(&crate::nave::Nave, crate::nave::PoseNave)>,
    ) {
        let (w, h) = (fb.width, fb.height);
        let vista = Vista::de(cam, w, h);
        let ctx = Contexto {
            cam: cam.pos,
            tiempo,
            sol: SOL,
        };
        let hilos = nucleos();

        fb.zbuf.fill(0.0);
        dibujar_cielo(fb, &vista, tiempo);

        let mut items = self.items(cam, tiempo);
        if let Some((nave, pose)) = nave {
            let inicio = items.len();
            nave.emitir_pose(cam, pose, &mut items, &mut crate::dibujo::Stats::default());
            for item in &mut items[inicio..] {
                item.pos = pose.pos + (item.pos - pose.pos) * ESCALA_NAVE_SUPERFICIE;
                item.escala = item.escala * ESCALA_NAVE_SUPERFICIE;
                item.dist = (item.pos - cam.pos).len();
            }
        }
        let mut tris = vec![];
        for (id, item) in items.iter().enumerate() {
            preparar_item(item, id as u32, &vista, &mut tris);
        }

        const BANDA: usize = 20;
        let trabajos = Mutex::new(
            fb.pixels
                .chunks_mut(w * 4 * BANDA)
                .zip(fb.zbuf.chunks_mut(w * BANDA))
                .enumerate(),
        );
        std::thread::scope(|s| {
            for _ in 0..hilos {
                s.spawn(|| loop {
                    let sig = trabajos.lock().unwrap().next();
                    let Some((b, (color, z))) = sig else { break };
                    rasterizar_banda(b * BANDA, color, z, w, &tris, &items, &ctx);
                });
            }
        });
    }
}

// Fondo: cielo blanco con nubes muy sutiles, calculado por píxel en paralelo.
fn dibujar_cielo(fb: &mut Framebuffer, vista: &Vista, tiempo: f32) {
    let (w, h) = (fb.width, fb.height);
    let (tan_x, tan_y) = (1.0 / vista.fx, 1.0 / vista.fy);
    let filas = h.div_ceil(nucleos()).max(1);
    std::thread::scope(|s| {
        for (ci, trozo) in fb.pixels.chunks_mut(filas * w * 4).enumerate() {
            s.spawn(move || {
                for (i, px) in trozo.chunks_exact_mut(4).enumerate() {
                    let (x, y) = (i % w, ci * filas + i / w);
                    let nx = 2.0 * (x as f32 + 0.5) / w as f32 - 1.0;
                    let ny = 1.0 - 2.0 * (y as f32 + 0.5) / h as f32;
                    let d = (vista.f + vista.r * (nx * tan_x) + vista.u * (ny * tan_y)).norm();
                    let mut col = cielo_color(d);
                    if d.y > 0.0 {
                        let nube =
                            fbm(v3(d.x * 1.8 + tiempo * 0.004, d.y * 4.0, d.z * 1.8), 3, 920);
                        col = mezcla(
                            col,
                            rgb(214.0, 227.0, 233.0),
                            smoothstep(0.05, 0.5, nube) * 0.45 * smoothstep(0.02, 0.4, d.y),
                        );
                    }
                    px[0] = (clamp01(col.x) * 255.0) as u8;
                    px[1] = (clamp01(col.y) * 255.0) as u8;
                    px[2] = (clamp01(col.z) * 255.0) as u8;
                    px[3] = 255;
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aterrizaje_muestra_nave_grande_sin_cambiar_la_nave_de_vuelo() {
        let e = EscenaMiller::nueva();
        let nave = crate::nave::Nave::predeterminada().unwrap();
        let original = nave.pos;
        let mut inicio = Framebuffer::new(320, 200);
        let mut fin = Framebuffer::new(320, 200);
        e.dibujar_descenso(&mut inicio, &nave, 0.1);
        e.dibujar_descenso(&mut fin, &nave, 1.0);
        assert_ne!(inicio.pixels, fin.pixels);
        assert!(pose_aterrizaje(0.0).pos.y > pose_aterrizaje(1.0).pos.y + 20.0);
        assert_eq!(pose_aterrizaje(1.0).empuje, 0.0);
        assert_eq!(ESCALA_NAVE_SUPERFICIE, 5.0);
        assert_eq!(nave.pos, original);
    }

    #[test]
    fn miller_tiene_agua_pared_y_cajas_y_es_reproducible() {
        let e = EscenaMiller::nueva();
        assert_eq!(e.cajas.len(), 3);
        let cam = EscenaMiller::camara(160, 100);
        let mut a = Framebuffer::new(160, 100);
        let mut b = Framebuffer::new(160, 100);
        e.dibujar(&mut a, &cam, 1.0);
        e.dibujar(&mut b, &cam, 1.0);
        assert_eq!(a.pixels, b.pixels);
        let cubierto = a.zbuf.iter().filter(|z| **z > 0.0).count();
        assert!(
            cubierto > 160 * 100 / 5 && cubierto < 160 * 100,
            "cubierto: {cubierto}"
        );
    }
}
