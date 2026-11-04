// Orquesta un cuadro: fondo -> estrellas -> objetos opacos -> transparentes.
use crate::camara::Camara;
use crate::cielo::Cielo;
use crate::dibujo::*;
use crate::escena::Escena;
use crate::math::*;
use crate::raster::*;
use crate::shader::Contexto;
use crate::textura::nucleos;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub fn dibujar(fb: &mut Framebuffer, escena: &Escena, cielo: &Cielo, cam: &Camara, tiempo: f32, aj: &Ajustes) -> Stats {
    let (w, h) = (fb.width, fb.height);
    let vista = Vista::de(cam, w, h);
    let ctx = Contexto { cam: cam.pos, tiempo, sol: escena.pos_sol() };
    let hilos = nucleos();

    // 1. Qué dibujar (culling + LOD). Opacos primero; transparentes de lejos a cerca.
    let mut stats = Stats::default();
    let mut items: Vec<Item> = Vec::new();
    escena.emitir(cam, aj, &mut items, &mut stats);
    let (mut opacos, mut transp): (Vec<Item>, Vec<Item>) = items.into_iter().partition(|i| i.mezcla == Mezcla::Opaco);
    transp.sort_by(|a, b| b.dist.total_cmp(&a.dist));
    opacos.extend(transp);
    let items = opacos;

    // 2. Fondo: nebulosa por píxel + z-buffer a cero.
    fb.zbuf.fill(0.0);
    {
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
                        if u < 0.0 { u += 1.0; }
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
    }

    // 3. Estrellas (puntos 3D proyectados; las grandes con destello en cruz).
    dibujar_estrellas(fb, cielo, &vista, tiempo);

    // 4. Triángulos: etapa de vértices en paralelo (cola de trabajo por objeto).
    let siguiente = AtomicUsize::new(0);
    let resultados: Mutex<Vec<(usize, Vec<Tri>)>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..hilos.min(items.len().max(1)) {
            s.spawn(|| {
                let mut locales = Vec::new();
                loop {
                    let i = siguiente.fetch_add(1, Ordering::Relaxed);
                    if i >= items.len() { break; }
                    let mut v = Vec::new();
                    preparar_item(&items[i], i as u32, &vista, &mut v);
                    locales.push((i, v));
                }
                resultados.lock().unwrap().extend(locales);
            });
        }
    });
    let mut res = resultados.into_inner().unwrap();
    res.sort_by_key(|r| r.0); // se conserva el orden (importa para las transparencias)
    let tris: Vec<Tri> = res.into_iter().flat_map(|r| r.1).collect();
    stats.tris_raster = tris.len();

    // 5. Rasterización por bandas de filas repartidas entre los hilos.
    const BANDA: usize = 20;
    let trabajos = Mutex::new(fb.pixels.chunks_mut(w * 4 * BANDA).zip(fb.zbuf.chunks_mut(w * BANDA)).enumerate());
    std::thread::scope(|s| {
        for _ in 0..hilos {
            s.spawn(|| loop {
                let sig = trabajos.lock().unwrap().next();
                let Some((b, (color, z))) = sig else { break };
                rasterizar_banda(b * BANDA, color, z, w, &tris, &items, &ctx);
            });
        }
    });
    stats
}

fn dibujar_estrellas(fb: &mut Framebuffer, cielo: &Cielo, v: &Vista, t: f32) {
    let (w, h) = (fb.width as i32, fb.height as i32);
    let mut poner = |x: i32, y: i32, c: V3, k: f32| {
        if x < 0 || y < 0 || x >= w || y >= h { return; }
        let o = (y as usize * fb.width + x as usize) * 4;
        for q in 0..3 {
            let add = [c.x, c.y, c.z][q] * k * 255.0;
            fb.pixels[o + q] = (fb.pixels[o + q] as f32 + add).min(255.0) as u8;
        }
    };
    for e in &cielo.estrellas {
        let wv = e.dir.dot(v.f);
        if wv < 0.05 { continue; }
        let sx = (e.dir.dot(v.r) * v.fx / wv * 0.5 + 0.5) * v.w;
        let sy = (0.5 - e.dir.dot(v.u) * v.fy / wv * 0.5) * v.h;
        let (x, y) = (sx as i32, sy as i32);
        let b = e.brillo * (0.78 + 0.22 * (t * e.vel + e.fase).sin());
        match e.tipo {
            0 => poner(x, y, e.color, b),
            1 => {
                poner(x, y, e.color, b);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] { poner(x + dx, y + dy, e.color, b * 0.35); }
            }
            _ => {
                let largo = if e.tipo == 3 { 16 } else { 8 };
                poner(x, y, e.color, 1.0);
                for i in 1..=largo {
                    let f = (1.0 - i as f32 / largo as f32).powi(2) * b;
                    for (dx, dy) in [(i, 0), (-i, 0), (0, i), (0, -i)] { poner(x + dx, y + dy, e.color, f); }
                }
                if e.tipo == 3 {
                    for (dx, dy) in [(1, 1), (-1, 1), (1, -1), (-1, -1), (2, 0), (-2, 0), (0, 2), (0, -2)] {
                        poner(x + dx, y + dy, e.color, 0.6 * b);
                    }
                }
            }
        }
    }
}
