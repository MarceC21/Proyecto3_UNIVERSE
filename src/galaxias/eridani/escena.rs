// 40 Eridani: enana naranja (A), Erid con anillos azules luminosos, cuatro
// planetas más, cinturón de asteroides que rodea el sistema y, muy lejos,
// la binaria B (enana blanca) y C (enana roja).
use crate::escena::{Asteroide, Cuerpo, Escena};
use crate::generadores;
use crate::generadores_eridani as ge;
use crate::malla;
use crate::math::*;
use crate::progreso::Galaxia;
use crate::ruido::*;
use crate::shader::*;
use crate::textura::Textura;

pub const RADIO_LIMITE: f32 = 620.0;
pub const ALTO_LIMITE: f32 = 70.0;
pub const INICIO: V3 = v3(0.0, 8.0, 312.0); // dentro del cinturón, mirando al centro
pub const RADIO_ESTRELLA: f32 = 9.0; // el Sol mide 10
pub const CINTURON_INT: f32 = 340.0;
pub const CINTURON_EXT: f32 = 376.0;
const NUM_ASTEROIDES: usize = 1500;
const SEMILLA_CINTURON: u64 = 6262;

// ---------- Estrella naranja: mismo método que el Sol, otra rampa ----------
pub struct ShaderEstrellaK {
    pub tex: Textura,
}

impl Shader for ShaderEstrellaK {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let t = c.tiempo;
        let a = self.tex.muestrear(f.uv[0] + t * 0.010, f.uv[1], f.mip).x;
        let b = self
            .tex
            .muestrear(1.0 - f.uv[0] + t * 0.006 + 0.3, f.uv[1], f.mip)
            .x;
        let v = a * 0.6 + b * 0.4;
        let q = ((v * 4.0).floor() + smoothstep(0.35, 0.65, v * 4.0 - (v * 4.0).floor())) / 4.0;
        let col0 = mezcla(
            rgb(214.0, 70.0, 12.0),
            rgb(255.0, 150.0, 36.0),
            smoothstep(0.15, 0.55, q),
        );
        let mut col = mezcla(col0, rgb(255.0, 214.0, 120.0), smoothstep(0.55, 0.95, q));
        let ndv = f.normal.norm().dot((c.cam - f.pos).norm()).max(0.0);
        col = mezcla(col, rgb(230.0, 80.0, 16.0), (1.0 - ndv).powi(2) * 0.55);
        col = col * (1.0 + 0.05 * (t * 1.7).sin());
        [col.x, col.y, col.z, 1.0]
    }
}

// ---------- Resplandor naranja con rayos (sprite aditivo) ----------
pub struct ShaderBrilloK;

impl Shader for ShaderBrilloK {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let (x, y) = (f.uv[0] - 0.5, f.uv[1] - 0.5);
        let r = (x * x + y * y).sqrt() * 2.0;
        if r >= 1.0 {
            return [0.0; 4];
        }
        let ang = y.atan2(x);
        let aureola = (1.0 - r).powi(3);
        let rayos =
            (0.5 + 0.5 * (ang * 7.0 + c.tiempo * 0.2).cos()).powi(6) * (1.0 - r).powi(2) * 0.45;
        let col = mezcla(rgb(255.0, 110.0, 24.0), rgb(255.0, 200.0, 110.0), aureola);
        [col.x, col.y, col.z, (aureola * 0.85 + rayos) * f.tinte.x]
    }
}

// ---------- Compañeras B y C: esferas emisivas de un solo color ----------
pub struct ShaderEmisivo {
    pub centro: V3,
    pub borde: V3,
}

impl Shader for ShaderEmisivo {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let ndv = f.normal.norm().dot((c.cam - f.pos).norm()).max(0.0);
        let col = mezcla(self.centro, self.borde, (1.0 - ndv).powi(2))
            * (1.0 + 0.04 * (c.tiempo * 2.0).sin());
        [col.x, col.y, col.z, 1.0]
    }
}

// ---------- Anillos de Erid: azul-blanco luminoso y una banda cálida ----------
// banda 0 = interior cálida (dorada), 1 = azul brillante, 2 = exterior pálida.
pub struct ShaderAnillosErid {
    pub interior: f32,
    pub exterior: f32,
    pub banda: u8,
}

