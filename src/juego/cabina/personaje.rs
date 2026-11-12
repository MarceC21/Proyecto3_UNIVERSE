//! Personaje entregado: cuerpo, cabeza y brazos, conservando los materiales OBJ.
use crate::{
    dibujo::Item,
    dibujo_interior::Parte,
    intro::{Intro, ESCALA_PERSONAJE, POS_PERSONAJE, YAW_PERSONAJE},
    malla::Malla,
    materiales::{Luz, Mat, ShaderInterior},
    math::*,
    obj::Modelo,
};
use std::sync::Arc;
pub struct Personaje {
    partes: Vec<(usize, Parte)>,
}
fn pivote(grupo: usize) -> V3 {
    match grupo {
        1 => v3(0.0, 0.64, 0.0),
        2 => v3(-0.23, 0.54, 0.0),
        3 => v3(0.23, 0.54, 0.0),
        _ => V3::ZERO,
    }
}
impl Personaje {
    pub fn nuevo(luces: Arc<Vec<Luz>>, ambiente: V3) -> Self {
        let modelo = Modelo::desde_textos(
            include_str!("../../../assets/compartidos/personaje/personaje.obj"),
            include_str!("../../../assets/compartidos/personaje/personaje.mtl"),
        )
        .expect("personaje.obj valido");
        let nm = modelo.materiales.len();
        let mut grupos = vec![vec![]; nm * 4];
        for cara in &modelo.caras {
            let p = cara.v.map(|i| {
                let p = modelo.vertices[i];
                v3(p[0], p[1], p[2])
            });
            let c = (p[0] + p[1] + p[2]) * (1.0 / 3.0);
            let grupo = if c.y > 0.62 {
                1
            } else if c.x.abs() > 0.18 && c.y > 0.28 {
                if c.x < 0.0 {
                    2
                } else {
                    3
                }
            } else {
                0
            };
            grupos[grupo * nm + cara.material].push(p);
        }
        let mut partes = vec![];
        for (g, tris) in grupos.into_iter().enumerate() {
            if tris.is_empty() {
                continue;
            }
            let grupo = g / nm;
            let (mut pos, mut nrm, mut uv, mut idx) = (vec![], vec![], vec![], vec![]);
            for t in tris {
                let b = pos.len() as u32;
                let n = (t[1] - t[0]).cross(t[2] - t[0]).norm();
                pos.extend(t.map(|p| p - pivote(grupo)));
                nrm.extend([n; 3]);
                uv.extend([[0.0, 0.0]; 3]);
                idx.push([b, b + 1, b + 2]);
            }
            let k = modelo.materiales[g % nm].color;
            let radio = pos.iter().map(|p: &V3| p.len()).fold(0.0, f32::max);
            let malla = Malla {
                pos,
                nrm,
                uv,
                idx,
                radio,
            };
            partes.push((
                grupo,
                Parte {
                    malla,
                    shader: ShaderInterior::nuevo(Mat::Casco, luces.clone(), ambiente),
                    tinte: v3(k[0], k[1], k[2]),
                    doble: true,
                },
            ));
        }
        Self { partes }
    }
    pub fn items(&self, intro: &Intro) -> Vec<Item<'_>> {
        let cuerpo = Rot::rot_y(YAW_PERSONAJE);
        let s = ESCALA_PERSONAJE;
        let pos = POS_PERSONAJE + v3(0.0, intro.respiracion() * s, 0.0);
        self.partes
            .iter()
            .map(|(grupo, p)| {
                let local = match grupo {
                    1 => Rot::rot_y(intro.giro_cabeza()),
                    2 => Rot::rot_x(intro.balanceo_brazos()),
                    3 => Rot::rot_x(-intro.balanceo_brazos()),
                    _ => Rot::identidad(),
                };
                p.item(
                    pos + cuerpo.aplicar(pivote(*grupo) * s),
                    cuerpo.mul(&local),
                    v3(s, s, s),
                )
            })
            .collect()
    }
}
