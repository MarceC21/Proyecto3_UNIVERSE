//! Paseo autónomo del dúo: destinos variados, pausas, conversación y colisiones.
use crate::{
    jugador::{Mundo, Obst, Zona},
    math::*,
    paisaje_erid::{altura_suelo, limites_pasillo},
    ruido::Rng,
};
#[derive(Clone, Copy)]
pub struct Paseante {
    pub pos: V3,
    pub angulo: f32,
    pub paso: f32,
    pub moviendo: bool,
}
impl Paseante {
    fn nuevo(x: f32, z: f32, angulo: f32) -> Self {
        Self {
            pos: v3(x, altura_suelo(x, z), z),
            angulo,
            paso: 0.0,
            moviendo: false,
        }
    }
    fn mirar(&mut self, p: V3, dt: f32) {
        let d = p - self.pos;
        let ang = d.x.atan2(d.z);
        let dif = (ang - self.angulo + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        self.angulo += dif.clamp(-dt * 1.8, dt * 1.8);
    }
    fn mover(&mut self, objetivo: V3, velocidad: f32, dt: f32, mundo: &Mundo, jugador: V3) {
        self.moviendo = false;
        if (self.pos - jugador).x.hypot((self.pos - jugador).z) < 2.8 {
            self.mirar(jugador, dt);
            return;
        }
        let d = v3(objetivo.x - self.pos.x, 0.0, objetivo.z - self.pos.z);
        let dist = d.len();
        if dist < 0.12 {
            return;
        }
        let dir = d * (1.0 / dist);
        let tang = v3(-dir.z, 0.0, dir.x);
        let paso = (velocidad * dt).min(dist);
        for direccion in [dir, (dir * 0.35 + tang).norm(), (dir * 0.35 - tang).norm()] {
            let candidato = self.pos + direccion * paso;
            if mundo.libre(candidato.x, candidato.z, 1.02) {
                self.mirar(self.pos + direccion, dt);
                self.pos = v3(
                    candidato.x,
                    altura_suelo(candidato.x, candidato.z),
                    candidato.z,
                );
                self.paso += paso;
                self.moviendo = true;
                break;
            }
        }
    }
}
pub struct PaseoErid {
    pub rocky: Paseante,
    pub tars: Paseante,
    pub mundo: Mundo,
    objetivos: Option<[V3; 2]>,
    charla: Option<(usize, f32)>,
    ronda: usize,
    descanso: f32,
    atasco: f32,
    rng: Rng,
}
impl PaseoErid {
    pub fn nuevo() -> Self {
        let mut zonas = vec![];
        for i in -29..38 {
            let x = i as f32 * 2.0;
            zonas.push(Zona::Rect {
                x0: x,
                x1: x + 2.0,
                z0: limites_pasillo(x).0.max(limites_pasillo(x + 2.0).0) + 0.5,
                z1: limites_pasillo(x).1.min(limites_pasillo(x + 2.0).1) - 0.5,
            });
        }
        Self {
            rocky: Paseante::nuevo(-3.0, 18.0, 1.1),
            tars: Paseante::nuevo(-7.0, 21.0, 1.0),
            mundo: Mundo {
                zonas,
                obst: vec![Obst::caja(14.0, 31.0, 12.0, 9.0)],
                dinamicos: vec![],
            },
            objetivos: None,
            charla: None,
            ronda: 0,
            descanso: 1.5,
            atasco: 0.0,
            rng: Rng::new(4012),
        }
    }
    pub fn colisionadores(&self) -> Vec<Obst> {
        vec![
            Obst::caja(self.rocky.pos.x, self.rocky.pos.z, 2.0, 2.0),
            Obst::caja(self.tars.pos.x, self.tars.pos.z, 2.0, 2.0),
        ]
    }
    pub fn comentario(&self, jugador: V3) -> Option<&'static str> {
        if (self.rocky.pos - jugador).len() > 25.0 {
            return None;
        }
        let (i, _) = self.charla?;
        const LINEAS: [[&str; 3]; 3] = [
            [
                "ROCKY: Agua bonita. Vacaciones, pregunta?",
                "TARS: Confirmado. Nivel de trabajo: cero por ciento.",
                "ROCKY: Cero trabajo. Mucho amaze!",
            ],
            [
                "TARS: He analizado esta piedra. Es una piedra.",
                "ROCKY: Buena piedra. Rocky aprobar.",
                "TARS: La guardare como recuerdo. Humor al noventa por ciento.",
            ],
            [
                "ROCKY: Estrellas esperar. Hoy disfrutar hogar.",
                "TARS: Sugiero caminar junto al mar. Sin misiones pendientes.",
                "ROCKY: Buen plan. Buenos amigos.",
            ],
        ];
        Some(LINEAS[self.ronda % 3][i])
    }
    pub fn actualizar(&mut self, dt: f32, jugador: V3) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let n = (dt / 0.02).ceil().max(1.0) as usize;
        for _ in 0..n {
            self.paso(dt / n as f32, jugador);
        }
    }
    fn paso(&mut self, dt: f32, jugador: V3) {
        self.rocky.moviendo = false;
        self.tars.moviendo = false;
        if let Some((i, t)) = &mut self.charla {
            self.rocky.mirar(self.tars.pos, dt);
            self.tars.mirar(self.rocky.pos, dt);
            *t += dt;
            if *t >= 4.0 {
                *t -= 4.0;
                *i += 1;
                if *i == 3 {
                    self.charla = None;
                    self.ronda += 1;
                    self.descanso = self.rng.rango(2.0, 5.0);
                }
            }
            return;
        }
        if self.descanso > 0.0 {
            self.descanso -= dt;
            return;
        }
        if self.objetivos.is_none() {
            for _ in 0..32 {
                let x = self.rng.rango(-34.0, 30.0);
                let z = limites_pasillo(x).0 + self.rng.rango(6.0, 20.0);
                let destinos = [v3(x - 2.2, 0.0, z), v3(x + 2.2, 0.0, z + 0.7)];
                if destinos.iter().all(|p| {
                    self.mundo.libre(p.x, p.z, 1.2)
                        && (v3(p.x, jugador.y, p.z) - jugador).len() > 5.0
                }) && (destinos[0] - v3(self.rocky.pos.x, 0.0, self.rocky.pos.z)).len() > 5.0
                {
                    self.objetivos = Some(destinos);
                    break;
                }
            }
        }
        let Some(obj) = self.objetivos else {
            return;
        };
        let mut cols = self.colisionadores();
        self.mundo.dinamicos = vec![cols[1], Obst::caja(jugador.x, jugador.z, 0.7, 0.7)];
        self.rocky.mover(obj[0], 1.05, dt, &self.mundo, jugador);
        cols = self.colisionadores();
        self.mundo.dinamicos = vec![cols[0], Obst::caja(jugador.x, jugador.z, 0.7, 0.7)];
        self.tars.mover(obj[1], 0.85, dt, &self.mundo, jugador);
        self.mundo.dinamicos.clear();
        let cerca = |p: V3, o: V3| (v3(p.x, 0.0, p.z) - o).len() < 0.16;
        if cerca(self.rocky.pos, obj[0]) && cerca(self.tars.pos, obj[1]) {
            self.objetivos = None;
            self.charla = Some((0, 0.0));
            self.atasco = 0.0;
        } else if !self.rocky.moviendo && !self.tars.moviendo {
            self.atasco += dt;
            if self.atasco > 4.0 {
                self.objetivos = None;
                self.atasco = 0.0;
            }
        } else {
            self.atasco = 0.0;
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn pasean_conversan_y_no_atraviesan_nave_costa_o_al_otro() {
        let mut p = PaseoErid::nuevo();
        let inicio = (p.rocky.pos, p.tars.pos);
        let jugador = v3(65.0, 0.0, -60.0);
        let mut conversaron = false;
        for _ in 0..18000 {
            p.actualizar(1.0 / 60.0, jugador);
            for persona in [p.rocky, p.tars] {
                assert!(p.mundo.libre(persona.pos.x, persona.pos.z, 1.0));
                assert!(altura_suelo(persona.pos.x, persona.pos.z) > 0.0);
            }
            assert!((p.rocky.pos - p.tars.pos).len() > 2.0);
            conversaron |= p.charla.is_some();
        }
        assert!((p.rocky.pos - inicio.0).len() > 3.0 && (p.tars.pos - inicio.1).len() > 3.0);
        assert!(p.rocky.paso > 30.0 && p.tars.paso > 30.0);
        assert!(conversaron, "nunca se reunieron a conversar");
    }
    #[test]
    fn al_acercarse_el_jugador_se_detienen_y_hablan_en_su_posicion_actual() {
        let mut p = PaseoErid::nuevo();
        let inicio = p.rocky.pos;
        for _ in 0..300 {
            p.actualizar(1.0 / 60.0, inicio + v3(2.0, 0.0, 0.0));
        }
        assert_eq!(inicio, p.rocky.pos);
        assert!(!p.rocky.moviendo);
    }
}
