// Introducción mínima en 3 planos: Tierra apagándose -> cabina con el personaje -> título.
// Solo tiempo y números (cámaras, fundidos, animación). Dibujar es tarea de la interfaz.
// Independiente de los FPS: solo acumula dt.
use crate::dialogos_intro as texto;
use crate::math::*;
use crate::menu::orbital;
use std::f32::consts::PI;

pub const FIN_TIERRA: f32 = 6.0;
pub const FIN_CABINA: f32 = 14.0;
pub const DURACION: f32 = 18.0; // el título se queda hasta que el jugador pulse ENTER

// Personaje dentro de la cabina (ajusta a gusto). Mide ~1.09 * 1.5 = 1.64.
pub const ESCALA_PERSONAJE: f32 = 1.5;
pub const POS_PERSONAJE: V3 = v3(0.0, 0.0, -11.6); // entre las dos sillas, frente a la ventana
pub const YAW_PERSONAJE: f32 = PI; // el modelo mira a +Z; así queda de espaldas a la cámara

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plano {
    Tierra,
    Cabina,
    Titulo,
}

#[derive(Default)]
pub struct Intro {
    t: f32,
}

impl Intro {
    pub fn nueva() -> Intro {
        Intro::default()
    }
    pub fn t(&self) -> f32 {
        self.t
    }

    pub fn actualizar(&mut self, dt: f32) {
        if dt.is_finite() && dt > 0.0 {
            self.t = (self.t + dt).min(DURACION);
        }
    }

    pub fn plano(&self) -> Plano {
        if self.t < FIN_TIERRA {
            Plano::Tierra
        } else if self.t < FIN_CABINA {
            Plano::Cabina
        } else {
            Plano::Titulo
        }
    }
    pub fn en_titulo(&self) -> bool {
        self.plano() == Plano::Titulo
    }

    // 0..1 dentro del plano actual.
    pub fn progreso(&self) -> f32 {
        let (a, b) = match self.plano() {
            Plano::Tierra => (0.0, FIN_TIERRA),
            Plano::Cabina => (FIN_TIERRA, FIN_CABINA),
            Plano::Titulo => (FIN_CABINA, DURACION),
        };
        clamp01((self.t - a) / (b - a))
    }

    // Opacidad del negro por encima de todo: entrada al inicio y corte entre planos.
    // Vale 1.0 justo en cada corte, así el cambio de escena nunca se ve.
    pub fn fundido(&self) -> f32 {
        let corte = |c: f32| 1.0 - smoothstep(0.0, 0.5, (self.t - c).abs());
        clamp01(
            (1.0 - smoothstep(0.0, 0.8, self.t))
                .max(corte(FIN_TIERRA))
                .max(corte(FIN_CABINA)),
        )
    }

    // Subtítulo actual y su opacidad.
    pub fn subtitulo(&self) -> Option<(&'static texto::Subtitulo, f32)> {
        texto::activo(self.t)
    }

    // ---- Plano 1: la Tierra, con el Sol apagándose ----
    // Alfa (0..0.65) de un billboard negro delante del Sol para oscurecerlo.
    pub fn oscuridad_sol(&self) -> f32 {
        0.65 * smoothstep(0.5, 5.5, self.t.min(FIN_TIERRA))
    }
    // (eye, center): órbita lenta que se va alejando.
    pub fn camara_tierra(&self, tierra: V3, radio: f32) -> (V3, V3) {
        let p = self.progreso();
        (
            orbital(tierra, radio * (4.5 + 1.5 * p), 0.4 + self.t * 0.10, 0.3),
            tierra,
        )
    }

    // ---- Plano 2: cabina ----
    // Cámara fija con un dolly muy lento hacia el personaje.
    pub fn camara_cabina(&self) -> (V3, V3) {
        (
            v3(0.0, 1.7, -5.5 - 0.7 * self.progreso()),
            v3(0.0, 1.3, -12.0),
        )
    }
    // Respiración: desplazamiento vertical (en metros del modelo) de todo el cuerpo.
    pub fn respiracion(&self) -> f32 {
        0.01 * (self.t * 2.0).sin()
    }
    // Giro de la cabeza (rad, sobre Y) hacia la cámara al final del plano.
    // Se aplica solo a las caras con y > 0.62 (cuello, cráneo, gorro, lentes).
    pub fn giro_cabeza(&self) -> f32 {
        if self.plano() == Plano::Cabina {
            0.9 * smoothstep(0.55, 0.9, self.progreso())
        } else {
            0.0
        }
    }
    // Balanceo suave de los brazos (rad), caras con |x| > 0.18.
    pub fn balanceo_brazos(&self) -> f32 {
        0.04 * (self.t * 1.6).sin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn simular(fps: u32, seg: f32) -> Intro {
        let mut i = Intro::nueva();
        for _ in 0..(fps as f32 * seg) as u32 {
            i.actualizar(1.0 / fps as f32);
        }
        i
    }
    #[test]
    fn planos_en_orden_y_el_titulo_se_queda() {
        let mut i = Intro::nueva();
        assert_eq!(i.plano(), Plano::Tierra);
        i.actualizar(7.0);
        assert_eq!(i.plano(), Plano::Cabina);
        i.actualizar(7.0);
        assert!(i.en_titulo());
        i.actualizar(1000.0);
        assert_eq!(i.t(), DURACION);
        assert!(i.en_titulo());
    }
    #[test]
    fn consistente_entre_fps() {
        let (a, b) = (simular(30, 9.0), simular(144, 9.0));
        assert!((a.t() - b.t()).abs() < 0.05);
        assert!((a.giro_cabeza() - b.giro_cabeza()).abs() < 0.02);
    }
    #[test]
    fn dt_invalido_se_ignora() {
        let mut i = Intro::nueva();
        i.actualizar(f32::NAN);
        i.actualizar(-1.0);
        assert_eq!(i.t(), 0.0);
    }
    #[test]
    fn fundido_negro_en_los_cortes_y_rangos_validos() {
        for corte in [0.0, FIN_TIERRA, FIN_CABINA] {
            let mut i = Intro::nueva();
            i.actualizar(corte);
            assert!(i.fundido() > 0.99, "corte {corte}");
        }
        let mut i = Intro::nueva();
        i.actualizar(3.0);
        assert_eq!(i.fundido(), 0.0);
        for k in 0..180 {
            let mut j = Intro::nueva();
            j.actualizar(k as f32 * 0.1);
            assert!((0.0..=1.0).contains(&j.fundido()) && (0.0..=1.0).contains(&j.progreso()));
        }
    }
    #[test]
    fn el_sol_se_oscurece_solo_en_el_primer_plano() {
        let (a, b) = (simular(60, 0.5), simular(60, 5.8));
        assert!(a.oscuridad_sol() < 0.01 && b.oscuridad_sol() > 0.6);
    }
}
