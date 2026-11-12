//! Final y paseo por Erid. Sin raylib ni cambios en la nave del espacio.
use crate::{
    jugador::{EntradaJ, Jugador, Mundo, Obst, Zona},
    math::*,
    paisaje_erid::{altura_suelo, limites_pasillo},
};
pub const ROCKY: (f32, f32) = (-3.0, 18.0);
pub const TARS: (f32, f32) = (-7.0, 21.0);
pub const NAVE: (f32, f32) = (14.0, 31.0);
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fase {
    Descenso,
    Dialogo(usize),
    Cierre,
    Libre,
}

#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn descenso_dialogos_final_y_visitas_posteriores() {
        assert!(!FinalErid::aterrizaje_disponible(3.0, 0.0, true, false));
        assert!(!FinalErid::aterrizaje_disponible(3.0, 7.0, true, true));
        assert!(!FinalErid::aterrizaje_disponible(3.0, 0.0, false, true));
        assert!(FinalErid::aterrizaje_disponible(3.0, 0.0, true, true));
        let mut m = FinalErid::nuevo(false);
        m.continuar();
        assert_eq!(m.fase, Fase::Descenso);
        m.actualizar(5.1, &EntradaJ::default());
        assert_eq!(m.fase, Fase::Dialogo(0));
        let p = (m.jugador.x, m.jugador.z);
        m.actualizar(
            1.0,
            &EntradaJ {
                adelante: 1.0,
                ..Default::default()
            },
        );
        assert_eq!(p, (m.jugador.x, m.jugador.z));
        for _ in 0..DIALOGOS.len() - 1 {
            m.continuar();
            assert!(!m.completado());
        }
        m.continuar();
        assert!(m.completado());
        assert_eq!(m.fase, Fase::Cierre);
        m.continuar();
        assert_eq!(m.fase, Fase::Libre);
        let mut nueva = FinalErid::nuevo(true);
        nueva.actualizar(5.1, &EntradaJ::default());
        assert_eq!(nueva.fase, Fase::Libre);
    }
    #[test]
    fn entrar_al_mar_y_regresar_sin_hundir_la_camara() {
        let mut m = FinalErid::nuevo(true);
        m.actualizar(5.1, &EntradaJ::default());
        m.jugador.colocar(25.0, 20.0, 0.0);
        for _ in 0..1500 {
            m.actualizar(
                1.0 / 60.0,
                &EntradaJ {
                    adelante: 1.0,
                    ..Default::default()
                },
            );
        }
        assert!(m.nadando() && m.jugador.z < -10.0);
        assert!((0.80..1.0).contains(&m.jugador.altura_ojos));
        m.jugador.yaw = std::f32::consts::PI;
        for _ in 0..1800 {
            m.actualizar(
                1.0 / 60.0,
                &EntradaJ {
                    adelante: 1.0,
                    ..Default::default()
                },
            );
        }
        assert!(!m.nadando() && altura_suelo(m.jugador.x, m.jugador.z) > 0.0);
    }
    #[test]
    fn caminar_y_nadar_respetan_rocas_acantilado_y_personajes() {
        let mut m = FinalErid::nuevo(true);
        m.actualizar(5.1, &EntradaJ::default());
        for yaw in [
            0.0,
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
            -std::f32::consts::FRAC_PI_2,
        ] {
            m.jugador.yaw = yaw;
            for _ in 0..1000 {
                m.actualizar(
                    1.0 / 60.0,
                    &EntradaJ {
                        adelante: 1.0,
                        correr: true,
                        ..Default::default()
                    },
                );
                let j = &m.jugador;
                let (_, alto) = limites_pasillo(j.x);
                assert!(j.x >= -58.0 && j.x <= 76.0 && j.z >= -85.0 && j.z <= alto);
                assert!(m.mundo.libre(j.x, j.z, crate::jugador::RADIO));
                assert!(j.altura_ojos > 0.80);
            }
        }
    }
}
pub const DIALOGOS: [(&str,&str);4] = [
    ("ROCKY", "Erid. Hogar. Tú ayudar Rocky. Tú salvar estrellas. ¡Amaze!"),
    ("TARS", "La estación recibió los datos de las taumoebas y el protocolo contra los astrofagos. La Tierra tiene una oportunidad."),
    ("JUGADOR", "Entonces lo logramos. Ahora podemos descansar un momento."),
    ("ROCKY", "Amigos. Buen equipo. Tú quedar aquí o explorar estrellas. Siempre hogar."),
];
pub struct FinalErid {
    pub fase: Fase,
    pub descenso: f32,
    pub tiempo: f32,
    pub jugador: Jugador,
    pub mundo: Mundo,
    pub mensaje: Option<(&'static str, f32)>,
    pub paseo: crate::paseo_erid::PaseoErid,
    completado: bool,
}
impl FinalErid {
    pub fn nuevo(completado: bool) -> Self {
        let mut zonas = vec![];
        // Costa y zona de baño; se conserva el límite terrestre antes del acantilado.
        for i in -29..38 {
            let x = i as f32 * 2.0;
            let bajo = -85.0;
            let alto = limites_pasillo(x).1.min(limites_pasillo(x + 2.0).1) - 0.5;
            zonas.push(Zona::Rect {
                x0: x,
                x1: x + 2.0,
                z0: bajo,
                z1: alto,
            });
        }
        let obst = vec![
            // Pilares del arco: se puede nadar por el hueco, no atravesar la roca.
            Obst::caja(-66.5, -14.0, 8.0, 12.0),
            Obst::caja(-45.5, -14.0, 8.0, 12.0),
            Obst::caja(NAVE.0, NAVE.1, 12.0, 9.0),
        ];
        let mut j = Jugador::nuevo(6.0, 20.0, -1.1);
        j.altura_ojos = altura_suelo(j.x, j.z) + 1.65;
        Self {
            fase: Fase::Descenso,
            descenso: 0.0,
            tiempo: 0.0,
            jugador: j,
            mundo: Mundo {
                zonas,
                obst,
                dinamicos: vec![],
            },
            mensaje: None,
            paseo: crate::paseo_erid::PaseoErid::nuevo(),
            completado,
        }
    }
    pub fn aterrizaje_disponible(altura: f32, velocidad: f32, en_vuelo: bool, datos: bool) -> bool {
        en_vuelo && datos && (0.0..=6.5).contains(&altura) && (0.0..=6.0).contains(&velocidad)
    }
    pub fn completado(&self) -> bool {
        self.completado
    }
    pub fn actualizar(&mut self, dt: f32, entrada: &EntradaJ) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.tiempo += dt;
        if let Some((_, t)) = &mut self.mensaje {
            *t -= dt;
            if *t <= 0.0 {
                self.mensaje = None;
            }
        }
        match self.fase {
            Fase::Descenso => {
                self.descenso = (self.descenso + dt / 5.0).min(1.0);
                if self.descenso >= 1.0 {
                    self.fase = if self.completado {
                        Fase::Libre
                    } else {
                        Fase::Dialogo(0)
                    };
                }
            }
            Fase::Libre => {
                let n = (dt / 0.02).ceil().max(1.0) as usize;
                for _ in 0..n {
                    let paso = dt / n as f32;
                    self.paseo
                        .actualizar(paso, v3(self.jugador.x, 0.0, self.jugador.z));
                    self.mundo.dinamicos = self.paseo.colisionadores();
                    let mojado = altura_suelo(self.jugador.x, self.jugador.z) < 0.0;
                    let e = EntradaJ {
                        adelante: entrada.adelante * if mojado { 0.60 } else { 1.0 },
                        lateral: entrada.lateral * if mojado { 0.60 } else { 1.0 },
                        giro_x: entrada.giro_x / n as f32,
                        giro_y: entrada.giro_y / n as f32,
                        correr: entrada.correr && !mojado,
                    };
                    self.jugador.actualizar(&e, paso, &self.mundo);
                }
                let suelo = altura_suelo(self.jugador.x, self.jugador.z);
                let flotacion = 1.0 - smoothstep(-1.2, -0.5, suelo);
                self.jugador.altura_ojos =
                    (suelo + 1.65).max(0.85) + 0.035 * (self.tiempo * 2.0).sin() * flotacion;
            }
            _ => {}
        }
    }
    pub fn nadando(&self) -> bool {
        altura_suelo(self.jugador.x, self.jugador.z) < -0.8
    }
    pub fn comentario(&self) -> Option<&'static str> {
        self.mensaje.map(|m| m.0).or_else(|| {
            self.paseo
                .comentario(v3(self.jugador.x, 0.0, self.jugador.z))
        })
    }
    pub fn continuar(&mut self) {
        match self.fase {
            Fase::Dialogo(i) if i + 1 < DIALOGOS.len() => self.fase = Fase::Dialogo(i + 1),
            Fase::Dialogo(_) => {
                self.completado = true;
                self.fase = Fase::Cierre;
            }
            Fase::Cierre => self.fase = Fase::Libre,
            Fase::Libre => {
                let p = v3(self.jugador.x, 0.0, self.jugador.z);
                if (p - v3(self.paseo.rocky.pos.x, 0.0, self.paseo.rocky.pos.z)).len() < 3.3 {
                    self.mensaje = Some(("ROCKY: Buen lugar. Buenos amigos. ¡Amaze!", 5.0));
                } else if (p - v3(self.paseo.tars.pos.x, 0.0, self.paseo.tars.pos.z)).len() < 3.3 {
                    self.mensaje = Some((
                        "TARS: Misión cumplida. El universo sigue abierto, capitán.",
                        5.0,
                    ));
                }
            }
            _ => {}
        }
    }
}
