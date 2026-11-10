// Tau Ceti: estrella azul, Adrien (planeta principal), gigante gaseoso, un mundo
// pequeño con anillos, cuatro planetas más y un cinturón de asteroides al final
// de la galaxia. Reutiliza mallas, LOD, culling y shaders de Escena.
use crate::escena::{Asteroide, Cuerpo, Escena};
use crate::generadores;
use crate::generadores_tau_ceti as gt;
use crate::malla;
use crate::math::*;
use crate::progreso::Galaxia;
use crate::ruido::*;
use crate::shader::*;
use crate::textura::Textura;

pub const RADIO_LIMITE: f32 = 470.0;
pub const ALTO_LIMITE: f32 = 70.0;
pub const INICIO: V3 = v3(0.0, 8.0, 350.0); // entre el último planeta y el cinturón, mirando al centro
pub const RADIO_ESTRELLA: f32 = 12.0; // el Sol mide 10
pub const CINTURON_INT: f32 = 400.0;
pub const CINTURON_EXT: f32 = 436.0;
const NUM_ASTEROIDES: usize = 1400;
const SEMILLA_CINTURON: u64 = 5151;

// ---------- Estrella azul: mismo método que el Sol, otra rampa de color ----------
pub struct ShaderEstrella {
    pub tex: Textura,
}

impl Shader for ShaderEstrella {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let t = c.tiempo;
        let a = self.tex.muestrear(f.uv[0] + t * 0.012, f.uv[1], f.mip).x;
        let b = self
            .tex
            .muestrear(1.0 - f.uv[0] + t * 0.007 + 0.3, f.uv[1], f.mip)
            .x;
        let v = a * 0.6 + b * 0.4;
        let q = ((v * 4.0).floor() + smoothstep(0.35, 0.65, v * 4.0 - (v * 4.0).floor())) / 4.0;
        let col0 = mezcla(
            rgb(36.0, 130.0, 255.0),
            rgb(90.0, 220.0, 255.0),
            smoothstep(0.15, 0.55, q),
        );
        let mut col = mezcla(col0, rgb(232.0, 252.0, 255.0), smoothstep(0.55, 0.95, q));
        let ndv = f.normal.norm().dot((c.cam - f.pos).norm()).max(0.0);
        col = mezcla(col, rgb(30.0, 120.0, 255.0), (1.0 - ndv).powi(2) * 0.55);
        col = col * (1.0 + 0.05 * (t * 1.7).sin());
        [col.x, col.y, col.z, 1.0]
    }
}

// ---------- Resplandor cian con rayos (sprite aditivo) ----------
pub struct ShaderBrilloEstrella;

struct AnillosPurpura(ShaderAnillos);
impl Shader for AnillosPurpura {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let base = self.0.shade(f, c);
        let luz = base[0] * 0.5 + base[1] * 0.3 + base[2] * 0.2;
        [luz * 0.86, luz * 0.78, luz * 1.16, base[3]]
    }
}

impl Shader for ShaderBrilloEstrella {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let (x, y) = (f.uv[0] - 0.5, f.uv[1] - 0.5);
        let r = (x * x + y * y).sqrt() * 2.0;
        if r >= 1.0 {
            return [0.0; 4];
        }
        let ang = y.atan2(x);
        let aureola = (1.0 - r).powi(3);
        let rayos =
            (0.5 + 0.5 * (ang * 9.0 + c.tiempo * 0.25).cos()).powi(6) * (1.0 - r).powi(2) * 0.5;
        let col = mezcla(rgb(40.0, 130.0, 255.0), rgb(170.0, 240.0, 255.0), aureola);
        [col.x, col.y, col.z, (aureola * 0.85 + rayos) * f.tinte.x]
    }
}

