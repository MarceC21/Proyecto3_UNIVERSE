// El OBJ original se conserva como recurso. Rust lo carga y lo adapta al
// mismo pipeline de triángulos, normales y profundidad que usan los planetas.
use crate::malla::Malla;
use crate::math::*;
use crate::obj::Modelo;
use crate::shader::ShaderEstacion;

pub struct Estacion {
    pub pos: V3,
    pub radio: f32,
    pub rot: Rot,
    pub partes: Vec<(Malla, ShaderEstacion)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camara::Camara;
    use crate::dibujo::{Item, Mezcla};
    use crate::raster::{preparar_item, rasterizar_banda, Framebuffer, Vista};
    use crate::shader::Contexto;

    #[test]
    fn modelo_separado_de_tierra_y_dentro_de_su_esfera() {
        let tierra = v3(0.0, 0.0, -64.0);
        let e = Estacion::nueva(tierra, V3::ZERO);
        assert!((e.pos - tierra).len() > 4.2 * 1.025 + e.radio + 1.0);
        assert_eq!(e.partes.iter().map(|(m, _)| m.idx.len()).sum::<usize>(), 5811);
        for (m, _) in &e.partes {
            assert!(m.pos.iter().all(|p| p.len() <= e.radio + 1e-5));
            assert!(m.nrm.iter().all(|n| (n.len() - 1.0).abs() < 1e-4));
        }
    }

    #[test]
    fn profundidad_estable_al_invertir_orden_de_triangulos() {
        let e = Estacion::nueva(v3(0.0, 0.0, -64.0), V3::ZERO);
        // Ambas caras del anillo deben verse con descarte de caras activo.
        for altura in [-6.0, 6.0] {
            let mut cam = Camara::nueva(320.0, 200.0);
            cam.pos = e.pos + v3(8.0, altura, 9.0);
            cam.mirar_a(e.pos);
            let vista = Vista::de(&cam, 320, 200);
            let ctx = Contexto { cam: cam.pos, tiempo: 0.0, sol: V3::ZERO };
            let items: Vec<_> = e.partes.iter().map(|(malla, shader)| Item {
                malla, shader, pos: e.pos, rot: e.rot, escala: v3(1.0, 1.0, 1.0),
                mezcla: Mezcla::Opaco, doble_cara: false, tinte: v3(1.0, 1.0, 1.0),
                mip: 0.0, dist: 1.0,
            }).collect();
            let mut tris = vec![];
            for (id, item) in items.iter().enumerate() {
                preparar_item(item, id as u32, &vista, &mut tris);
            }
            let mut a = Framebuffer::new(320, 200);
            rasterizar_banda(0, &mut a.pixels, &mut a.zbuf, 320, &tris, &items, &ctx);
            assert!(a.zbuf.iter().filter(|z| **z > 0.0).count() > 500);
            tris.reverse();
            let mut b = Framebuffer::new(320, 200);
            rasterizar_banda(0, &mut b.pixels, &mut b.zbuf, 320, &tris, &items, &ctx);
            assert!(a.zbuf.iter().zip(&b.zbuf).all(|(a, b)| (a - b).abs() < 1e-5));
        }
    }
}

impl Estacion {
    pub fn nueva(tierra: V3, sol: V3) -> Self {
        let modelo = Modelo::desde_textos(
            include_str!("../../../assets/galaxias/sistema_solar/estacion/estacion.obj"),
            include_str!("../../../assets/galaxias/sistema_solar/estacion/estacion.mtl"),
        ).expect("el recurso de la estación debe ser un OBJ válido");
        let hacia_sol = (sol - tierra).norm();
        let tangente = v3(0.0, 1.0, 0.0).cross(hacia_sol).norm();
        let pos = tierra + hacia_sol * 7.0 + tangente * 10.0 + v3(0.0, 2.0, 0.0);
        Self::desde_modelo(modelo, 3.2, pos, Rot::rot_y(0.35).mul(&Rot::rot_z(0.12)))
    }

    // Emisor OBJ compartido por la estación y la nave de encuentro de Tau Ceti.
    pub(crate) fn desde_modelo(modelo: Modelo, radio: f32, pos: V3, rot: Rot) -> Self {
        let (centro, _) = modelo.centro_y_radio();
        let centro = v3(centro[0], centro[1], centro[2]);
        let vector = |v: &[f32; 3]| v3(v[0], v[1], v[2]);
        let radio_original = modelo.vertices.iter()
            .map(|v| (vector(v) - centro).len()).fold(0.0, f32::max);
        let escala = radio / radio_original;
        let mut partes = Vec::new();
        for (i, material) in modelo.materiales.iter().enumerate() {
            let mut m = Malla { pos: vec![], nrm: vec![], uv: vec![], idx: vec![], radio };
            for cara in modelo.caras.iter().filter(|c| c.material == i) {
                let p = cara.v.map(|id| (vector(&modelo.vertices[id]) - centro) * escala);
                let n = (p[1] - p[0]).cross(p[2] - p[0]).norm();
                if n.len() < 0.5 { continue; }
                let base = m.pos.len() as u32;
                m.pos.extend(p);
                m.nrm.extend([n; 3]);
                m.uv.extend([[0.0; 2]; 3]);
                m.idx.push([base, base + 1, base + 2]);
            }
            if !m.idx.is_empty() {
                partes.push((m, ShaderEstacion::nuevo(&material.nombre, vector(&material.color))));
            }
        }
        Self { pos, radio, rot, partes }
    }
}
