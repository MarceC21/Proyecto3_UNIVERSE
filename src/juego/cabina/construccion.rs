// Constructor de geometría para interiores: quads con UV en metros, cajas giradas,
// paredes cilíndricas con aberturas y un "Marco" local para colocar piezas sobre una pared.
// Convención de caras: antihorario visto desde afuera (la misma de malla.rs).
use crate::dibujo_interior::Parte;
use crate::malla::Malla;
use crate::materiales::{Luz, Mat, ShaderInterior};
use crate::math::*;
use std::f32::consts::TAU;
use std::sync::Arc;

#[derive(Default)]
pub struct Geo {
    pub pos: Vec<V3>,
    pub nrm: Vec<V3>,
    pub uv: Vec<[f32; 2]>,
    pub idx: Vec<[u32; 3]>,
}

impl Geo {
    pub fn vacio() -> Geo {
        Geo::default()
    }
    pub fn es_vacio(&self) -> bool {
        self.idx.is_empty()
    }

    // `n` es la normal deseada: el sentido de giro se corrige solo.
    pub fn quad(&mut self, p: [V3; 4], uv: [[f32; 2]; 4], n: V3) {
        let b = self.pos.len() as u32;
        for k in 0..4 {
            self.pos.push(p[k]);
            self.nrm.push(n);
            self.uv.push(uv[k]);
        }
        if (p[1] - p[0]).cross(p[2] - p[0]).dot(n) >= 0.0 {
            self.idx.push([b, b + 1, b + 2]);
            self.idx.push([b, b + 2, b + 3]);
        } else {
            self.idx.push([b, b + 2, b + 1]);
            self.idx.push([b, b + 3, b + 2]);
        }
    }

    pub fn caja(&mut self, c: V3, s: V3) {
        self.caja_rot(c, s, 0.0);
    }

    pub fn caja_rot(&mut self, c: V3, s: V3, yaw: f32) {
        let r = Rot::rot_y(yaw);
        let hs = [s.x * 0.5, s.y * 0.5, s.z * 0.5];
        let cc = [c.x, c.y, c.z];
        for a in 0..3 {
            for sg in [-1.0f32, 1.0] {
                let (b, d) = ((a + 1) % 3, (a + 2) % 3);
                let mut pts = [V3::ZERO; 4];
                let mut uvs = [[0.0f32; 2]; 4];
                for (k, (sb, sd)) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                    .iter()
                    .enumerate()
                {
                    let mut l = [0.0f32; 3];
                    l[a] = sg * hs[a];
                    l[b] = sb * hs[b];
                    l[d] = sd * hs[d];
                    pts[k] = c + r.aplicar(v3(l[0], l[1], l[2]));
                    uvs[k] = match a {
                        0 => [l[2] + cc[2], l[1] + cc[1]],
                        1 => [l[0] + cc[0], l[2] + cc[2]],
                        _ => [l[0] + cc[0], l[1] + cc[1]],
                    };
                }
                let mut nl = [0.0f32; 3];
                nl[a] = sg;
                self.quad(pts, uvs, r.aplicar(v3(nl[0], nl[1], nl[2])));
            }
        }
    }

    // Pared cilíndrica vertical (eje Y por `c`). `tramos(k, ángulo)` da los intervalos
    // verticales a construir en cada segmento: así salen ventanas y puertas.
    pub fn cilindro(
        &mut self,
        c: V3,
        r: f32,
        seg: usize,
        hacia_dentro: bool,
        tramos: &dyn Fn(usize, f32) -> Vec<(f32, f32)>,
    ) {
        for k in 0..seg {
            let a0 = k as f32 / seg as f32 * TAU;
            let a1 = (k + 1) as f32 / seg as f32 * TAU;
            let am = (a0 + a1) * 0.5;
            let n = v3(am.cos(), 0.0, am.sin()) * if hacia_dentro { -1.0 } else { 1.0 };
            for (y0, y1) in tramos(k, am) {
                let q = |a: f32, y: f32| v3(c.x + r * a.cos(), y, c.z + r * a.sin());
                self.quad(
                    [q(a0, y0), q(a1, y0), q(a1, y1), q(a0, y1)],
                    [[a0 * r, y0], [a1 * r, y0], [a1 * r, y1], [a0 * r, y1]],
                    n,
                );
            }
        }
    }

