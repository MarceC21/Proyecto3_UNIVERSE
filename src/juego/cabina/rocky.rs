//! OBJ del boceto, animado por extremidades con pasos alternados como TARS.
use crate::{
    dibujo::Item,
    dibujo_interior::Parte,
    encuentro_rocky::EncuentroRocky,
    malla::Malla,
    materiales::{Luz, Mat, ShaderInterior},
    math::*,
    obj::Modelo,
};
use std::{f32::consts::TAU, sync::Arc};
const ESCALA: f32 = 1.20;
pub struct Rocky {
    partes: Vec<(usize, Parte)>,
}
impl Rocky {
    pub fn nuevo(luces: Arc<Vec<Luz>>, ambiente: V3) -> Self {
        let modelo = Modelo::desde_textos(
            include_str!("../../../assets/compartidos/rocky/rocky.obj"),
            include_str!("../../../assets/compartidos/rocky/rocky.mtl"),
        )
        .expect("boceto Rocky válido");
        let nm = modelo.materiales.len();
        let mut grupos = vec![vec![]; 6 * nm];
        for cara in &modelo.caras {
            let p = cara.v.map(|i| {
                let p = modelo.vertices[i];
                v3(p[0], p[1], p[2])
            });
            let centro = (p[0] + p[1] + p[2]) * (1.0 / 3.0);
            // Caparazón central y cinco patas radiales (el OBJ mira hacia +Z).
            let extremidad = if centro.x.hypot(centro.z) < 0.43 {
                5
            } else {
                (centro.x.atan2(centro.z) / TAU * 5.0)
                    .round()
                    .rem_euclid(5.0) as usize
            };
            grupos[extremidad * nm + cara.material].push(p);
        }
        let mut partes = vec![];
        for (g, tris) in grupos.into_iter().enumerate() {
            if tris.is_empty() {
                continue;
            }
            let (mut pos, mut nrm, mut uv, mut idx) = (vec![], vec![], vec![], vec![]);
            for t in tris {
                let n = (t[1] - t[0]).cross(t[2] - t[0]).norm();
                let b = pos.len() as u32;
                pos.extend(t);
                nrm.extend([n; 3]);
                uv.extend([[0.0, 0.0]; 3]);
                idx.push([b, b + 1, b + 2]);
            }
            let color = modelo.materiales[g % nm].color;
            partes.push((
                g / nm,
                Parte {
                    malla: Malla {
                        pos,
                        nrm,
                        uv,
                        idx,
                        radio: 2.0,
                    },
                    shader: ShaderInterior::nuevo(Mat::Casco, luces.clone(), ambiente),
                    tinte: v3(color[0], color[1], color[2]),
                    doble: false,
                },
            ));
        }
        Self { partes }
    }
    pub fn items(&self, estado: &EncuentroRocky, tiempo: f32) -> Vec<Item<'_>> {
        let rot = Rot::rot_y(estado.angulo);
        self.partes
            .iter()
            .map(|(p, parte)| {
                let fase = estado.paso / 0.62 * TAU + *p as f32 * TAU * 2.0 / 5.0;
                let offset = if *p == 5 {
                    v3(0.0, 0.012 * (tiempo * 2.0).sin(), 0.0)
                } else if estado.moviendo {
                    v3(0.0, 0.07 * fase.sin().max(0.0), 0.035 * fase.cos())
                } else {
                    V3::ZERO
                };
                parte.item(
                    estado.pos + rot.aplicar(offset),
                    rot,
                    v3(ESCALA, ESCALA, ESCALA),
                )
            })
            .collect()
    }
}
