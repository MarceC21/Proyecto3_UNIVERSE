// Rasterizador por software.
//
// Pipeline de un objeto:
//   1. Vértices: modelo -> mundo -> espacio de la cámara -> "clip" (x, y, w).
//   2. Recorte (clipping) contra el plano cercano y una banda de guarda lateral.
//   3. División por w y transformación a píxeles. Descarte de caras traseras.
//   4. Rasterización por bandas de filas (una por hilo): coordenadas
//      baricéntricas, prueba de profundidad con 1/w, interpolación de atributos
//      con CORRECCIÓN DE PERSPECTIVA y llamada al fragment shader.
use crate::camara::Camara;
use crate::dibujo::{Item, Mezcla};
use crate::math::*;
use crate::shader::{Contexto, Fragmento};

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>, // RGBA
    pub zbuf: Vec<f32>,  // 1/w: mayor = más cerca
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Framebuffer {
        Framebuffer { width, height, pixels: vec![255; width * height * 4], zbuf: vec![0.0; width * height] }
    }
    pub fn redimensionar(&mut self, width: usize, height: usize) {
        if width == self.width && height == self.height { return; }
        *self = Framebuffer::new(width, height);
    }
}

const CERCA: f32 = 0.05;
const GUARDA: f32 = 3.0; // la banda de guarda evita coordenadas gigantescas de pantalla

// Vértice en espacio clip con sus atributos: pos(3) mundo, normal(3), uv(2).
#[derive(Clone, Copy)]
struct CV {
    x: f32,
    y: f32,
    w: f32,
    a: [f32; 8],
}

// Vértice ya en pantalla; los atributos vienen multiplicados por 1/w.
#[derive(Clone, Copy)]
pub struct SV {
    pub x: f32,
    pub y: f32,
    pub iw: f32,
    pub a: [f32; 8],
}

pub struct Tri {
    pub v: [SV; 3],
    pub item: u32,
    pub min_y: f32,
    pub max_y: f32,
    pub inv_area: f32,
}

fn dist_plano(k: usize, c: &CV) -> f32 {
    match k {
        0 => c.w - CERCA,
        1 => GUARDA * c.w - c.x,
        2 => GUARDA * c.w + c.x,
        3 => GUARDA * c.w - c.y,
        _ => GUARDA * c.w + c.y,
    }
}

fn interpolar(a: &CV, b: &CV, t: f32) -> CV {
    let mut r = *a;
    r.x += (b.x - a.x) * t;
    r.y += (b.y - a.y) * t;
    r.w += (b.w - a.w) * t;
    for k in 0..8 { r.a[k] += (b.a[k] - a.a[k]) * t; }
    r
}

// Sutherland–Hodgman contra un plano.
fn recortar(poly: &mut Vec<CV>, tmp: &mut Vec<CV>, k: usize) {
    tmp.clear();
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (da, db) = (dist_plano(k, &a), dist_plano(k, &b));
        if da >= 0.0 { tmp.push(a); }
        if (da >= 0.0) != (db >= 0.0) { tmp.push(interpolar(&a, &b, da / (da - db))); }
    }
    std::mem::swap(poly, tmp);
}

// Datos de la cámara que necesita la etapa de vértices.
pub struct Vista {
    pub eye: V3,
    pub f: V3,
    pub r: V3,
    pub u: V3,
    pub fx: f32,
    pub fy: f32,
    pub w: f32,
    pub h: f32,
}

impl Vista {
    pub fn de(cam: &Camara, w: usize, h: usize) -> Vista {
        let (f, r, u) = cam.base();
        let (fx, fy) = cam.factores();
        Vista { eye: cam.pos, f, r, u, fx, fy, w: w as f32, h: h as f32 }
    }
}

// Etapa de vértices + recorte + descarte. Devuelve los triángulos listos para pintar.
pub fn preparar_item(item: &Item, id: u32, vista: &Vista, out: &mut Vec<Tri>) {
    let m = item.malla;
    let mut cvs: Vec<CV> = Vec::with_capacity(m.pos.len());
    for i in 0..m.pos.len() {
        let p = m.pos[i];
        let local = v3(p.x * item.escala.x, p.y * item.escala.y, p.z * item.escala.z);
        let mundo = item.pos + item.rot.aplicar(local);
        let n = item.rot.aplicar(m.nrm[i]);
        let d = mundo - vista.eye;
        let w = d.dot(vista.f);
        cvs.push(CV {
            x: d.dot(vista.r) * vista.fx,
            y: d.dot(vista.u) * vista.fy,
            w,
            a: [mundo.x, mundo.y, mundo.z, n.x, n.y, n.z, m.uv[i][0], m.uv[i][1]],
        });
    }

    let mut poly: Vec<CV> = Vec::with_capacity(8);
    let mut tmp: Vec<CV> = Vec::with_capacity(8);
    for t in &m.idx {
        let tri = [cvs[t[0] as usize], cvs[t[1] as usize], cvs[t[2] as usize]];
        let (mut fuera_comun, mut fuera_alguno) = (0xFFu32, 0u32);
        for c in &tri {
            let mut bits = 0u32;
            for k in 0..5 { if dist_plano(k, c) < 0.0 { bits |= 1 << k; } }
            fuera_comun &= bits;
            fuera_alguno |= bits;
        }
        if fuera_comun != 0 { continue; } // totalmente fuera de un plano: se descarta
        poly.clear();
        poly.extend_from_slice(&tri);
        for k in 0..5 {
            if fuera_alguno & (1 << k) != 0 {
                recortar(&mut poly, &mut tmp, k);
                if poly.len() < 3 { break; }
            }
        }
        if poly.len() < 3 { continue; }

        // Abanico de triángulos del polígono recortado.
        for i in 1..poly.len() - 1 {
            let mut sv = [SV { x: 0.0, y: 0.0, iw: 0.0, a: [0.0; 8] }; 3];
            for (j, c) in [poly[0], poly[i], poly[i + 1]].iter().enumerate() {
                let iw = 1.0 / c.w;
                let mut a = c.a;
                for k in 0..8 { a[k] *= iw; }
                sv[j] = SV { x: (c.x * iw * 0.5 + 0.5) * vista.w, y: (0.5 - c.y * iw * 0.5) * vista.h, iw, a };
            }
            let area = (sv[1].x - sv[0].x) * (sv[2].y - sv[0].y) - (sv[1].y - sv[0].y) * (sv[2].x - sv[0].x);
            if area.abs() < 1e-7 { continue; }
            // En pantalla (y hacia abajo) las caras frontales antihorarias dan área negativa.
            if !item.doble_cara && area >= 0.0 { continue; }
            out.push(Tri {
                v: sv,
                item: id,
                min_y: sv[0].y.min(sv[1].y).min(sv[2].y),
                max_y: sv[0].y.max(sv[1].y).max(sv[2].y),
                inv_area: 1.0 / area,
            });
        }
    }
}