    // Disco/anillo horizontal (r0 = 0 da un disco). UV = coordenadas del mundo.
    pub fn anillo_h(&mut self, c: V3, r0: f32, r1: f32, y: f32, seg: usize, arriba: bool) {
        let n = v3(0.0, if arriba { 1.0 } else { -1.0 }, 0.0);
        for k in 0..seg {
            let a0 = k as f32 / seg as f32 * TAU;
            let a1 = (k + 1) as f32 / seg as f32 * TAU;
            let q = |r: f32, a: f32| v3(c.x + r * a.cos(), y, c.z + r * a.sin());
            let p = [q(r0, a0), q(r1, a0), q(r1, a1), q(r0, a1)];
            self.quad(
                p,
                [
                    [p[0].x, p[0].z],
                    [p[1].x, p[1].z],
                    [p[2].x, p[2].z],
                    [p[3].x, p[3].z],
                ],
                n,
            );
        }
    }

    pub fn rect_h(&mut self, x0: f32, x1: f32, z0: f32, z1: f32, y: f32, arriba: bool) {
        let p = [v3(x0, y, z0), v3(x1, y, z0), v3(x1, y, z1), v3(x0, y, z1)];
        self.quad(
            p,
            [[x0, z0], [x1, z0], [x1, z1], [x0, z1]],
            v3(0.0, if arriba { 1.0 } else { -1.0 }, 0.0),
        );
    }

    // Polígono convexo horizontal en abanico desde su centroide.
    pub fn poligono_h(&mut self, pts: &[(f32, f32)], y: f32, arriba: bool) {
        let n = pts.len() as f32;
        let (cx, cz) = (
            pts.iter().map(|p| p.0).sum::<f32>() / n,
            pts.iter().map(|p| p.1).sum::<f32>() / n,
        );
        let nn = v3(0.0, if arriba { 1.0 } else { -1.0 }, 0.0);
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            let p = [
                v3(cx, y, cz),
                v3(a.0, y, a.1),
                v3(b.0, y, b.1),
                v3(b.0, y, b.1),
            ];
            self.quad(p, [[cx, cz], [a.0, a.1], [b.0, b.1], [b.0, b.1]], nn);
        }
    }

    // Pared recta entre dos puntos del plano, con la cara hacia `hacia`.
    pub fn pared(&mut self, a: (f32, f32), b: (f32, f32), y0: f32, y1: f32, hacia: (f32, f32)) {
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dz * dz).sqrt().max(1e-6);
        let mut n = v3(-dz / l, 0.0, dx / l);
        let m = ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
        if n.x * (hacia.0 - m.0) + n.z * (hacia.1 - m.1) < 0.0 {
            n = -n;
        }
        self.quad(
            [
                v3(a.0, y0, a.1),
                v3(b.0, y0, b.1),
                v3(b.0, y1, b.1),
                v3(a.0, y1, a.1),
            ],
            [[0.0, y0], [l, y0], [l, y1], [0.0, y1]],
            n,
        );
    }

    pub fn a_malla(self) -> Malla {
        let radio = self.pos.iter().map(|p| p.len()).fold(0.0, f32::max);
        Malla {
            pos: self.pos,
            nrm: self.nrm,
            uv: self.uv,
            idx: self.idx,
            radio,
        }
    }
}

// Marco local de una pared: u recorre la pared, `dep` mide hacia el interior.
#[derive(Clone, Copy)]
pub struct Marco {
    pub a: (f32, f32),
    pub d: (f32, f32),
    pub n: (f32, f32),
    pub len: f32,
    pub yaw: f32,
}

