// Dibujo de escenas interiores sobre un framebuffer que ya contiene el exterior
// (ventanas) o solo el cielo. Reutiliza la etapa de vértices y el rasterizador del motor.
use crate::camara::Camara;
use crate::cielo::Cielo;
use crate::dibujo::{Item, Mezcla};
use crate::malla::Malla;
use crate::materiales::ShaderInterior;
use crate::math::*;
use crate::raster::*;
use crate::shader::Contexto;
use crate::textura::nucleos;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub struct Escenario {
    pub partes: Vec<Parte>,
    pub mundo: crate::jugador::Mundo,
    pub luces: std::sync::Arc<Vec<crate::materiales::Luz>>,
    pub ambiente: V3,
}

pub struct Parte {
    pub malla: Malla,
    pub shader: ShaderInterior,
    pub tinte: V3,
    pub doble: bool,
}

impl Parte {
    pub fn item(&self, pos: V3, rot: Rot, escala: V3) -> Item<'_> {
        Item {
            malla: &self.malla,
            shader: &self.shader,
            pos,
            rot,
            escala,
            mezcla: Mezcla::Opaco,
            doble_cara: self.doble,
            tinte: self.tinte,
            mip: 0.0,
            dist: 0.0,
        }
    }
    pub fn item_con(&self, pos: V3, rot: Rot, escala: V3, tinte: V3) -> Item<'_> {
        let mut it = self.item(pos, rot, escala);
        it.tinte = tinte;
        it
    }
    pub fn estatico(&self) -> Item<'_> {
        self.item(V3::ZERO, Rot::identidad(), v3(1.0, 1.0, 1.0))
    }
}

// Borra la profundidad y dibuja los items encima de lo que ya hay en el framebuffer.
pub fn dibujar_items(fb: &mut Framebuffer, items: &[Item], cam: &Camara, tiempo: f32) -> usize {
    let (w, _h) = (fb.width, fb.height);
    let vista = Vista::de(cam, fb.width, fb.height);
    let ctx = Contexto {
        cam: cam.pos,
        tiempo,
        sol: v3(0.0, 6.0, 0.0),
    };
    let hilos = nucleos();
    fb.zbuf.fill(0.0);

    let siguiente = AtomicUsize::new(0);
    let resultados: Mutex<Vec<(usize, Vec<Tri>)>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..hilos.min(items.len().max(1)) {
            s.spawn(|| {
                let mut locales = Vec::new();
                loop {
                    let i = siguiente.fetch_add(1, Ordering::Relaxed);
                    if i >= items.len() {
                        break;
                    }
                    let mut v = Vec::new();
                    preparar_item(&items[i], i as u32, &vista, &mut v);
                    locales.push((i, v));
                }
                resultados.lock().unwrap().extend(locales);
            });
        }
    });
    let mut res = resultados.into_inner().unwrap();
    res.sort_by_key(|r| r.0);
    let tris: Vec<Tri> = res.into_iter().flat_map(|r| r.1).collect();
    let total = tris.len();

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
                rasterizar_banda(b * BANDA, color, z, w, &tris, items, &ctx);
            });
        }
    });
    total
}

// Solo nebulosa + estrellas (para las ventanas de la estación, sin geometría exterior).
pub fn cielo_fondo(fb: &mut Framebuffer, cielo: &Cielo, cam: &Camara, tiempo: f32) {
    let (w, h) = (fb.width, fb.height);
    let vista = Vista::de(cam, w, h);
    let hilos = nucleos();
    let filas = h.div_ceil(hilos).max(1);
    let (tan_x, tan_y) = (1.0 / vista.fx, 1.0 / vista.fy);
    std::thread::scope(|s| {
        for (ci, trozo) in fb.pixels.chunks_mut(filas * w * 4).enumerate() {
            let (vista, cielo) = (&vista, cielo);
            s.spawn(move || {
                for (i, px) in trozo.chunks_exact_mut(4).enumerate() {
                    let (x, y) = (i % w, ci * filas + i / w);
                    let nx = 2.0 * (x as f32 + 0.5) / w as f32 - 1.0;
                    let ny = 1.0 - 2.0 * (y as f32 + 0.5) / h as f32;
                    let d = (vista.f + vista.r * (nx * tan_x) + vista.u * (ny * tan_y)).norm();
                    let mut u = d.z.atan2(d.x) / std::f32::consts::TAU;
                    if u < 0.0 {
                        u += 1.0;
                    }
                    let v = 0.5 - d.y.clamp(-1.0, 1.0).asin() / std::f32::consts::PI;
                    let c = cielo.nebulosa.muestrear(u, v, 0.0);
                    px[0] = (c.x * 255.0) as u8;
                    px[1] = (c.y * 255.0) as u8;
                    px[2] = (c.z * 255.0) as u8;
                    px[3] = 255;
                }
            });
        }
    });
    let (wi, hi) = (w as i32, h as i32);
    for e in &cielo.estrellas {
        let wv = e.dir.dot(vista.f);
        if wv < 0.05 {
            continue;
        }
        let sx = (e.dir.dot(vista.r) * vista.fx / wv * 0.5 + 0.5) * vista.w;
        let sy = (0.5 - e.dir.dot(vista.u) * vista.fy / wv * 0.5) * vista.h;
        let b = e.brillo * (0.78 + 0.22 * (tiempo * e.vel + e.fase).sin());
        let tam = if e.tipo >= 2 {
            2
        } else if e.tipo == 1 {
            1
        } else {
            0
        };
        for dy in -tam..=tam {
            for dx in -tam..=tam {
                if dx != 0 && dy != 0 {
                    continue;
                }
                let (x, y) = (sx as i32 + dx, sy as i32 + dy);
                if x < 0 || y < 0 || x >= wi || y >= hi {
                    continue;
                }
                let o = (y as usize * w + x as usize) * 4;
                let k = if dx == 0 && dy == 0 { 1.0 } else { 0.4 };
                for q in 0..3 {
                    let add = [e.color.x, e.color.y, e.color.z][q] * b * k * 255.0;
                    fb.pixels[o + q] = (fb.pixels[o + q] as f32 + add).min(255.0) as u8;
                }
            }
        }
    }
}
