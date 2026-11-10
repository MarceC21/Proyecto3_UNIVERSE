// Línea Petrova, taumoebas y asteroides: todo se dibuja con el rasterizador CPU.
use crate::{
    camara::Camara,
    dibujo::{Item, Mezcla, Stats},
    generadores,
    malla::{self, Malla},
    math::*,
    mision_adrian::{Fase, MisionAdrian, Tipo, SEMIALTO, SEMIANCHO},
    nave::{Nave, PoseNave},
    raster::{preparar_item, rasterizar_banda, Framebuffer, Vista},
    ruido::{fbm, ruido, Rng},
    shader::{Contexto, Fragmento, Shader, ShaderAsteroide},
    textura::nucleos,
};
use std::sync::Mutex;

struct Fondo {
    intensidad: f32,
}

impl Shader for Fondo {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let x = f.uv[0] * 2.0 - 1.0;
        let y = f.uv[1] * 2.0 - 1.0;
        let nube = fbm(v3(x * 2.2 + c.tiempo * 0.04, y * 2.7, 2.0), 3, 1701) * 0.5 + 0.5;
        let franja =
            (-(y - x * 0.32 + (x * 4.0 + c.tiempo * 0.1).sin() * 0.08).powi(2) * 8.0).exp();
        let filamento =
            (1.0 - ruido(v3(x * 9.0, y * 12.0, c.tiempo * 0.08), 1702).abs() * 3.0).max(0.0);
        let col = rgb(6.0, 8.0, 20.0)
            + rgb(140.0, 11.0, 24.0) * (franja * nube * self.intensidad)
            + rgb(176.0, 43.0, 48.0) * (filamento.powi(6) * nube * franja * self.intensidad * 0.2);
        [col.x, col.y, col.z, 1.0]
    }
}
struct Particula;
impl Shader for Particula {
    fn shade(&self, f: &Fragmento, _: &Contexto) -> [f32; 4] {
        let x = (f.uv[0] - 0.5) * 2.0;
        let y = (f.uv[1] - 0.5) * 2.0;
        let r = x * x + y * y;
        let a = (1.0 - r).max(0.0).powi(3);
        [f.tinte.x, f.tinte.y, f.tinte.z, a * 0.8]
    }
}
struct Nucleo;
impl Shader for Nucleo {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let p = (f.pos - f.centro) * 4.0;
        let n = ruido(p + v3(c.tiempo * 0.35, 0.0, 0.0), 1711).abs();
        let veta = 1.0 - smoothstep(0.025, 0.12, n);
        let vista = f.normal.norm().dot((c.cam - f.pos).norm()).max(0.0);
        let col = mezcla(
            rgb(8.0, 165.0, 37.0),
            rgb(47.0, 248.0, 93.0),
            vista * 0.7 + veta * 0.3,
        );
        [col.x, col.y, col.z, 1.0]
    }
}
struct Membrana;
impl Shader for Membrana {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let p = (f.pos - f.centro) * 3.8;
        let n = ruido(p + v3(0.0, c.tiempo * 0.3, 0.0), 1712).abs();
        let hilo = 1.0 - smoothstep(0.025, 0.115, n);
        let borde = (1.0 - f.normal.norm().dot((c.cam - f.pos).norm()).abs()).powi(2);
        [
            0.05,
            0.95,
            0.88,
            (0.045 + hilo * 0.32 + borde * 0.38).min(0.7),
        ]
    }
}
struct Halo;
impl Shader for Halo {
    fn shade(&self, f: &Fragmento, _: &Contexto) -> [f32; 4] {
        let x = (f.uv[0] - 0.5) * 2.0;
        let y = (f.uv[1] - 0.5) * 2.0;
        let r = (x * x + y * y).sqrt();
        let a = (1.0 - r).max(0.0).powi(3) * 0.25;
        [0.05, 0.9, 0.67, a]
    }
}
struct Anillo;
impl Shader for Anillo {
    fn shade(&self, _: &Fragmento, c: &Contexto) -> [f32; 4] {
        [0.3, 1.0, 0.72, 0.75 + 0.2 * (c.tiempo * 3.0).sin()]
    }
}

