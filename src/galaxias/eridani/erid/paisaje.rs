// Bahía de Erid: escenario del final y exploración libre. Una bahía de playa estilo Durdle Door:
// océano turquesa (misma técnica de ondas/Fresnel que ShaderAgua de Miller),
// playa de guijarros con franja de arena, acantilados de tiza con pasto arriba
// y un arco de roca a la izquierda. Mismo rasterizador que paisaje_miller.rs.
//
// Integración con el epílogo:
//   * `dibujar_con(.., extras)`: recibe los Item de Rocky, TARS y la nave
//     (con su propio shader de cuerpo) y se dibujan con el mismo z-buffer.
//   * `altura_suelo(x, z)` y `limites_pasillo(x)`: el "pasillo" caminable
//     a lo largo de la orilla (sin agua, sin acantilado).
//   * `ShaderRoca`: material de tiza estratificada del arco.
use crate::camara::Camara;
use crate::dibujo::{Item, Mezcla};
use crate::malla::Malla;
use crate::math::*;
use crate::raster::{preparar_item, rasterizar_banda, Framebuffer, Vista};
use crate::ruido::{celdas, fbm, ruido};
use crate::shader::{Contexto, Fragmento, Shader};
use crate::textura::nucleos;
use std::f32::consts::{PI, TAU};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Parámetros (ajustables sin tocar el resto)
// ---------------------------------------------------------------------------
const SOL: V3 = v3(-160.0, 150.0, -120.0);
const DISTANCIA_NIEBLA: f32 = 520.0;
const POS_ARCO: V3 = v3(-56.0, -0.8, -14.0);
pub const INICIO: V3 = v3(6.0, 2.2, 20.0);

fn horizonte() -> V3 {
    rgb(214.0, 234.0, 244.0)
}

// Línea de costa (curva suave: la bahía se mete hacia tierra en los extremos).
fn orilla(x: f32) -> f32 {
    10.0 + 0.003 * x * x
}

fn rugosidad(x: f32, z: f32) -> f32 {
    0.7 + 0.3 * (fbm(v3(x * 0.05, 0.0, z * 0.05), 4, 611) * 0.5 + 0.5)
}

// Altura del terreno: playa suave, fondo marino, acantilado trasero y dos promontorios.
pub fn altura_suelo(x: f32, z: f32) -> f32 {
    let d = z - orilla(x);
    let base = if d > 0.0 {
        d * 0.045 + ruido(v3(x * 0.9, 0.0, z * 0.9), 601) * 0.06 * smoothstep(0.0, 3.0, d)
    } else {
        (d * 0.09).max(-12.0)
    };
    let rug = rugosidad(x, z);
    let acantilado = 40.0 * smoothstep(34.0, 48.0, d) * rug + 0.08 * (d - 48.0).max(0.0);
    let izq = 30.0 * smoothstep(62.0, 80.0, -x) * smoothstep(-34.0, -8.0, z) * rug;
    let der = 24.0 * smoothstep(80.0, 100.0, x) * smoothstep(-6.0, 16.0, z) * rug;
    base + acantilado + izq + der
}

// Franja caminable (el pasillo): (z_min, z_max) para una x dada, entre la
// orilla y el pie del acantilado. x caminable válido: -58..76 (antes de los promontorios).
pub fn limites_pasillo(x: f32) -> (f32, f32) {
    let s = orilla(x);
    (s + 1.5, s + 30.0)
}

// ---------------------------------------------------------------------------
// Utilidades de color
// ---------------------------------------------------------------------------
fn sol_dir() -> V3 {
    SOL.norm()
}

fn cielo_color(d: V3) -> V3 {
    let h = clamp01(d.y);
    let base = mezcla(
        horizonte(),
        rgb(98.0, 158.0, 226.0),
        smoothstep(0.0, 0.7, h).powf(0.7),
    );
    let s = d.dot(sol_dir()).max(0.0);
    let c = base + rgb(255.0, 244.0, 220.0) * (s.powi(48) * 0.7 + s.powi(5) * 0.1);
    v3(c.x.min(1.0), c.y.min(1.0), c.z.min(1.0))
}

