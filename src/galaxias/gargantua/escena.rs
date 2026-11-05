// Configuración de Gargantua; mallas, shaders, LOD y emisor compartidos con Solar.
use crate::escena::{Cuerpo, Escena};
use crate::generadores;
use crate::generadores_gargantua as gen;
use crate::malla;
use crate::math::*;
use crate::progreso::Galaxia;
use crate::shader::*;

pub const RADIO_LIMITE: f32 = 420.0;
pub const ALTO_LIMITE: f32 = 75.0;
pub const INICIO: V3 = v3(0.0, 9.0, 380.0);

impl Escena {
    pub fn gargantua() -> Self {
        let mut mallas = vec![malla::esfera(96, 64), malla::esfera(48, 32), malla::esfera(20, 12)];
        let m_cuadrado = mallas.len(); mallas.push(malla::cuadrado());
        let m_octaedro = mallas.len(); mallas.push(malla::octaedro());
        let m_muro = mallas.len(); mallas.push(malla::muro(RADIO_LIMITE, -ALTO_LIMITE, ALTO_LIMITE, 192));
        let m_anillo_limite = mallas.len(); mallas.push(malla::anillo(RADIO_LIMITE - 1.0, RADIO_LIMITE + 1.0, 384, 150.0));
        let mut shaders: Vec<Box<dyn Shader>> = vec![Box::new(ShaderSol { tex: generadores::sol() })];
        for (tex, atmosfera) in [
            (gen::miller(), rgb(130.0, 219.0, 232.0)),
            (gen::mann(), rgb(178.0, 219.0, 238.0)),
            (gen::edmunds(), rgb(240.0, 173.0, 108.0)),
            (gen::thalos(), rgb(161.0, 183.0, 235.0)),
            (gen::nyx(), rgb(185.0, 138.0, 214.0)),
            (gen::aurora(), rgb(136.0, 225.0, 189.0)),
        ] { shaders.push(Box::new(ShaderPlaneta { tex, atmosfera, fuerza_atm: 0.55 })); }
        let s_asteroide = shaders.len(); shaders.push(Box::new(ShaderAsteroide { tex: generadores::asteroide() }));
        let s_orbita = shaders.len(); shaders.push(Box::new(ShaderOrbita));
        let s_barrera = shaders.len(); shaders.push(Box::new(ShaderBarrera));
        let s_baliza = shaders.len(); shaders.push(Box::new(ShaderBaliza));
        let s_brillo = shaders.len(); shaders.push(Box::new(ShaderBrillo));
        let mut cuerpos = Vec::new();
        // Radios un poco mayores; el mismo presupuesto de triángulos que Solar.
        for (i, (nombre, distancia, radio, angulo, inclinacion)) in [
            ("Gargantua", 0.0f32, 18.0, 0.0f32, 0.0),
            ("Miller", 58.0, 7.0, 305.0, 0.12),
            ("Mann", 104.0, 8.0, 210.0, 0.36),
            ("Edmunds", 155.0, 8.5, 35.0, 0.45),
            ("Thalos", 220.0, 11.0, 280.0, 0.52),
            ("Nyx", 282.0, 9.2, 145.0, 0.18),
            ("Aurora", 352.0, 9.0, 65.0, 0.38),
        ].into_iter().enumerate() {
            let a = angulo.to_radians();
            let orbita = if i == 0 { None } else {
                let m = mallas.len();
                mallas.push(malla::anillo(distancia - 0.3, distancia + 0.3, 320, (distancia * std::f32::consts::TAU / 2.6).round()));
                Some((m, rgb(141.0, 211.0, 212.0)))
            };
            let mut anillos = vec![];
            if nombre == "Thalos" {
                // Dos bandas separadas por un hueco visible; mismo plano ecuatorial.
                for (interior, exterior) in [(1.35, 1.9), (2.15, 2.65)] {
                    let m = mallas.len(); mallas.push(malla::anillo(interior, exterior, 256, 1.0));
                    let s = shaders.len(); shaders.push(Box::new(ShaderAnillos { interior, exterior }));
                    anillos.push((m, s));
                }
            }
            cuerpos.push(Cuerpo { nombre, pos: v3(distancia * a.cos(), 0.0, distancia * a.sin()), radio,
                inclinacion, vel_rot: if i == 0 { 0.05 } else { 0.16 + i as f32 * 0.035 },
                angulo: 0.0, shader: i, es_sol: i == 0, nubes: None, angulo_nubes: 0.0, orbita, anillos });
        }
        Self { galaxia: Galaxia::Gargantua, radio_limite: RADIO_LIMITE, alto_limite: ALTO_LIMITE,
            cinturon: None, estacion: None, mallas, shaders, cuerpos,
            asteroides: vec![], cols_ast: vec![], m_cuadrado, m_octaedro, m_muro, m_anillo_limite,
            m_borde_cinturon: vec![], s_asteroide, s_orbita, s_barrera, s_baliza, s_brillo }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{camara::Camara, dibujo::{Ajustes, Stats}, nave::{Control, Mundo, Nave}};
    #[test]
    fn seis_planetas_miller_interior_dos_anillos_y_ningun_cinturon() {
        let mut e = Escena::gargantua();
        assert_eq!(e.cuerpos.len(), 7);
        assert_eq!(e.cuerpos[1].nombre, "Miller");
        assert!(e.cinturon.is_none() && e.estacion.is_none());
        assert!(e.colisionadores_asteroides().is_empty());
        assert_eq!(e.cuerpos[4].anillos.len(), 2);
        for c in &e.cuerpos[1..] {
            let extension = c.anillos.iter().map(|(m, _)| e.mallas[*m].radio).fold(1.0, f32::max) * c.radio;
            assert!(c.pos.len() + extension < RADIO_LIMITE - 20.0);
            assert!(c.radio >= 7.0 && c.pos.len() >= e.cuerpos[1].pos.len());
        }
        let angulo = e.cuerpos[1].angulo;
        e.actualizar(1.0);
        assert!(e.cuerpos[1].angulo != angulo);
        let cam = Camara::nueva(320.0, 200.0);
        let mut st = Stats::default();
        e.emitir(&cam, &Ajustes::default(), &mut vec![], &mut st);
        assert!(st.tris_sin_optimizar < 120_000);
    }
    #[test]
    fn vuelo_cerca_de_miller_y_reaparicion_en_el_sistema_nuevo() {
        let e = Escena::gargantua();
        let cuerpos = e.esferas();
        let mundo = Mundo { cuerpos: &cuerpos, asteroides: &[], radio_limite: RADIO_LIMITE,
            alto_limite: ALTO_LIMITE, cinturon: None };
        let miller = &e.cuerpos[1];
        let mut n = Nave::fisica();
        n.establecer_reaparicion(INICIO);
        n.teletransportar(miller.pos + v3(0.0, 0.0, 30.0), miller.pos);
        n.modo_local = true;
        let pos = n.pos;
        for _ in 0..120 { n.actualizar(&Control { lateral: 1.0, ..Default::default() }, 1.0 / 60.0, &mundo); }
        assert!((n.pos - pos).len() > 1.0);
        assert!((n.pos - miller.pos).len() > miller.radio);
        n.reaparecer();
        assert_eq!(n.pos, INICIO);
    }
}