// Pinta todos los triángulos que caen en una banda de filas.
pub fn rasterizar_banda(y0: usize, color: &mut [u8], zbuf: &mut [f32], w: usize,
                        tris: &[Tri], items: &[Item], ctx: &Contexto) {
    let filas = zbuf.len() / w;
    let (ytop, ybot) = (y0 as f32, (y0 + filas) as f32);
    for t in tris {
        if t.max_y < ytop || t.min_y > ybot { continue; }
        let item = &items[t.item as usize];
        let (a, b, c) = (&t.v[0], &t.v[1], &t.v[2]);
        let x_min = a.x.min(b.x).min(c.x).floor().max(0.0) as usize;
        let x_max = (a.x.max(b.x).max(c.x).ceil().min(w as f32 - 1.0)).max(0.0) as usize;
        let ya = (t.min_y.floor().max(ytop)) as usize;
        let yb = ((t.max_y.ceil()).min(ybot - 1.0)).max(0.0) as usize;
        if ya > yb || x_min > x_max { continue; }

        // Funciones de borde: E0 pesa a `a`, E1 a `b`, E2 a `c`.
        // Normalizadas con 1/área son las coordenadas baricéntricas.
        let k = t.inv_area;
        let (d0x, d0y) = (-(c.y - b.y) * k, (c.x - b.x) * k);
        let (d1x, d1y) = (-(a.y - c.y) * k, (a.x - c.x) * k);
        let (d2x, d2y) = (-(b.y - a.y) * k, (b.x - a.x) * k);
        let px0 = x_min as f32 + 0.5;
        for y in ya..=yb {
            let py = y as f32 + 0.5;
            let mut w0 = (d0y * (py - b.y) + d0x * (px0 - b.x)) ;
            let mut w1 = (d1y * (py - c.y) + d1x * (px0 - c.x)) ;
            let mut w2 = (d2y * (py - a.y) + d2x * (px0 - a.x)) ;
            let fila = (y - y0) * w;
            for x in x_min..=x_max {
                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let iw = w0 * a.iw + w1 * b.iw + w2 * c.iw;
                    let idx = fila + x;
                    if iw > zbuf[idx] {
                        let inv = 1.0 / iw;
                        let mut at = [0.0f32; 8];
                        for q in 0..8 { at[q] = (w0 * a.a[q] + w1 * b.a[q] + w2 * c.a[q]) * inv; }
                        let frag = Fragmento {
                            pos: v3(at[0], at[1], at[2]),
                            normal: v3(at[3], at[4], at[5]),
                            uv: [at[6], at[7]],
                            tinte: item.tinte,
                            centro: item.pos,
                            mip: item.mip,
                        };
                        let s = item.shader.shade(&frag, ctx);
                        let o = idx * 4;
                        match item.mezcla {
                            Mezcla::Opaco => {
                                zbuf[idx] = iw;
                                color[o] = (clamp01(s[0]) * 255.0) as u8;
                                color[o + 1] = (clamp01(s[1]) * 255.0) as u8;
                                color[o + 2] = (clamp01(s[2]) * 255.0) as u8;
                            }
                            Mezcla::Alfa => {
                                let al = clamp01(s[3]);
                                if al > 0.003 {
                                    for q in 0..3 {
                                        let d = color[o + q] as f32;
                                        color[o + q] = (d + (clamp01(s[q]) * 255.0 - d) * al) as u8;
                                    }
                                }
                            }
                            Mezcla::Aditivo => {
                                let al = clamp01(s[3]);
                                for q in 0..3 {
                                    color[o + q] = (color[o + q] as f32 + clamp01(s[q]) * 255.0 * al).min(255.0) as u8;
                                }
                            }
                        }
                    }
                }
                w0 += d0x;
                w1 += d1x;
                w2 += d2x;
            }
        }
    }
}