impl Escena {
    pub fn tau_ceti() -> Self {
        // Mallas: [0..3] esferas, [3..15] asteroides (4 tipos x 3 LODs), luego decorado.
        let mut mallas = vec![
            malla::esfera(96, 64),
            malla::esfera(48, 32),
            malla::esfera(20, 12),
        ];
        for t in 0..4u32 {
            let s = 100 + t * 17;
            mallas.push(malla::asteroide(s, 14, 9));
            mallas.push(malla::asteroide(s, 8, 6));
            mallas.push(malla::asteroide(s, 5, 4));
        }
        let m_cuadrado = mallas.len();
        mallas.push(malla::cuadrado());
        let m_octaedro = mallas.len();
        mallas.push(malla::octaedro());
        let m_muro = mallas.len();
        mallas.push(malla::muro(RADIO_LIMITE, -ALTO_LIMITE, ALTO_LIMITE, 192));
        let m_anillo_limite = mallas.len();
        mallas.push(malla::anillo(
            RADIO_LIMITE - 1.0,
            RADIO_LIMITE + 1.0,
            384,
            150.0,
        ));
        let m_borde_cinturon = vec![mallas.len(), mallas.len() + 1];
        for r in [CINTURON_INT, CINTURON_EXT] {
            mallas.push(malla::anillo(
                r - 0.35,
                r + 0.35,
                320,
                (r * std::f32::consts::TAU / 2.6).round(),
            ));
        }

        // ---- Shaders ----
        let mut shaders: Vec<Box<dyn Shader>> = Vec::new();
        let mut agregar = |s: Box<dyn Shader>| {
            shaders.push(s);
            shaders.len() - 1
        };
        let s_estrella = agregar(Box::new(ShaderEstrella {
            tex: generadores::sol(),
        }));
        let s_adrien = agregar(Box::new(ShaderPlaneta {
            tex: gt::adrien(),
            atmosfera: rgb(120.0, 255.0, 90.0),
            fuerza_atm: 0.95,
        }));
        let s_gigante = agregar(Box::new(ShaderPlaneta {
            tex: gt::gigante(),
            atmosfera: rgb(130.0, 190.0, 255.0),
            fuerza_atm: 0.4,
        }));
        let s_ceti_nova = agregar(Box::new(ShaderPlaneta {
            tex: gt::ceti_nova(),
            atmosfera: rgb(110.0, 210.0, 255.0),
            fuerza_atm: 1.0,
        }));
        let s_ceti_nova_ii = agregar(Box::new(ShaderPlaneta {
            tex: gt::ceti_nova_ii(),
            atmosfera: rgb(230.0, 170.0, 120.0),
            fuerza_atm: 0.3,
        }));
        let s_oceanus = agregar(Box::new(ShaderPlaneta {
            tex: gt::oceanus(),
            atmosfera: rgb(100.0, 220.0, 235.0),
            fuerza_atm: 0.9,
        }));
        let s_saturn = agregar(Box::new(ShaderPlaneta {
            tex: gt::saturn(),
            atmosfera: rgb(200.0, 170.0, 255.0),
            fuerza_atm: 0.5,
        }));
        let s_veridia = agregar(Box::new(ShaderPlaneta {
            tex: gt::veridia(),
            atmosfera: rgb(255.0, 150.0, 120.0),
            fuerza_atm: 0.45,
        }));
        let s_aethel = agregar(Box::new(ShaderPlaneta {
            tex: gt::aethel(),
            atmosfera: rgb(200.0, 220.0, 140.0),
            fuerza_atm: 0.4,
        }));
        let (r_int, r_ext) = (1.35f32, 2.5f32); // anillos del pequeño, estilo Saturno
        let s_anillos = agregar(Box::new(AnillosPurpura(ShaderAnillos {
            interior: r_int,
            exterior: r_ext,
        })));
        let s_asteroide = agregar(Box::new(ShaderAsteroide {
            tex: generadores::asteroide(),
        }));
        let s_orbita = agregar(Box::new(ShaderOrbita));
        let s_barrera = agregar(Box::new(ShaderBarrera));
        let s_baliza = agregar(Box::new(ShaderBaliza));
        let s_brillo = agregar(Box::new(ShaderBrilloEstrella));

        // ---- Cuerpos: (nombre, distancia, radio, ángulo°, rotación, inclinación, shader) ----
        let mut cuerpos = vec![Cuerpo {
            nombre: "Tau Ceti",
            pos: V3::ZERO,
            radio: RADIO_ESTRELLA,
            inclinacion: 0.0,
            vel_rot: 0.05,
            angulo: 0.0,
            shader: s_estrella,
            es_sol: true,
            nubes: None,
            angulo_nubes: 0.0,
            orbita: None,
            anillos: vec![],
        }];
        let planetas: [(&'static str, f32, f32, f32, f32, f32, usize); 8] = [
            ("Adrien", 85.0, 7.0, 138.0, 0.50, 0.20, s_adrien), // principal: tamaño de Miller
            ("Gigante", 125.0, 9.0, 41.0, 0.90, 0.06, s_gigante), // como Júpiter
            ("Ceti Nova", 170.0, 5.0, 200.0, 0.60, 0.30, s_ceti_nova),
            ("Oceanus", 215.0, 3.2, 110.0, 0.70, 0.15, s_oceanus),
            ("Saturn", 255.0, 1.7, 312.0, 1.20, 0.45, s_saturn), // como Mercurio, con anillos
            (
                "Ceti Nova II",
                295.0,
                4.0,
                230.0,
                0.40,
                0.10,
                s_ceti_nova_ii,
            ),
            ("Veridia", 335.0, 4.6, 150.0, 0.45, 0.35, s_veridia),
            ("Aethel", 372.0, 3.8, 29.0, 0.55, 0.25, s_aethel),
        ];
        for (nombre, dist, radio, ang, rot, incl, shader) in planetas {
            let a = ang.to_radians();
            let i = mallas.len();
            mallas.push(malla::anillo(
                dist - 0.3,
                dist + 0.3,
                320,
                (dist * std::f32::consts::TAU / 2.6).round(),
            ));
            let anillos = if nombre == "Saturn" {
                let m = mallas.len();
                mallas.push(malla::anillo(r_int, r_ext, 256, 1.0));
                vec![(m, s_anillos)]
            } else {
                vec![]
            };
            cuerpos.push(Cuerpo {
                nombre,
                pos: v3(dist * a.cos(), 0.0, dist * a.sin()),
                radio,
                inclinacion: incl,
                vel_rot: rot * 0.5,
                angulo: 0.0,
                shader,
                es_sol: false,
                nubes: None,
                angulo_nubes: 0.0,
                orbita: Some((i, rgb(190.0, 225.0, 240.0))),
                anillos,
            });
        }

        // ---- Cinturón de asteroides al final de la galaxia (reproducible) ----
        let mut rng = Rng::new(SEMILLA_CINTURON);
        let mut asteroides = Vec::new();
        let paleta = [
            rgb(255.0, 255.0, 255.0),
            rgb(200.0, 230.0, 255.0),
            rgb(190.0, 255.0, 220.0),
            rgb(235.0, 205.0, 255.0),
        ];
        while asteroides.len() < NUM_ASTEROIDES {
            let ang = rng.rango(0.0, std::f32::consts::TAU);
            let r = CINTURON_INT + (CINTURON_EXT - CINTURON_INT) * (rng.f() + rng.f()) * 0.5;
            let y = (rng.f() + rng.f() + rng.f() - 1.5) * 6.0;
            let pos = v3(r * ang.cos(), y, r * ang.sin());
            let densidad = clamp01(fbm(pos * 0.045, 3, 7) * 0.9 + 0.55);
            if rng.f() > densidad * densidad {
                continue;
            }
            let u = rng.f();
            let escala = 0.45 + 4.2 * u * u * u * u;
            let rot = Rot::rot_y(rng.rango(0.0, 6.28))
                .mul(&Rot::rot_x(rng.rango(0.0, 6.28)))
                .mul(&Rot::rot_z(rng.rango(0.0, 6.28)));
            asteroides.push(Asteroide {
                pos,
                rot,
                escala,
                tipo: (rng.siguiente() % 4) as usize,
                tinte: paleta[(rng.siguiente() % 4) as usize] * rng.rango(0.8, 1.05),
            });
        }
        let cols_ast = asteroides
            .iter()
            .map(|a| {
                let primero = 3 + a.tipo * 3;
                let radio = mallas[primero..primero + 3]
                    .iter()
                    .map(|m| m.radio)
                    .fold(0.0, f32::max);
                (a.pos, a.escala * radio)
            })
            .collect();

        let nave_cafe = Some(crate::nave_cafe::crear(&cuerpos));
        Self {
            galaxia: Galaxia::TauCeti,
            radio_limite: RADIO_LIMITE,
            alto_limite: ALTO_LIMITE,
            cinturon: Some((CINTURON_INT, CINTURON_EXT)),
            agujero_negro: None,
            estacion: None,
            nave_cafe,
            mallas,
            shaders,
            cuerpos,
            asteroides,
            cols_ast,
            m_cuadrado,
            m_octaedro,
            m_muro,
            m_anillo_limite,
            m_borde_cinturon,
            s_asteroide,
            s_orbita,
            s_barrera,
            s_baliza,
            s_brillo,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn estrella_ocho_planetas_y_cinturon_al_final() {
        let e = Escena::tau_ceti();
        assert_eq!(e.cuerpos.len(), 9);
        assert!(e.cuerpos[0].es_sol && e.cuerpos[0].radio > 10.0);
        assert_eq!(
            e.cuerpos
                .iter()
                .find(|c| c.nombre == "Adrien")
                .unwrap()
                .radio,
            7.0
        );
        let saturn = e.cuerpos.iter().find(|c| c.nombre == "Saturn").unwrap();
        assert_eq!(saturn.anillos.len(), 1);
        let (ci, ce) = e.cinturon.unwrap();
        for c in &e.cuerpos[1..] {
            assert!(c.pos.len() + c.radio * 3.0 < ci - 5.0);
        }
        assert!(ce < RADIO_LIMITE - 20.0);
        assert_eq!(e.colisionadores_asteroides().len(), NUM_ASTEROIDES);
        assert!(e.agujero_negro.is_none() && e.estacion.is_none());
        for (p, r) in e.esferas().iter().chain(e.colisionadores_asteroides()) {
            assert!((INICIO - *p).len() > r + 2.0, "punto de llegada obstruido");
        }
    }
    #[test]
    fn vuelo_y_reaparicion_con_los_limites_del_nuevo_sistema() {
        use crate::nave::{Control, Mundo, Nave};
        let e = Escena::tau_ceti();
        let cuerpos = e.esferas();
        let mundo = Mundo {
            cuerpos: &cuerpos,
            asteroides: e.colisionadores_asteroides(),
            radio_limite: e.radio_limite,
            alto_limite: e.alto_limite,
            cinturon: e.cinturon,
            gravedad: None,
        };
        let mut nave = Nave::fisica();
        nave.establecer_reaparicion(INICIO);
        nave.teletransportar(INICIO, V3::ZERO);
        for _ in 0..120 {
            nave.actualizar(&Control::default(), 1.0 / 60.0, &mundo);
        }
        assert_eq!(nave.pos, INICIO);
        assert_eq!(nave.integridad, 100.0);
        for _ in 0..120 {
            nave.actualizar(
                &Control {
                    empuje: 1.0,
                    ..Default::default()
                },
                1.0 / 60.0,
                &mundo,
            );
        }
        assert!((nave.pos - INICIO).len() > 5.0);
        nave.reaparecer();
        assert_eq!(nave.pos, INICIO);
    }
}
