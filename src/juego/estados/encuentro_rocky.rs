//! Encuentro de Blip-A. El progreso solo se concede al acabar la charla de partida.
use crate::{jugador::Mundo, math::*};

pub const RADIO_ROCKY: f32 = 0.48;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Etapa {
    Esperando,
    Inicial(usize),
    Guiando,
    Partida(usize),
    Abordo,
}
#[derive(Clone, Copy)]
pub struct Linea {
    pub nombre: &'static str,
    pub texto: &'static str,
}
const INICIAL: [Linea; 5] = [
    Linea {
        nombre: "ROCKY",
        texto: "¡Humano! ¡Humano aquí! ¡Amaze! Amaze! Amaze!",
    },
    Linea {
        nombre: "JUGADOR",
        texto: "Espera... ¿puedes entenderme?",
    },
    Linea {
        nombre: "ROCKY",
        texto: "Sí, sí. Tú venir. Rocky esperar. Mucho tiempo.",
    },
    Linea {
        nombre: "JUGADOR",
        texto: "¿Por qué me estabas esperando?",
    },
    Linea {
        nombre: "ROCKY",
        texto: "Estrellas enfermas. Astrofagos comer luz. Tú ayudar Rocky, ¿pregunta?",
    },
];
const PARTIDA: [Linea; 4] = [
    Linea {
        nombre: "ROCKY",
        texto: "Adrian tiene taumoebas. Pequeñas. Comen astrofagos.",
    },
    Linea {
        nombre: "ROCKY",
        texto: "Tú y Rocky salvar estrellas. ¡Amaze! Amaze! Amaze!",
    },
    Linea {
        nombre: "ROCKY",
        texto: "Tú pilotar. Rocky estabilizar nave. Tú recoger muestras. Evitar cosas peligrosas.",
    },
    Linea {
        nombre: "ROCKY",
        texto: "¡Buen plan! Tú listo, ¿pregunta?",
    },
];
const RECORRIDO: [Linea; 3] = [
    Linea {
        nombre: "ROCKY",
        texto: "Tú nave rápida. Rocky nave inteligente. Buen equipo.",
    },
    Linea {
        nombre: "JUGADOR",
        texto: "¿Y a dónde se supone que vamos?",
    },
    Linea {
        nombre: "ROCKY",
        texto: "Adrian. Vida pequeña. Muy importante. Tú ver.",
    },
];
pub struct EncuentroRocky {
    pub etapa: Etapa,
    pub pos: V3,
    pub en_nave: bool,
    pub apertura: f32,
    pub angulo: f32,
    pub paso: f32,
    pub moviendo: bool,
    pub esperando: bool,
    destino: usize,
    charla: f32,
    recorrido: f32,
}
impl Default for EncuentroRocky {
    fn default() -> Self {
        Self {
            etapa: Etapa::Esperando,
            pos: v3(0.0, 0.14, -28.0),
            en_nave: false,
            apertura: 0.0,
            angulo: 0.0,
            paso: 0.0,
            moviendo: false,
            esperando: false,
            destino: 0,
            charla: 0.0,
            recorrido: 0.0,
        }
    }
}
impl EncuentroRocky {
    pub fn bloquea(&self) -> bool {
        matches!(self.etapa, Etapa::Inicial(_) | Etapa::Partida(_))
    }
    pub fn completo(&self) -> bool {
        self.etapa == Etapa::Abordo
    }
    pub fn dialogo(&self) -> Option<Linea> {
        match self.etapa {
            Etapa::Inicial(i) => Some(INICIAL[i]),
            Etapa::Partida(i) => Some(PARTIDA[i]),
            _ => None,
        }
    }
    pub fn comentario(&self) -> Option<Linea> {
        if self.etapa != Etapa::Guiando {
            return None;
        }
        match self.charla {
            t if (0.0..5.0).contains(&t) => Some(RECORRIDO[0]),
            t if (5.0..9.0).contains(&t) => Some(RECORRIDO[1]),
            t if (9.0..15.0).contains(&t) => Some(RECORRIDO[2]),
            _ => None,
        }
    }
    pub fn avanzar(&mut self) -> Option<&'static str> {
        match self.etapa {
            Etapa::Inicial(i) if i + 1 < INICIAL.len() => self.etapa = Etapa::Inicial(i + 1),
            Etapa::Inicial(_) => {
                self.etapa = Etapa::Guiando;
                return Some(
                    "NUEVA MISIÓN: SIGUE A ROCKY. Deja libre un lado del pasillo para que pase.",
                );
            }
            Etapa::Partida(i) if i + 1 < PARTIDA.len() => self.etapa = Etapa::Partida(i + 1),
            Etapa::Partida(_) => {
                self.etapa = Etapa::Abordo;
                return Some("OBJETIVO ACTUALIZADO: VIAJA A ADRIAN");
            }
            _ => {}
        }
        None
    }
    pub fn en_muelle(&self) -> bool {
        self.etapa == Etapa::Guiando && !self.en_nave && self.destino >= 2
    }
    pub fn abordar(&mut self, fin_tunel: f32) {
        if self.en_muelle() {
            self.en_nave = true;
            self.pos = v3(0.0, 0.0, fin_tunel - 2.8);
            self.destino = 0;
            self.esperando = false;
        }
    }
    pub fn actualizar(&mut self, dt: f32, mundo: &Mundo, jx: f32, jz: f32, en_nave: bool) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.moviendo = false;
        if en_nave != self.en_nave {
            return;
        }
        let distancia = ((self.pos.x - jx).powi(2) + (self.pos.z - jz).powi(2)).sqrt();
        if self.etapa == Etapa::Esperando && !en_nave && jz <= -22.5 && distancia < 6.5 {
            self.etapa = Etapa::Inicial(0);
        }
        if self.bloquea() || self.completo() {
            return;
        }
        if self.etapa != Etapa::Guiando {
            return;
        }
        self.apertura = (self.apertura + dt / 1.6).min(1.0);
        if self.apertura < 1.0 {
            return;
        }
        if distancia > 6.0 {
            self.esperando = true;
        }
        if distancia < 4.5 {
            self.esperando = false;
        }
        if self.esperando {
            return;
        }
        let ruta: &[(f32, f32)] = if en_nave {
            &[(0.0, 10.0), (0.0, 6.0), (3.0, 6.0), (4.0, 4.0)]
        } else {
            &[(-0.40, -21.5), (-0.40, -1.0)]
        };
        if self.destino >= ruta.len() {
            if en_nave && distancia < 2.8 {
                self.etapa = Etapa::Partida(0);
            }
            return;
        }
        let (x, z) = ruta[self.destino];
        let delta = v3(x - self.pos.x, 0.0, z - self.pos.z);
        let d = delta.len();
        if d < 0.04 {
            self.destino += 1;
            return;
        }
        let dir = delta * (1.0 / d);
        let giro = (dir.x.atan2(dir.z) - self.angulo + std::f32::consts::PI)
            .rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        self.angulo += giro.clamp(-dt * 2.2, dt * 2.2);
        // Mismos subpasos y consulta de colisión del jugador/TARS; ni teletransporte ni motor nuevo.
        let avance = (1.15 * dt).min(d);
        let n = (avance / 0.08).ceil().max(1.0) as usize;
        for _ in 0..n {
            let p = self.pos + dir * (avance / n as f32);
            if ((p.x - jx).powi(2) + (p.z - jz).powi(2)).sqrt()
                < RADIO_ROCKY + crate::jugador::RADIO + 0.06
                || !mundo.libre(p.x, p.z, RADIO_ROCKY)
            {
                break;
            }
            self.pos = p;
            self.paso += avance / n as f32;
            self.moviendo = true;
        }
        if self.moviendo {
            self.recorrido += avance;
            if self.recorrido > 3.0 {
                self.charla += dt;
            }
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::jugador::{Obst, Zona};
    fn mundo() -> Mundo {
        Mundo {
            zonas: vec![Zona::Rect {
                x0: -1.24,
                x1: 1.24,
                z0: -31.0,
                z1: -0.1,
            }],
            ..Default::default()
        }
    }
    #[test]
    fn espera_al_jugador_y_respeta_obstaculos_incluso_con_dt_grande() {
        let mut r = EncuentroRocky::default();
        r.etapa = Etapa::Guiando;
        r.apertura = 1.0;
        r.pos = v3(-0.62, 0.14, -20.0);
        r.destino = 1;
        let mut m = mundo();
        let inicial = r.pos;
        r.actualizar(2.0, &m, 0.5, -30.0, false);
        assert!(r.esperando && r.pos == inicial);
        r.actualizar(0.2, &m, 0.5, -22.0, false);
        assert!(!r.esperando && r.pos.z > inicial.z);
        m.obst.push(Obst::caja(0.0, -18.0, 2.5, 0.1));
        r.actualizar(5.0, &m, 0.5, -22.0, false);
        assert!(r.pos.z < -18.5 && m.libre(r.pos.x, r.pos.z, RADIO_ROCKY));
    }
    #[test]
    fn ninguna_conversacion_se_repite_ni_completa_antes_de_partir() {
        let m = mundo();
        let mut r = EncuentroRocky::default();
        r.actualizar(0.1, &m, 0.0, -22.6, false);
        assert_eq!(r.etapa, Etapa::Inicial(0));
        let posicion = r.pos;
        r.actualizar(10.0, &m, 0.0, -22.6, false);
        assert_eq!(r.pos, posicion);
        for _ in 0..5 {
            r.avanzar();
        }
        assert!(!r.completo());
        r.etapa = Etapa::Partida(0);
        for _ in 0..4 {
            r.avanzar();
        }
        assert!(r.completo());
        r.actualizar(0.1, &m, 0.0, -22.6, false);
        r.avanzar();
        assert!(r.completo() && r.dialogo().is_none());
    }
}
