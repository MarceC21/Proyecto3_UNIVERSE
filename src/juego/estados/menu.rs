// Menú principal: lógica independiente de raylib y del renderizado.
// El fondo es la Tierra vista con una cámara orbital (eye = center + esféricas).
use crate::math::*;
use std::f32::consts::TAU;

pub const OPCIONES: [&str; 3] = ["NUEVO JUEGO", "CONTROLES", "SALIR"];

pub const CONTROLES: [(&str, &str); 15] = [
    ("W / S", "Acelerar / frenar"),
    ("A / D", "Moverse de lado"),
    ("Espacio / Ctrl", "Subir / bajar"),
    ("Ratón / flechas", "Girar la nave"),
    ("Q / E", "Alabeo"),
    ("Shift", "Turbo"),
    ("Clic derecho", "Orbitar la cámara"),
    ("F", "Modo local (rodear un cuerpo)"),
    ("V", "Cámara libre"),
    ("M", "Mapa interestelar"),
    ("H", "Entrar a la cabina"),
    ("E", "Interactuar / atracar"),
    ("R", "Reaparecer"),
    ("TAB", "Liberar el cursor"),
    ("F1 - F3", "Calidad de render"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accion {
    Ninguna,
    NuevoJuego,
    Salir,
}

// Posición de la cámara orbital: sobre una esfera de `radio` alrededor de `centro`.
// Es la misma conversión esféricas -> cartesianas de la materia (yaw, pitch, radio).
pub fn orbital(centro: V3, radio: f32, yaw: f32, pitch: f32) -> V3 {
    let pitch = pitch.clamp(-1.5, 1.5);
    centro
        + v3(
            radio * pitch.cos() * yaw.sin(),
            radio * pitch.sin(),
            radio * pitch.cos() * yaw.cos(),
        )
}

#[derive(Default)]
pub struct Menu {
    seleccion: usize,
    controles: bool,
    tiempo: f32,
}

impl Menu {
    pub fn nuevo() -> Menu {
        Menu::default()
    }
    pub fn seleccion(&self) -> usize {
        self.seleccion
    }
    pub fn controles_abiertos(&self) -> bool {
        self.controles
    }
    pub fn tiempo(&self) -> f32 {
        self.tiempo
    }
    // Pulso 0..1 para el borde de la opción seleccionada.
    pub fn pulso(&self) -> f32 {
        0.5 + 0.5 * (self.tiempo * 3.0).sin()
    }

    pub fn actualizar(&mut self, dt: f32) {
        if dt.is_finite() && dt > 0.0 {
            self.tiempo = (self.tiempo + dt) % (TAU * 1000.0);
        }
    }
    pub fn subir(&mut self) {
        if !self.controles {
            self.seleccion = (self.seleccion + OPCIONES.len() - 1) % OPCIONES.len();
        }
    }
    pub fn bajar(&mut self) {
        if !self.controles {
            self.seleccion = (self.seleccion + 1) % OPCIONES.len();
        }
    }
    // Para el ratón: pasar el índice de la opción bajo el cursor.
    pub fn elegir(&mut self, i: usize) {
        if !self.controles && i < OPCIONES.len() {
            self.seleccion = i;
        }
    }
    pub fn aceptar(&mut self) -> Accion {
        if self.controles {
            self.controles = false;
            return Accion::Ninguna;
        }
        match self.seleccion {
            0 => Accion::NuevoJuego,
            1 => {
                self.controles = true;
                Accion::Ninguna
            }
            _ => Accion::Salir,
        }
    }
    pub fn atras(&mut self) {
        self.controles = false;
    }

    // (eye, center) del fondo: órbita lenta alrededor de la Tierra.
    pub fn camara(&self, centro: V3, radio: f32) -> (V3, V3) {
        (
            orbital(centro, radio * 4.2, self.tiempo * 0.08 + 0.6, 0.22),
            centro,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seleccion_con_wrap() {
        let mut m = Menu::nuevo();
        m.subir();
        assert_eq!(m.seleccion(), OPCIONES.len() - 1);
        m.bajar();
        assert_eq!(m.seleccion(), 0);
        m.elegir(99);
        assert_eq!(m.seleccion(), 0);
    }
    #[test]
    fn acciones_y_panel_de_controles() {
        let mut m = Menu::nuevo();
        assert_eq!(m.aceptar(), Accion::NuevoJuego);
        m.bajar();
        assert_eq!(m.aceptar(), Accion::Ninguna);
        assert!(m.controles_abiertos());
        m.bajar();
        assert_eq!(m.seleccion(), 1, "con el panel abierto no se navega");
        assert_eq!(m.aceptar(), Accion::Ninguna);
        assert!(!m.controles_abiertos());
        m.bajar();
        assert_eq!(m.aceptar(), Accion::Salir);
    }
    #[test]
    fn orbita_mantiene_el_radio() {
        let c = v3(1.0, 2.0, 3.0);
        for k in 0..20 {
            let p = orbital(c, 17.0, k as f32 * 0.7, 0.22);
            assert!(((p - c).len() - 17.0).abs() < 1e-3);
        }
    }
}
