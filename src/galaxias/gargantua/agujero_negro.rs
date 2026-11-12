// Horizonte oscuro, disco de acreción y arco luminoso. Es una representación
// artística rasterizada en CPU, sin integración relativista de rayos.
use crate::{camara::Camara, dibujo::*, gravedad::CampoGravitatorio, malla::{self, Malla}, math::*, shader::*};

pub const RADIO_HORIZONTE: f32 = 108.0; // Tres veces el radio anterior de 36.
pub const RADIO_DISCO: f32 = 294.0;     // Disco más extendido; deja espacio antes de Miller.

pub struct AgujeroNegro {
    pub campo: CampoGravitatorio,
    disco: Malla,
    halo: Malla,
    shader_disco: ShaderDisco,
    shader_halo: ShaderHalo,
}
pub struct ShaderHorizonte;
impl Shader for ShaderHorizonte {
    fn shade(&self, _: &Fragmento, _: &Contexto) -> [f32; 4] { [0.0, 0.0, 0.0, 1.0] }
}
struct ShaderDisco;
impl Shader for ShaderDisco {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let r = f.uv[1];
        let a = f.uv[0] * std::f32::consts::TAU;
        let bandas = (r * 115.0 + (a * 3.0 - c.tiempo * 1.4).sin() * 0.8).sin() * 0.5 + 0.5;
        let remolino = 0.85 + 0.15 * (a * 5.0 + r * 9.0 - c.tiempo * 2.8).sin();
        let caliente = (1.0 - r).powi(2);
        let col = mezcla(rgb(232.0, 82.0, 22.0), rgb(255.0, 223.0, 137.0), caliente);
        let col = mezcla(col, rgb(255.0, 252.0, 225.0), caliente.powi(3)) * ((0.72 + bandas * 0.4) * remolino);
        let alfa = smoothstep(0.0, 0.045, r) * (1.0 - smoothstep(0.78, 1.0, r)) * 0.97;
        [col.x, col.y, col.z, alfa]
    }
}
struct ShaderHalo;
impl Shader for ShaderHalo {
    fn shade(&self, f: &Fragmento, _: &Contexto) -> [f32; 4] {
        let x = (f.uv[0] - 0.5) * 2.0;
        let y = (f.uv[1] - 0.5) * 2.0;
        let r = (x * x + y * y).sqrt();
        if r < 0.875 || r > 1.0 { return [0.0; 4]; }
        let arco = (-((r - 0.90) / 0.013).powi(2)).exp();
        let resplandor = (-((r - 0.91) / 0.055).powi(2)).exp() * 0.45;
        let col = mezcla(rgb(255.0, 128.0, 37.0), rgb(255.0, 247.0, 206.0), arco);
        [col.x, col.y, col.z, (arco + resplandor).min(1.0)]
    }
}

impl AgujeroNegro {
    pub fn nuevo() -> Self {
        Self { campo: CampoGravitatorio { centro: V3::ZERO, radio_horizonte: RADIO_HORIZONTE,
            radio_influencia: 600.0, mu: 162000.0, aceleracion_max: 22.0 },
            disco: malla::anillo(RADIO_HORIZONTE * 1.07, RADIO_DISCO, 256, 1.0),
            halo: malla::cuadrado(), shader_disco: ShaderDisco, shader_halo: ShaderHalo }
    }
    pub fn emitir<'a>(&'a self, cam: &Camara, aj: &Ajustes, out: &mut Vec<Item<'a>>, st: &mut Stats) {
        st.objetos_total += 2;
        st.tris_sin_optimizar += self.disco.idx.len() + self.halo.idx.len();
        let pos = self.campo.centro;
        if aj.culling && !cam.esfera_visible(pos, RADIO_DISCO) { return; }
        let distancia = (cam.pos - pos).len().max(RADIO_HORIZONTE + 0.01);
        let (f, r, u) = cam.base();
        // Compensar el radio aparente de la esfera al acercarse a ella.
        let radio_aparente = RADIO_HORIZONTE * distancia / (distancia * distancia - RADIO_HORIZONTE.powi(2)).sqrt();
        let escala = radio_aparente * 1.12;
        out.push(Item { malla: &self.disco, shader: &self.shader_disco, pos,
            rot: Rot::rot_x(0.18).mul(&Rot::rot_z(0.10)), escala: v3(1.0, 1.0, 1.0),
            mezcla: Mezcla::Alfa, doble_cara: true, tinte: v3(1.0, 1.0, 1.0), mip: 0.0, dist: distancia });
        out.push(Item { malla: &self.halo, shader: &self.shader_halo, pos,
            rot: Rot::de_columnas(r, u, -f), escala: v3(escala, escala, escala),
            mezcla: Mezcla::Aditivo, doble_cara: true, tinte: v3(1.0, 1.0, 1.0), mip: 0.0, dist: distancia });
        st.objetos_enviados += 2;
        st.tris_enviados += self.disco.idx.len() + self.halo.idx.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn horizonte_negro_y_disco_animado_con_profundidad() {
        use crate::{cielo::Cielo, escena::Escena, raster::Framebuffer, render::dibujar};
        let e = Escena::gargantua();
        let cielo = Cielo::gargantua();
        let mut cam = Camara::nueva(320.0, 200.0);
        cam.pos = v3(0.0, RADIO_HORIZONTE * 0.9, RADIO_HORIZONTE * 4.2);
        cam.mirar_a(V3::ZERO);
        let mut fb = Framebuffer::new(320, 200);
        let st = dibujar(&mut fb, &e, &cielo, &cam, 0.0, &Ajustes::default());
        let centro = (100 * 320 + 160) * 4;
        assert!(fb.pixels[centro..centro + 3].iter().all(|v| *v < 8));
        assert!(fb.zbuf[100 * 320 + 160] > 0.0);
        let luminosos = fb.pixels.chunks_exact(4).filter(|p| p[0] > 170 && p[1] > 70 && p[0] > p[2]).count();
        assert!(luminosos > 100, "no aparece el disco: {luminosos}");
        assert!(st.tris_sin_optimizar < 120_000);
        let f = Fragmento { pos: v3(35.0, 0.0, 0.0), normal: v3(0.0, 1.0, 0.0), uv: [0.2, 0.35],
            tinte: v3(1.0, 1.0, 1.0), centro: V3::ZERO, mip: 0.0 };
        let mut ctx = Contexto { cam: cam.pos, sol: V3::ZERO, tiempo: 0.0 };
        let a = ShaderDisco.shade(&f, &ctx);
        ctx.tiempo = 1.0;
        assert_ne!(a, ShaderDisco.shade(&f, &ctx));
    }
}