fn niebla(col: V3, dist: f32) -> V3 {
    let k = 1.0 - (-(dist / DISTANCIA_NIEBLA).powi(2)).exp();
    mezcla(col, horizonte(), k * 0.8)
}

// Luz suave: las sombras se enfrían un poco.
fn iluminar(col: V3, n: V3, p: V3, sol: V3) -> V3 {
    let k = smoothstep(-0.15, 0.55, n.dot((sol - p).norm()));
    mezcla(col * v3(0.70, 0.78, 0.95), col * 1.05, k)
}

// Tiza blanca con estratos y musgo; pasto verde en lo alto y plano.
fn color_acantilado(p: V3, n: V3) -> V3 {
    let estratos =
        ((p.y * 0.8) + fbm(v3(p.x * 0.06, p.y * 0.12, p.z * 0.06), 3, 631) * 3.0).sin() * 0.5 + 0.5;
    let tiza = mezcla(rgb(214.0, 210.0, 196.0), rgb(246.0, 244.0, 236.0), estratos);
    let musgo = smoothstep(0.1, 0.4, ruido(v3(p.x * 0.18, p.y * 0.03, p.z * 0.18), 632));
    let tiza = mezcla(tiza, rgb(150.0, 162.0, 118.0), musgo * 0.45);
    let hierba = mezcla(
        rgb(88.0, 134.0, 54.0),
        rgb(146.0, 176.0, 84.0),
        clamp01(fbm(v3(p.x * 0.08, 0.0, p.z * 0.08), 3, 633) * 0.5 + 0.5),
    );
    mezcla(
        tiza,
        hierba,
        smoothstep(0.62, 0.82, n.y) * smoothstep(6.0, 11.0, p.y),
    )
}

// ---------------------------------------------------------------------------
// Shaders
// ---------------------------------------------------------------------------

// Océano: ondas por gradiente de ruido, Fresnel y brillo solar como en Miller,
// pero turquesa claro en la orilla -> azul profundo mar adentro, con espuma de oleaje.
pub struct ShaderAguaBahia;

impl Shader for ShaderAguaBahia {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let p = f.pos;
        let t = c.tiempo;
        let dv = c.cam - p;
        let dist = dv.len().max(1e-3);
        let v = dv * (1.0 / dist);
        let detalle = 1.0 - smoothstep(25.0, 160.0, dist);

        // Rizado: el gradiente del ruido inclina la normal.
        let s = v3(p.x * 0.7, t * 0.35, p.z * 0.7);
        let e = 0.12;
        let h0 = ruido(s, 305);
        let gx = (ruido(s + v3(e, 0.0, 0.0), 305) - h0) / e;
        let gz = (ruido(s + v3(0.0, 0.0, e), 305) - h0) / e;
        let k = 0.14 * detalle + 0.02;
        let n = v3(-gx * k, 1.0, -gz * k).norm();

        // Distancia mar adentro desde la línea de costa.
        let d = orilla(p.x) - p.z;
        let somero = 1.0 - smoothstep(0.0, 25.0, d);
        let mut col = mezcla(
            rgb(150.0, 232.0, 214.0),
            rgb(70.0, 205.0, 205.0),
            smoothstep(0.0, 10.0, d),
        );
        col = mezcla(col, rgb(10.0, 110.0, 160.0), smoothstep(8.0, 70.0, d));

        // Líneas de luz (cáusticas) en aguas someras.
        let q = v3(p.x * 0.22, t * 0.08, p.z * 0.22);
        let n0 = fbm(q, 4, 301);
        let olas = ruido(q * 3.2 + v3(n0 * 2.3, n0 * 1.4, -n0 * 1.8), 302).abs();
        let linea = 1.0 - smoothstep(0.015, 0.075, olas);
        col = mezcla(
            col,
            rgb(200.0, 250.0, 235.0),
            linea * 0.35 * somero * detalle,
        );
        col = col * (0.85 + 0.15 * smoothstep(-0.2, 0.6, n.dot((c.sol - p).norm())));

