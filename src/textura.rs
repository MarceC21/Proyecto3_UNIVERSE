// Textura: imagen RGB con mipmaps y muestreo bilineal por UV.
// Las texturas de planetas se "hornean" una sola vez al arrancar, evaluando
// ruido 3D sobre la dirección de cada texel (así no hay costura ni deformación
// en los polos). Después el shader solo lee la textura: muy barato.
use crate::math::*;

struct Nivel {
    w: usize,
    h: usize,
    px: Vec<[u8; 3]>,
}

pub struct Textura {
    niveles: Vec<Nivel>,
}

pub fn nucleos() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

impl Textura {
    fn desde_nivel0(w: usize, h: usize, px: Vec<[u8; 3]>) -> Textura {
        let mut niveles = vec![Nivel { w, h, px }];
        // Cadena de mipmaps: promedio de bloques 2x2 hasta ~8 px de ancho.
        while niveles.last().unwrap().w > 8 {
            let a = niveles.last().unwrap();
            let (nw, nh) = (a.w / 2, (a.h / 2).max(1));
            let mut px = vec![[0u8; 3]; nw * nh];
            for y in 0..nh {
                for x in 0..nw {
                    let mut s = [0u32; 3];
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let p = a.px[((y * 2 + dy).min(a.h - 1)) * a.w + x * 2 + dx];
                        for k in 0..3 { s[k] += p[k] as u32; }
                    }
                    px[y * nw + x] = [(s[0] / 4) as u8, (s[1] / 4) as u8, (s[2] / 4) as u8];
                }
            }
            niveles.push(Nivel { w: nw, h: nh, px });
        }
        Textura { niveles }
    }

    // Hornea una textura equirectangular: `f` recibe la dirección 3D unitaria
    // que corresponde a cada texel (u = longitud, v = latitud) y devuelve el color.
    pub fn generar(w: usize, h: usize, f: &(dyn Fn(V3) -> V3 + Sync)) -> Textura {
        let mut px = vec![[0u8; 3]; w * h];
        let filas = h.div_ceil(nucleos()).max(1);
        std::thread::scope(|s| {
            for (ci, trozo) in px.chunks_mut(filas * w).enumerate() {
                s.spawn(move || {
                    for (i, p) in trozo.iter_mut().enumerate() {
                        let (x, y) = (i % w, ci * filas + i / w);
                        let lon = (x as f32 + 0.5) / w as f32 * std::f32::consts::TAU;
                        let lat = (0.5 - (y as f32 + 0.5) / h as f32) * std::f32::consts::PI;
                        let d = v3(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin());
                        let c = f(d);
                        *p = [(clamp01(c.x) * 255.0) as u8, (clamp01(c.y) * 255.0) as u8,
                              (clamp01(c.z) * 255.0) as u8];
                    }
                });
            }
        });
        Textura::desde_nivel0(w, h, px)
    }

    // Muestreo bilineal. u envuelve (da la vuelta), v se recorta. `mip` elige
    // el nivel de detalle: objetos lejanos usan niveles pequeños (menos parpadeo).
    pub fn muestrear(&self, u: f32, v: f32, mip: f32) -> V3 {
        let l = (mip.max(0.0).round() as usize).min(self.niveles.len() - 1);
        let n = &self.niveles[l];
        let fx = u * n.w as f32 - 0.5;
        let fy = v * n.h as f32 - 0.5;
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let (x0, y0) = (x0 as i32, y0 as i32);
        let g = |x: i32, y: i32| {
            let xi = x.rem_euclid(n.w as i32) as usize;
            let yi = y.clamp(0, n.h as i32 - 1) as usize;
            let p = n.px[yi * n.w + xi];
            v3(p[0] as f32, p[1] as f32, p[2] as f32)
        };
        let arriba = g(x0, y0).lerp(g(x0 + 1, y0), tx);
        let abajo = g(x0, y0 + 1).lerp(g(x0 + 1, y0 + 1), tx);
        arriba.lerp(abajo, ty) * (1.0 / 255.0)
    }

    pub fn ancho(&self) -> usize { self.niveles[0].w }
}