fn agregar<'a>(
    items: &mut Vec<Item<'a>>,
    cam: &Camara,
    malla: &'a Malla,
    shader: &'a dyn Shader,
    pos: V3,
    rot: Rot,
    escala: V3,
    mezcla: Mezcla,
    tinte: V3,
) {
    items.push(Item {
        malla,
        shader,
        pos,
        rot,
        escala,
        mezcla,
        tinte,
        doble_cara: true,
        mip: 0.0,
        dist: (pos - cam.pos).len(),
    });
}

pub struct EscenaAdrian {
    esfera: Malla,
    membrana: Malla,
    cuadrado: Malla,
    anillo: Malla,
    marco: Malla,
    rocas: Vec<Malla>,
    roca: ShaderAsteroide,
    nucleo: Nucleo,
    piel: Membrana,
    halo: Halo,
    particula: Particula,
    s_anillo: Anillo,
    particulas: Vec<(V3, f32)>,
}
impl EscenaAdrian {
    pub fn nueva() -> Self {
        let mut membrana = malla::esfera(32, 20);
        for p in &mut membrana.pos {
            *p = *p * (1.0 + ruido(*p * 5.0, 1721) * 0.13);
        }
        membrana.radio = 1.2;
        let mut rocas = vec![];
        // Mismos generadores y semillas que el cinturón del sistema solar.
        for t in 0..4 {
            for (lon, lat) in [(14, 9), (8, 6), (5, 4)] {
                rocas.push(malla::asteroide(100 + t * 17, lon, lat));
            }
        }
        let mut geo = crate::construccion::Geo::vacio();
        // Referencia visual lejana de la zona frontal, solo cuatro pequeñas esquinas.
        for x in [-SEMIANCHO, SEMIANCHO] {
            for y in [-SEMIALTO, SEMIALTO] {
                geo.caja(v3(x, y, 0.0), v3(0.7, 0.04, 0.04));
                geo.caja(v3(x, y, 0.0), v3(0.04, 0.7, 0.04));
            }
        }
        let mut rng = Rng::new(1731);
        let particulas = (0..800)
            .map(|_| {
                (
                    v3(
                        rng.rango(-25.0, 25.0),
                        rng.rango(-15.0, 15.0),
                        rng.rango(0.0, 75.0),
                    ),
                    rng.rango(0.035, 0.16),
                )
            })
            .collect();
        Self {
            esfera: malla::esfera(24, 16),
            membrana,
            cuadrado: malla::cuadrado(),
            anillo: malla::anillo(0.94, 1.06, 72, 1.0),
            marco: geo.a_malla(),
            rocas,
            roca: ShaderAsteroide {
                tex: generadores::asteroide(),
            },
            nucleo: Nucleo,
            piel: Membrana,
            halo: Halo,
            particula: Particula,
            s_anillo: Anillo,
            particulas,
        }
    }
    pub fn camara(m: &MisionAdrian, w: usize, h: usize) -> Camara {
        let mut cam = Camara::nueva(w as f32, h as f32);
        let p = m.marco.local(m.nave.pos);
        let intro = if m.fase == Fase::Petrova {
            (m.intro / 5.0) as f32
        } else {
            1.0
        };
        let giro = (intro * std::f32::consts::PI).sin() * 1.5;
        cam.pos = m
            .marco
            .mundo(v3(p.x * 0.32 + giro, p.y * 0.3 + 2.7, p.z - 8.5));
        cam.mirar_a(m.marco.mundo(v3(p.x * 0.15, p.y * 0.12, p.z + 15.0)));
        cam.fov = (62.0 + 10.0 * (1.0 - intro)).to_radians();
        cam
    }
    pub fn dibujar(&self, fb: &mut Framebuffer, m: &MisionAdrian, nave: &Nave) {
        let cam = Self::camara(m, fb.width, fb.height);
        let t = m.visual as f32;
        let cinematica = m.fase == Fase::Petrova;
        let intensidad = if cinematica {
            let q = m.intro as f32;
            0.5 + 1.0 * smoothstep(0.0, 2.2, q) * (1.0 - smoothstep(3.5, 5.0, q))
        } else {
            0.32
        };
        let fondo = Fondo { intensidad };
        let (f, r, u) = cam.base();
        let billboard = Rot::de_columnas(r, u, -f);
        let mut items = vec![];
        let p = m.marco.local(m.nave.pos);
        agregar(
            &mut items,
            &cam,
            &self.cuadrado,
            &fondo,
            m.marco.mundo(v3(0.0, 0.0, p.z + 90.0)),
            billboard,
            v3(150.0, 100.0, 1.0),
            Mezcla::Opaco,
            v3(1.0, 1.0, 1.0),
        );
        let cantidad = if cinematica {
            (120.0
                + 600.0
                    * smoothstep(0.1, 2.5, m.intro as f32)
                    * (1.0 - smoothstep(3.7, 5.0, m.intro as f32))) as usize
        } else {
            85
        };
        for (i, (base, tam)) in self.particulas.iter().enumerate() {
            if i >= cantidad {
                break;
            }
            let z = (base.z - t * if cinematica { 22.0 } else { 9.0 }).rem_euclid(75.0) - 5.0 + p.z;
            let pos = m.marco.mundo(v3(base.x, base.y, z));
            agregar(
                &mut items,
                &cam,
                &self.cuadrado,
                &self.particula,
                pos,
                billboard,
                v3(*tam, *tam * 1.5, 1.0),
                Mezcla::Aditivo,
                if i % 5 == 0 {
                    v3(1.0, 0.5, 0.43)
                } else {
                    v3(0.75, 0.09, 0.13)
                } * intensidad,
            );
        }
        if !cinematica {
            let rot = Rot::de_columnas(m.marco.derecha, m.marco.up, -m.marco.fwd);
            agregar(
                &mut items,
                &cam,
                &self.marco,
                &self.s_anillo,
                m.marco.mundo(v3(0.0, 0.0, 45.0)),
                rot,
                v3(1.0, 1.0, 1.0),
                Mezcla::Alfa,
                v3(1.0, 1.0, 1.0),
            );
        }
        for o in &m.objetos {
            let pos = m.marco.mundo(o.pos);
            if !cam.esfera_visible(pos, o.radio * 1.6) {
                continue;
            }
            let rot = Rot::rot_y(t * 0.35 + o.fase).mul(&Rot::rot_x(t * 0.17));
            match o.tipo {
                Tipo::Muestra => {
                    let pulso = 1.0 + 0.055 * (t * 3.5 + o.fase).sin();
                    let tam = o.radio * pulso;
                    agregar(
                        &mut items,
                        &cam,
                        &self.esfera,
                        &self.nucleo,
                        pos,
                        rot,
                        v3(tam * 0.52, tam * 0.52, tam * 0.52),
                        Mezcla::Opaco,
                        v3(1.0, 1.0, 1.0),
                    );
                    agregar(
                        &mut items,
                        &cam,
                        &self.membrana,
                        &self.piel,
                        pos,
                        rot,
                        v3(tam, tam, tam),
                        Mezcla::Alfa,
                        v3(1.0, 1.0, 1.0),
                    );
                    agregar(
                        &mut items,
                        &cam,
                        &self.cuadrado,
                        &self.halo,
                        pos,
                        billboard,
                        v3(tam * 1.6, tam * 1.6, 1.0),
                        Mezcla::Aditivo,
                        v3(1.0, 1.0, 1.0),
                    );
                }
                Tipo::Obstaculo => {
                    let px = o.radio / (pos - cam.pos).len().max(0.1) * cam.escala_px();
                    let lod = if px > 50.0 {
                        0
                    } else if px > 15.0 {
                        1
                    } else {
                        2
                    };
                    let mesh = &self.rocas[o.variante * 3 + lod];
                    let tam = o.radio / mesh.radio;
                    agregar(
                        &mut items,
                        &cam,
                        mesh,
                        &self.roca,
                        pos,
                        rot,
                        v3(tam, tam, tam),
                        Mezcla::Opaco,
                        v3(0.58, 0.7, 1.12),
                    );
                }
            }
        }
        if m.fase == Fase::Extraccion {
            let rot = Rot::de_columnas(m.marco.derecha, -m.marco.fwd, m.marco.up);
            agregar(
                &mut items,
                &cam,
                &self.anillo,
                &self.s_anillo,
                m.marco.mundo(m.extraccion),
                rot,
                v3(3.2, 1.0, 3.2),
                Mezcla::Alfa,
                v3(1.0, 1.0, 1.0),
            );
        }
        if m.invulnerable <= 0.0 || (t * 10.0).fract() < 0.65 {
            let inclinacion = -m.nave.vel.dot(m.marco.derecha) * 0.018;
            nave.emitir_pose(
                &cam,
                PoseNave {
                    pos: m.nave.pos,
                    fwd: m.nave.fwd,
                    up: girar(m.nave.up, m.nave.fwd, inclinacion),
                    empuje: if cinematica { 1.2 } else { 0.7 },
                },
                &mut items,
                &mut Stats::default(),
            );
        }
        let (mut opacos, mut alpha): (Vec<_>, Vec<_>) =
            items.into_iter().partition(|i| i.mezcla == Mezcla::Opaco);
        alpha.sort_by(|a, b| b.dist.total_cmp(&a.dist));
        opacos.extend(alpha);
        let items = opacos;
        for p in fb.pixels.chunks_exact_mut(4) {
            p.copy_from_slice(&[4, 5, 13, 255]);
        }
        fb.zbuf.fill(0.0);
        let vista = Vista::de(&cam, fb.width, fb.height);
        let mut tris = vec![];
        for (i, item) in items.iter().enumerate() {
            preparar_item(item, i as u32, &vista, &mut tris);
        }
        let ctx = Contexto {
            cam: cam.pos,
            tiempo: t,
            sol: m.marco.mundo(v3(-25.0, 35.0, 15.0)),
        };
        let w = fb.width;
        const BANDA: usize = 20;
        let trabajos = Mutex::new(
            fb.pixels
                .chunks_mut(w * 4 * BANDA)
                .zip(fb.zbuf.chunks_mut(w * BANDA))
                .enumerate(),
        );
        std::thread::scope(|s| {
            for _ in 0..nucleos() {
                s.spawn(|| loop {
                    let sig = trabajos.lock().unwrap().next();
                    let Some((b, (color, z))) = sig else { break };
                    rasterizar_banda(b * BANDA, color, z, w, &tris, &items, &ctx);
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mision_adrian::{Configuracion, Objeto};
    #[test]
    fn renderiza_petrovay_muestras_y_reutiliza_las_rocas_del_cinturon() {
        let escena = EscenaAdrian::nueva();
        assert_eq!(escena.rocas[0].pos, malla::asteroide(100, 14, 9).pos);
        let nave = Nave::predeterminada().unwrap();
        let original = (
            nave.pos,
            nave.fwd,
            nave.up,
            nave.vel,
            nave.integridad,
            nave.modo_local,
        );
        let mut m = MisionAdrian::nueva(
            5,
            v3(1.0, 0.2, 0.0),
            v3(0.0, 1.0, 0.0),
            Configuracion::default(),
        );
        m.intro = 2.5;
        m.visual = 2.5;
        let mut fb = Framebuffer::new(160, 100);
        escena.dibujar(&mut fb, &m, &nave);
        let rojo = fb
            .pixels
            .chunks_exact(4)
            .filter(|p| p[0] > p[1].saturating_add(15))
            .count();
        assert!(rojo > 1000);
        let intro = fb.pixels.clone();
        m.actualizar(5.0, &crate::nave::Control::default());
        m.comenzar();
        m.objetos.push(Objeto {
            id: 1,
            tipo: Tipo::Muestra,
            pos: v3(0.0, 1.5, 2.0),
            radio: 1.4,
            variante: 0,
            fase: 0.0,
        });
        escena.dibujar(&mut fb, &m, &nave);
        assert_ne!(fb.pixels, intro);
        assert!(fb
            .pixels
            .chunks_exact(4)
            .any(|p| p[1] > 160 && p[1] > p[0].saturating_add(60)));
        assert_eq!(
            original,
            (
                nave.pos,
                nave.fwd,
                nave.up,
                nave.vel,
                nave.integridad,
                nave.modo_local
            )
        );
    }
}