        // Espuma: lengua de oleaje que avanza y retrocede en la orilla.
        let dd = d.max(0.0);
        let avance = 1.5
            + 1.4 * (t * 0.8 + p.x * 0.12).sin()
            + 0.8 * ruido(v3(p.x * 0.3, t * 0.3, 0.0), 310);
        let banda = 1.0 - smoothstep(0.0, 0.9, (dd - avance).abs());
        let lengua = 1.0 - smoothstep(0.0, avance.max(0.5), dd);
        let espuma = clamp01(banda * 0.8 + lengua * 0.5);
        col = mezcla(col, rgb(240.0, 252.0, 250.0), espuma * 0.85);

        // Reflejo del cielo (Fresnel) y destello del sol.
        let r = n * (2.0 * n.dot(v)) - v;
        let fres = 0.03 + 0.97 * (1.0 - n.dot(v).max(0.0)).powi(5);
        col = mezcla(
            col,
            cielo_color(v3(r.x, r.y.abs(), r.z)),
            fres * 0.8 * (1.0 - espuma),
        );
        let l = (c.sol - p).norm();
        let hv = (l + v).norm();
        col += v3(1.0, 0.97, 0.88) * (n.dot(hv).max(0.0).powf(220.0) * 1.4 * detalle.max(0.3));

        let col = niebla(col, dist);
        [col.x, col.y, col.z, 1.0]
    }
}

// Terreno: guijarros (celdas de Worley), franja de arena, orilla mojada,
// acantilados de tiza con pasto. Todo se decide por posición y pendiente.
pub struct ShaderCosta;

impl Shader for ShaderCosta {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let p = f.pos;
        let n = f.normal.norm();
        let dv = c.cam - p;
        let dist = dv.len().max(1e-3);
        let v = dv * (1.0 / dist);
        let d = p.z - orilla(p.x);

        let (f1, _, id) = celdas(p * 1.5, 621);
        let mut guijarro = mezcla(rgb(108.0, 106.0, 104.0), rgb(182.0, 176.0, 166.0), id);
        if id > 0.82 {
            guijarro = rgb(176.0, 148.0, 118.0);
        }
        guijarro =
            guijarro * (1.0 - 0.45 * smoothstep(0.30, 0.5, f1)) * (0.9 + 0.1 * ruido(p * 6.0, 622));
        let arena = rgb(216.0, 188.0, 140.0) * (0.92 + 0.08 * ruido(p * 2.0, 623));
        let t_arena = smoothstep(12.0, 20.0, d) * (1.0 - smoothstep(34.0, 42.0, d)) * 0.85;
        let mojado = 1.0 - smoothstep(0.0, 5.0, d);
        let playa = mezcla(guijarro, arena, t_arena) * (1.0 - 0.35 * mojado);

        let t_roca = smoothstep(2.0, 7.0, p.y);
        let base = if t_roca > 0.0 {
            mezcla(playa, color_acantilado(p, n), t_roca)
        } else {
            playa
        };
        let mut col = iluminar(base, n, p, c.sol);

        // Brillo de guijarros mojados.
        let hv = ((c.sol - p).norm() + v).norm();
        col += v3(1.0, 1.0, 1.0) * (n.dot(hv).max(0.0).powf(60.0) * 0.5 * mojado * (1.0 - t_roca));

        let col = niebla(col, dist);
        [col.x, col.y, col.z, 1.0]
    }
}

// Roca del arco, con estratos y algas bajo la línea de marea.
pub struct ShaderRoca;

impl Shader for ShaderRoca {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let p = f.pos;
        let dv = c.cam - p;
        let dist = dv.len().max(1e-3);
        let v = dv * (1.0 / dist);
        let mut n = f.normal.norm();
        if n.dot(v) < 0.0 {
            n = -n;
        } // malla a doble cara
        let mut col = color_acantilado(p, n);
        col = mezcla(
            col,
            rgb(150.0, 140.0, 125.0),
            0.35 * (1.0 - smoothstep(1.0, 8.0, p.y)),
        );
        // Línea de marea: oscura y con algas.
        col = mezcla(
            col,
            rgb(70.0, 92.0, 66.0),
            (1.0 - smoothstep(0.6, 2.0, p.y)) * 0.7,
        );
        let col = niebla(iluminar(col, n, p, c.sol), dist);
        [col.x, col.y, col.z, 1.0]
    }
}