impl Shader for ShaderAnillosErid {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let r = f.uv[1];
        let hilos = (r * 30.0).sin() * 0.5 + 0.5;
        let fino = (r * 90.0 + hilos * 2.0).sin() * 0.5 + 0.5;
        let (c0, c1) = match self.banda {
            0 => (rgb(92.0, 62.0, 34.0), rgb(222.0, 172.0, 96.0)),
            1 => (rgb(30.0, 96.0, 225.0), rgb(190.0, 236.0, 255.0)),
            _ => (rgb(70.0, 140.0, 230.0), rgb(214.0, 242.0, 255.0)),
        };
        let mut col = mezcla(c0, c1, hilos * 0.6 + fino * 0.4) * (0.85 + 0.35 * fino);
        if self.banda != 0 {
            col += c1 * (0.18 * (1.0 - (r - 0.5).abs() * 2.0)); // núcleo luminoso
        }
        let hacia_sol = c.sol - f.pos;
        let l = hacia_sol.norm();
        // Sombra del planeta sobre el anillo.
        let radio_planeta =
            (f.pos - f.centro).len() / (self.interior + (self.exterior - self.interior) * r);
        let al_centro = f.centro - f.pos;
        let t = al_centro.dot(l);
        let separacion = (al_centro - l * t).len();
        let sombra = if t > 0.0 && t < hacia_sol.len() {
            1.0 - smoothstep(radio_planeta * 0.96, radio_planeta * 1.04, separacion)
        } else {
            0.0
        };
        let luz = (0.55 + 0.45 * f.normal.norm().dot(l).abs()) * (1.0 - sombra * 0.7);
        let borde = smoothstep(0.0, 0.08, r) * (1.0 - smoothstep(0.92, 1.0, r));
        let alfa = borde * (0.62 + hilos * 0.3);
        let col = col * luz;
        [col.x, col.y, col.z, alfa]
    }
}

