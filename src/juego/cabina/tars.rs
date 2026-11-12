// TARS: el modelo tars.obj se parte en sus cuatro columnas (por la x de cada cara) para
// animar los pasos alternados como en el proyecto 2. Patrulla la cabina y mira al jugador.
use crate::dibujo_interior::Parte;
use crate::jugador::{Mundo, Obst};
use crate::malla::Malla;
use crate::materiales::{Luz, Mat, ShaderInterior};
use crate::math::*;
use crate::obj::Modelo;
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

pub const ESCALA: f32 = 2.10;
const PASO: f32 = 0.215;
pub const RADIO: f32 = 0.48 * ESCALA;
const VELOCIDAD: f32 = 0.85;
const VELOCIDAD_GIRO: f32 = 1.8;
const LONGITUD_PASO: f32 = 0.62;

#[derive(PartialEq, Clone, Copy)]
enum Etapa {
    Esperar,
    Caminar,
}

pub struct Tars {
    pub origen: V3,
    pub angulo: f32,
    cols: [V3; 4],
    partes: Vec<(usize, Parte)>,
    ruta: [(f32, f32); 2],
    destino: usize,
    etapa: Etapa,
    espera: f32,
    fase: f32,
    amplitud: f32,
}

impl Tars {
    pub fn nuevo(luces: Arc<Vec<Luz>>, ambiente: V3) -> Tars {
        let modelo = Modelo::desde_textos(
            include_str!("../../../assets/compartidos/tars/tars.obj"),
            include_str!("../../../assets/compartidos/tars/tars.mtl"),
        )
        .expect("tars.obj debe ser un OBJ válido");
        let mut grupos: Vec<Vec<[V3; 3]>> = vec![vec![]; 4 * modelo.materiales.len()];
        let nm = modelo.materiales.len();
        for cara in &modelo.caras {
            let p = cara.v.map(|i| {
                let v = modelo.vertices[i];
                v3(v[0], v[1], v[2])
            });
            let cx = (p[0].x + p[1].x + p[2].x) / 3.0;
            let col = ((cx / PASO) + 1.5).round().clamp(0.0, 3.0) as usize;
            grupos[col * nm + cara.material].push(p);
        }
        let mut partes = vec![];
        for (g, tris) in grupos.into_iter().enumerate() {
            if tris.is_empty() {
                continue;
            }
            let (col, mi) = (g / nm, g % nm);
            let (mut pos, mut nrm, mut uv, mut idx) = (vec![], vec![], vec![], vec![]);
            for t in &tris {
                let n = (t[1] - t[0]).cross(t[2] - t[0]).norm();
                let b = pos.len() as u32;
                for q in t {
                    pos.push(*q);
                    nrm.push(n);
                    uv.push([0.0, 0.0]);
                }
                idx.push([b, b + 1, b + 2]);
            }
            let k = modelo.materiales[mi].color;
            partes.push((
                col,
                Parte {
                    malla: Malla {
                        pos,
                        nrm,
                        uv,
                        idx,
                        radio: 2.0,
                    },
                    shader: ShaderInterior::nuevo(Mat::Casco, luces.clone(), ambiente),
                    tinte: v3(k[0], k[1], k[2]) * 1.15,
                    doble: false,
                },
            ));
        }
        Tars {
            origen: v3(7.6, 0.0, -6.5),
            angulo: -PI / 2.0,
            cols: [V3::ZERO; 4],
            partes,
            ruta: [(7.6, -6.5), (-7.6, -6.5)],
            destino: 0,
            etapa: Etapa::Esperar,
            espera: 5.0,
            fase: 0.0,
            amplitud: 0.0,
        }
    }

    fn girar(&mut self, objetivo: f32, dt: f32) -> bool {
        let dif = (objetivo - self.angulo + PI).rem_euclid(TAU) - PI;
        let paso = VELOCIDAD_GIRO * dt;
        self.angulo = (self.angulo + dif.clamp(-paso, paso) + PI).rem_euclid(TAU) - PI;
        dif.abs() <= paso
    }