// ---------------------------------------------------------------------------
// Mallas
// ---------------------------------------------------------------------------

// Océano: anillos concéntricos (triángulos pequeños cerca de la cámara).
fn malla_agua() -> Malla {
    const SECTORES: usize = 96;
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

// Terreno completo (playa + fondo + acantilados) como malla de alturas.
fn malla_terreno() -> Malla {
    let (nx, nz) = (150usize, 110usize);
    let (x0, x1, z0, z1) = (-150.0f32, 150.0f32, -60.0f32, 100.0f32);
    let (mut pos, mut nrm, mut uv) = (vec![], vec![], vec![]);
    let e = 0.5;
    for j in 0..=nz {
        let z = z0 + (z1 - z0) * j as f32 / nz as f32;
        for i in 0..=nx {
            let x = x0 + (x1 - x0) * i as f32 / nx as f32;
            let dx = (altura_suelo(x + e, z) - altura_suelo(x - e, z)) / (2.0 * e);
            let dz = (altura_suelo(x, z + e) - altura_suelo(x, z - e)) / (2.0 * e);
            pos.push(v3(x, altura_suelo(x, z), z));
            nrm.push(v3(-dx, 1.0, -dz).norm());
            uv.push([0.0, 0.0]);
        }
    }
    let mut idx = vec![];
    for j in 0..nz {
        for i in 0..nx {
            let a = (j * (nx + 1) + i) as u32;
            let b = a + (nx + 1) as u32;
            idx.push([a, a + 1, b]);
            idx.push([a + 1, b + 1, b]);
        }
    }
    Malla {
        pos,
        nrm,
        uv,
        idx,
        radio: 250.0,
    }
}

// Arco de roca: sección rectangular barrida en media vuelta (plano XY, grosor en Z),
// con los vértices desplazados por ruido y normales por cara (aspecto facetado).
fn malla_arco() -> Malla {
    const N: usize = 40;
    const M: usize = 12;
    let (r_in, r_out, hz) = (7.0f32, 14.0f32, 4.5f32);
    let esq = [(r_in, -hz), (r_out, -hz), (r_out, hz), (r_in, hz)];
    let mut sec = vec![];
    for e in 0..4 {
        let (a, b) = (esq[e], esq[(e + 1) % 4]);
        for s in 0..3 {
            let t = s as f32 / 3.0;
            sec.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
        }
    }
    let mut v = vec![vec![V3::ZERO; M]; N + 1];
    for i in 0..=N {
        let a = -0.08 + (PI + 0.16) * i as f32 / N as f32;
        let (s, c) = a.sin_cos();
        for (k, (r, z)) in sec.iter().enumerate() {
            let base = v3(r * c, r * s, *z);
            let off = 1.1 * ruido(base * 0.28, 641) + 0.35 * ruido(base * 0.9, 642);
            let dz = 0.9 * ruido(base * 0.5, 643);
            v[i][k] = base + v3(c, s, 0.0) * off + v3(0.0, 0.0, dz);
        }
    }
    let (mut pos, mut nrm, mut uv, mut idx) = (vec![], vec![], vec![], vec![]);
    for i in 0..N {
        for k in 0..M {
            let k1 = (k + 1) % M;
            let q = [v[i][k], v[i][k1], v[i + 1][k1], v[i + 1][k]];
            let n = (q[1] - q[0]).cross(q[2] - q[0]).norm();
            let b = pos.len() as u32;
            for p in q {
                pos.push(p);
                nrm.push(n);
                uv.push([0.0, 0.0]);
            }
            idx.push([b, b + 1, b + 2]);
            idx.push([b, b + 2, b + 3]);
        }
    }
    Malla {
        pos,
        nrm,
        uv,
        idx,
        radio: 30.0,
    }
}

// ---------------------------------------------------------------------------
// Escena
// ---------------------------------------------------------------------------
pub struct EscenaEridani {
    m_agua: Malla,
    m_terreno: Malla,
    m_arco: Malla,
    s_agua: ShaderAguaBahia,
    s_costa: ShaderCosta,
    s_roca: ShaderRoca,
    cielo: crate::textura::Textura,
}

impl EscenaEridani {
    pub fn nueva() -> EscenaEridani {
        EscenaEridani {
            m_agua: malla_agua(),
            m_terreno: malla_terreno(),
            m_arco: malla_arco(),
            s_agua: ShaderAguaBahia,
            s_costa: ShaderCosta,
            s_roca: ShaderRoca,
            cielo: crate::textura::Textura::generar(512, 256, &|d| {
                let mut col = cielo_color(d);
                if d.y > 0.0 {
                    let nube = fbm(v3(d.x * 2.2, d.y * 5.0, d.z * 2.2), 4, 920);
                    col = mezcla(
                        col,
                        rgb(255.0, 255.0, 255.0),
                        smoothstep(0.12, 0.5, nube) * 0.7 * smoothstep(0.03, 0.35, d.y),
                    );
                }
                col
            }),
        }
    }

    // De pie en la playa, mirando hacia el mar y el arco.
    pub fn camara(ancho: usize, alto: usize) -> Camara {
        let mut cam = Camara::nueva(ancho as f32, alto as f32);
        cam.pos = INICIO;
        cam.fov = 62f32.to_radians();
        cam.mirar_a(v3(-30.0, 3.0, -40.0));
        cam
    }

    fn items<'a>(&'a self) -> Vec<Item<'a>> {
        let unif = |s: f32| v3(s, s, s);
        let comun = |malla: &'a Malla, shader: &'a dyn Shader, pos: V3| Item {
            malla,
            shader,
            pos,
            rot: Rot::identidad(),
            escala: unif(1.0),
            mezcla: Mezcla::Opaco,
            doble_cara: true,
            tinte: unif(1.0),
            mip: 0.0,
            dist: 1e6,
        };
        vec![
            comun(&self.m_agua, &self.s_agua, V3::ZERO),
            comun(&self.m_terreno, &self.s_costa, V3::ZERO),
            comun(&self.m_arco, &self.s_roca, POS_ARCO),
        ]
    }

    pub fn dibujar(&self, fb: &mut Framebuffer, cam: &Camara, tiempo: f32) {
        self.dibujar_con(fb, cam, tiempo, vec![]);
    }

    // `extras`: Items adicionales (Rocky, etc.) que comparten z-buffer con la escena.
    pub fn dibujar_con<'a>(
        &'a self,
        fb: &mut Framebuffer,
        cam: &Camara,
        tiempo: f32,
        extras: Vec<Item<'a>>,
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
        dibujar_cielo(fb, &vista, &self.cielo);

        let mut items = self.items();
        items.extend(extras);
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