impl Marco {
    pub fn de(a: (f32, f32), b: (f32, f32)) -> Marco {
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dz * dz).sqrt();
        let d = (dx / len, dz / len);
        Marco {
            a,
            d,
            n: (-d.1, d.0),
            len,
            yaw: (-d.1).atan2(d.0),
        }
    }
    pub fn p(&self, u: f32, y: f32, dep: f32) -> V3 {
        v3(
            self.a.0 + self.d.0 * u + self.n.0 * dep,
            y,
            self.a.1 + self.d.1 * u + self.n.1 * dep,
        )
    }
    pub fn normal(&self) -> V3 {
        v3(self.n.0, 0.0, self.n.1)
    }
    pub fn caja(&self, g: &mut Geo, u: f32, y: f32, dep: f32, w: f32, h: f32, dp: f32) {
        g.caja_rot(self.p(u, y, dep), v3(w, h, dp), self.yaw);
    }
    // Pantalla vertical mirando hacia el interior, UV 0..1 (v = 0 arriba).
    pub fn pantalla(&self, g: &mut Geo, u: f32, y: f32, dep: f32, w: f32, h: f32) {
        g.quad(
            [
                self.p(u - w / 2.0, y - h / 2.0, dep),
                self.p(u + w / 2.0, y - h / 2.0, dep),
                self.p(u + w / 2.0, y + h / 2.0, dep),
                self.p(u - w / 2.0, y + h / 2.0, dep),
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            self.normal(),
        );
    }
    // Panel horizontal mirando hacia arriba, UV 0..1 (v = 0 lejos del observador).
    pub fn plano(&self, g: &mut Geo, u: f32, y: f32, dep: f32, w: f32, dp: f32) {
        g.quad(
            [
                self.p(u - w / 2.0, y, dep - dp / 2.0),
                self.p(u + w / 2.0, y, dep - dp / 2.0),
                self.p(u + w / 2.0, y, dep + dp / 2.0),
                self.p(u - w / 2.0, y, dep + dp / 2.0),
            ],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            v3(0.0, 1.0, 0.0),
        );
    }
    // Pared completa con aberturas (u0, u1, y0, y1) ordenadas por u.
    pub fn pared(&self, g: &mut Geo, h: f32, ab: &[(f32, f32, f32, f32)]) {
        let panel = |g: &mut Geo, u0: f32, u1: f32, y0: f32, y1: f32| {
            g.quad(
                [
                    self.p(u0, y0, 0.0),
                    self.p(u1, y0, 0.0),
                    self.p(u1, y1, 0.0),
                    self.p(u0, y1, 0.0),
                ],
                [[u0, y0], [u1, y0], [u1, y1], [u0, y1]],
                self.normal(),
            );
        };
        let mut u = 0.0;
        for &(u0, u1, y0, y1) in ab {
            if u0 > u {
                panel(g, u, u0, 0.0, h);
            }
            if y0 > 0.0 {
                panel(g, u0, u1, 0.0, y0);
            }
            if y1 < h {
                panel(g, u0, u1, y1, h);
            }
            u = u1;
        }
        if u < self.len {
            panel(g, u, self.len, 0.0, h);
        }
    }
}

// Acumula la geometría por material (y por color para las piezas pintadas).
pub struct Constructor {
    pub geos: Vec<Geo>,
    pub pint: Vec<(V3, Geo)>,
    pub luces: Vec<Luz>,
}

impl Constructor {
    pub fn nuevo() -> Constructor {
        Constructor {
            geos: (0..Mat::N).map(|_| Geo::vacio()).collect(),
            pint: vec![],
            luces: vec![],
        }
    }
    pub fn g(&mut self, m: Mat) -> &mut Geo {
        &mut self.geos[m as usize]
    }
    pub fn pintura(&mut self, c: V3) -> &mut Geo {
        let i = match self.pint.iter().position(|(k, _)| (*k - c).len() < 1e-3) {
            Some(i) => i,
            None => {
                self.pint.push((c, Geo::vacio()));
                self.pint.len() - 1
            }
        };
        &mut self.pint[i].1
    }
    pub fn luz(&mut self, pos: V3, color: V3, inten: f32, radio: f32) {
        self.luces.push(Luz {
            pos,
            color,
            inten,
            radio,
        });
    }
    pub fn terminar(self, luces: Arc<Vec<Luz>>, ambiente: V3) -> Vec<Parte> {
        let mut out = vec![];
        for (i, g) in self.geos.into_iter().enumerate() {
            if g.es_vacio() {
                continue;
            }
            out.push(Parte {
                malla: g.a_malla(),
                shader: ShaderInterior::nuevo(Mat::TODOS[i], luces.clone(), ambiente),
                tinte: v3(1.0, 1.0, 1.0),
                doble: false,
            });
        }
        for (c, g) in self.pint {
            out.push(Parte {
                malla: g.a_malla(),
                shader: ShaderInterior::nuevo(Mat::Pintura, luces.clone(), ambiente),
                tinte: c,
                doble: false,
            });
        }
        out
    }
}