    pub fn obstaculo(&self) -> Obst {
        Obst::caja(self.origen.x, self.origen.z, 2.0 * RADIO, 2.0 * RADIO)
    }

    pub fn distancia(&self, x: f32, z: f32) -> f32 {
        ((self.origen.x - x).powi(2) + (self.origen.z - z).powi(2)).sqrt()
    }

    // `mundo.dinamicos` no debe incluir a TARS (se bloquearía a sí mismo).
    pub fn actualizar(&mut self, dt: f32, mundo: &Mundo, jx: f32, jz: f32, conversando: bool) {
        let mut avance = 0.0;
        let mirar_jugador = (jx - self.origen.x).atan2(jz - self.origen.z);
        let cerca = self.distancia(jx, jz) < 3.5;
        if conversando {
            self.girar(mirar_jugador, dt);
        } else {
            match self.etapa {
                Etapa::Esperar => {
                    if cerca {
                        self.girar(mirar_jugador, dt);
                    }
                    self.espera = (self.espera - dt).max(0.0);
                    if self.espera == 0.0 {
                        self.destino = (self.destino + 1) % 2;
                        self.etapa = Etapa::Caminar;
                    }
                }
                Etapa::Caminar => {
                    let (tx, tz) = self.ruta[self.destino];
                    let (dx, dz) = (tx - self.origen.x, tz - self.origen.z);
                    let dist = (dx * dx + dz * dz).sqrt();
                    if dist < 0.05 {
                        self.etapa = Etapa::Esperar;
                        self.espera = 6.0;
                    } else if self.girar(dx.atan2(dz), dt) {
                        let paso = (VELOCIDAD * dt).min(dist);
                        let (x, z) = (
                            self.origen.x + dx / dist * paso,
                            self.origen.z + dz / dist * paso,
                        );
                        let jugador_cerca =
                            ((jx - x).powi(2) + (jz - z).powi(2)).sqrt() < RADIO + 0.35 + 0.25;
                        if !jugador_cerca && mundo.libre(x, z, RADIO * 0.9) {
                            self.origen.x = x;
                            self.origen.z = z;
                            avance = paso;
                        }
                    }
                }
            }
        }
        let objetivo = if avance > 0.0 { 1.0 } else { 0.0 };
        self.amplitud += (objetivo - self.amplitud).clamp(-dt * 5.0, dt * 5.0);
        self.fase = (self.fase + avance / LONGITUD_PASO * TAU).rem_euclid(TAU);
        for i in 0..4 {
            let f = self.fase + if i % 2 == 0 { 0.0 } else { PI };
            self.cols[i] = v3(
                0.0,
                f.sin().max(0.0) * 0.10 * self.amplitud,
                f.cos() * 0.07 * self.amplitud,
            );
        }
    }

    pub fn items(&self) -> Vec<crate::dibujo::Item<'_>> {
        self.items_columnas(self.origen, self.angulo, self.cols)
    }

    // La playa reutiliza el mismo modelo y el mismo balanceo de cuatro columnas.
    pub fn items_pose(
        &self,
        origen: V3,
        angulo: f32,
        paso: f32,
        moviendo: bool,
    ) -> Vec<crate::dibujo::Item<'_>> {
        let mut cols = [V3::ZERO; 4];
        if moviendo {
            for (i, col) in cols.iter_mut().enumerate() {
                let f = paso / LONGITUD_PASO * TAU + if i % 2 == 0 { 0.0 } else { PI };
                *col = v3(0.0, f.sin().max(0.0) * 0.10, f.cos() * 0.07);
            }
        }
        self.items_columnas(origen, angulo, cols)
    }
    fn items_columnas(
        &self,
        origen: V3,
        angulo: f32,
        cols: [V3; 4],
    ) -> Vec<crate::dibujo::Item<'_>> {
        let rot = Rot::rot_y(angulo);
        self.partes
            .iter()
            .map(|(c, p)| {
                let pos = origen + rot.aplicar(cols[*c] * ESCALA);
                p.item(pos, rot, v3(ESCALA, ESCALA, ESCALA))
            })
            .collect()
    }
}