// Fondo: cielo azul claro con nubes sutiles, por píxel y en paralelo.
fn dibujar_cielo(fb: &mut Framebuffer, vista: &Vista, cielo: &crate::textura::Textura) {
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
                    let u = d.z.atan2(d.x) / TAU;
                    let v = 0.5 - d.y.clamp(-1.0, 1.0).asin() / PI;
                    let col = cielo.muestrear(u, v, 0.0);
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
    fn playa_con_mar_cielo_y_es_reproducible() {
        let e = EscenaEridani::nueva();
        let cam = EscenaEridani::camara(160, 100);
        let mut a = Framebuffer::new(160, 100);
        let mut b = Framebuffer::new(160, 100);
        e.dibujar(&mut a, &cam, 1.0);
        e.dibujar(&mut b, &cam, 1.0);
        assert_eq!(a.pixels, b.pixels);
        let cubierto = a.zbuf.iter().filter(|z| **z > 0.0).count();
        assert!(
            cubierto > 160 * 100 / 4 && cubierto < 160 * 100,
            "cubierto: {cubierto}"
        );
    }
    #[test]
    fn el_pasillo_queda_sobre_el_agua() {
        for x in [-58.0, 0.0, 76.0] {
            let (z0, z1) = limites_pasillo(x);
            assert!(altura_suelo(x, z0) > 0.0 && altura_suelo(x, z1) < 4.0);
        }
    }
}