impl Escena {
    pub fn eridani() -> Self {
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
        let s_estrella = agregar(Box::new(ShaderEstrellaK {
            tex: generadores::sol(),
        }));
        let s_erid = agregar(Box::new(ShaderPlaneta {
            tex: ge::erid(),
            atmosfera: rgb(90.0, 170.0, 255.0),
            fuerza_atm: 1.1,
        }));
        let s_templado = agregar(Box::new(ShaderPlaneta {
            tex: ge::templado(),
            atmosfera: rgb(150.0, 200.0, 240.0),
            fuerza_atm: 0.8,
        }));
        let s_a1 = agregar(Box::new(ShaderPlaneta {
            tex: ge::rojo_a1(),
            atmosfera: rgb(255.0, 130.0, 100.0),
            fuerza_atm: 0.3,
        }));
        let s_gigante = agregar(Box::new(ShaderPlaneta {
            tex: ge::gigante(),
            atmosfera: rgb(200.0, 235.0, 255.0),
            fuerza_atm: 0.5,
        }));
        let s_a2b = agregar(Box::new(ShaderPlaneta {
            tex: ge::rojo_a2b(),
            atmosfera: rgb(255.0, 150.0, 110.0),
            fuerza_atm: 0.3,
        }));
        let s_enana_b = agregar(Box::new(ShaderEmisivo {
            centro: rgb(245.0, 250.0, 255.0),
            borde: rgb(150.0, 200.0, 255.0),
        }));
        let s_enana_c = agregar(Box::new(ShaderEmisivo {
            centro: rgb(255.0, 96.0, 60.0),
            borde: rgb(200.0, 36.0, 24.0),
        }));
        // Tres bandas de anillos para Erid, con huecos entre ellas.
        let bandas_erid: [(f32, f32, u8); 3] = [(1.30, 1.62, 0), (1.72, 2.30, 1), (2.40, 2.62, 2)];
        let s_anillos: Vec<usize> = bandas_erid
            .iter()
            .map(|&(i, e, b)| {
                agregar(Box::new(ShaderAnillosErid {
                    interior: i,
                    exterior: e,
                    banda: b,
                }))
            })
            .collect();
        let s_asteroide = agregar(Box::new(ShaderAsteroide {
            tex: generadores::asteroide(),
        }));
        let s_orbita = agregar(Box::new(ShaderOrbita));
        let s_barrera = agregar(Box::new(ShaderBarrera));
        let s_baliza = agregar(Box::new(ShaderBaliza));
        let s_brillo = agregar(Box::new(ShaderBrilloK));

        // ---- Cuerpos ----
        let mut cuerpos = vec![Cuerpo {
            nombre: "40 Eridani A",
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
        // (nombre, distancia, radio, ángulo°, rotación, inclinación, shader)
        let planetas: [(&'static str, f32, f32, f32, f32, f32, usize); 5] = [
            ("Erid", 80.0, 7.0, 140.0, 0.50, 0.45, s_erid), // principal: tamaño de Miller
            ("40 Eri A 2", 125.0, 4.4, 330.0, 0.60, 0.25, s_templado),
            ("40 Eri A 1", 170.0, 2.6, 250.0, 0.80, 0.10, s_a1),
            ("40 Eri A 3", 225.0, 10.0, 55.0, 0.90, 0.06, s_gigante), // gigante gaseoso
            ("40 Eri A 2b", 280.0, 3.0, 205.0, 0.55, 0.30, s_a2b),
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
            let mut anillos = vec![];
            if nombre == "Erid" {
                for (k, &(int, ext, _)) in bandas_erid.iter().enumerate() {
                    let m = mallas.len();
                    mallas.push(malla::anillo(int, ext, 256, 1.0));
                    anillos.push((m, s_anillos[k]));
                }
            }
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
                orbita: Some((i, rgb(235.0, 205.0, 165.0))),
                anillos,
            });
        }
        // Binaria lejana: B (enana blanca) y C (enana roja), fuera del cinturón.
        let b = 8f32.to_radians();
        let pos_b = v3(540.0 * b.cos(), 0.0, 540.0 * b.sin());
        for (nombre, pos, radio, shader) in [
            ("40 Eridani B", pos_b, 4.5, s_enana_b),
            ("40 Eridani C", pos_b + v3(28.0, 0.0, 14.0), 6.0, s_enana_c),
        ] {
            cuerpos.push(Cuerpo {
                nombre,
                pos,
                radio,
                inclinacion: 0.0,
                vel_rot: 0.05,
                angulo: 0.0,
                shader,
                es_sol: false,
                nubes: None,
                angulo_nubes: 0.0,
                orbita: None,
                anillos: vec![],
            });
        }

        // ---- Cinturón de asteroides (reproducible), rodea todo el sistema ----
        let mut rng = Rng::new(SEMILLA_CINTURON);
        let mut asteroides = Vec::new();
        let paleta = [
            rgb(255.0, 235.0, 210.0),
            rgb(235.0, 190.0, 150.0),
            rgb(210.0, 210.0, 215.0),
            rgb(200.0, 160.0, 120.0),
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

        Self {
            galaxia: Galaxia::Eridani,
            radio_limite: RADIO_LIMITE,
            alto_limite: ALTO_LIMITE,
            cinturon: Some((CINTURON_INT, CINTURON_EXT)),
            agujero_negro: None,
            estacion: None,
            nave_cafe: None,
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
    fn estrella_erid_con_anillos_cinturon_y_binaria_lejana() {
        let e = Escena::eridani();
        assert_eq!(e.cuerpos.len(), 8);
        assert!(e.cuerpos[0].es_sol && e.cuerpos[0].radio < 10.0);
        let erid = e.cuerpos.iter().find(|c| c.nombre == "Erid").unwrap();
        assert_eq!(erid.radio, 7.0);
        assert_eq!(erid.anillos.len(), 3);
        let (ci, ce) = e.cinturon.unwrap();
        for c in &e.cuerpos[1..=5] {
            assert!(c.pos.len() + c.radio * 3.0 < ci - 5.0);
        }
        for c in &e.cuerpos[6..] {
            assert!(c.pos.len() > ce + 40.0);
            assert!(c.pos.len() + c.radio * 3.0 < RADIO_LIMITE - 20.0);
        }
        assert_eq!(e.colisionadores_asteroides().len(), NUM_ASTEROIDES);
        assert!(e.agujero_negro.is_none() && e.estacion.is_none() && e.nave_cafe.is_none());
        for (p, r) in e.esferas().iter().chain(e.colisionadores_asteroides()) {
            assert!((INICIO - *p).len() > r + 2.0, "llegada obstruida");
        }
        // Las compañeras lejanas son cuerpos navegables, no parte del skybox.
        for c in &e.cuerpos[6..] {
            assert!(e
                .esferas()
                .iter()
                .any(|(p, r)| *p == c.pos && *r == c.radio));
        }
        let mut cam = crate::camara::Camara::nueva(960.0, 600.0);
        cam.pos = INICIO;
        cam.mirar_a(V3::ZERO);
        let mut st = crate::dibujo::Stats::default();
        e.emitir(
            &cam,
            &crate::dibujo::Ajustes::default(),
            &mut vec![],
            &mut st,
        );
        assert!(st.objetos_enviados < st.objetos_total && st.tris_enviados < st.tris_sin_optimizar);
    }
    #[test]
    fn vuelo_y_reaparicion_con_los_limites_del_nuevo_sistema() {
        use crate::nave::{Control, Mundo, Nave};
        let e = Escena::eridani();
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
