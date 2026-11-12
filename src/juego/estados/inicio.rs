// Coordinador de arranque: Menú -> Intro -> Juego. Sin raylib.
// main.rs traduce teclas/ratón a `Orden`, llama a `actualizar(dt)` y dibuja según `pantalla()`.
// Cuando `pantalla()` es Juego, el bucle original sigue exactamente igual que hoy.
use crate::intro::Intro;
use crate::menu::{Accion, Menu};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pantalla {
    Menu,
    Intro,
    Juego,
    Salir,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orden {
    Arriba,
    Abajo,
    Aceptar,
    Atras,
}

pub struct Inicio {
    pantalla: Pantalla,
    pub menu: Menu,
    pub intro: Intro,
}

impl Inicio {
    pub fn nuevo() -> Inicio {
        Inicio {
            pantalla: Pantalla::Menu,
            menu: Menu::nuevo(),
            intro: Intro::nueva(),
        }
    }
    // Para --miller, --adrian, --estacion, --galaxia...: salta menú e intro.
    pub fn omitido() -> Inicio {
        Inicio {
            pantalla: Pantalla::Juego,
            ..Inicio::nuevo()
        }
    }
    pub fn pantalla(&self) -> Pantalla {
        self.pantalla
    }

    pub fn actualizar(&mut self, dt: f32) {
        match self.pantalla {
            Pantalla::Menu => self.menu.actualizar(dt),
            Pantalla::Intro => self.intro.actualizar(dt),
            _ => {}
        }
    }

    pub fn orden(&mut self, o: Orden) -> Pantalla {
        match (self.pantalla, o) {
            (Pantalla::Menu, Orden::Arriba) => self.menu.subir(),
            (Pantalla::Menu, Orden::Abajo) => self.menu.bajar(),
            (Pantalla::Menu, Orden::Atras) => self.menu.atras(),
            (Pantalla::Menu, Orden::Aceptar) => match self.menu.aceptar() {
                Accion::NuevoJuego => {
                    self.intro = Intro::nueva();
                    self.pantalla = Pantalla::Intro;
                }
                Accion::Salir => self.pantalla = Pantalla::Salir,
                Accion::Ninguna => {}
            },
            // En la intro, Aceptar/Atras omiten (o confirman en el título).
            (Pantalla::Intro, Orden::Aceptar | Orden::Atras) => self.pantalla = Pantalla::Juego,
            _ => {}
        }
        self.pantalla
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menu_intro_juego() {
        let mut i = Inicio::nuevo();
        assert_eq!(i.pantalla(), Pantalla::Menu);
        assert_eq!(i.orden(Orden::Aceptar), Pantalla::Intro);
        i.actualizar(20.0);
        assert!(i.intro.en_titulo());
        assert_eq!(i.pantalla(), Pantalla::Intro, "el título espera al jugador");
        assert_eq!(i.orden(Orden::Aceptar), Pantalla::Juego);
    }
    #[test]
    fn omitir_la_intro_a_mitad() {
        let mut i = Inicio::nuevo();
        i.orden(Orden::Aceptar);
        i.actualizar(3.0);
        assert_eq!(i.orden(Orden::Atras), Pantalla::Juego);
    }
    #[test]
    fn salir_y_argumentos_de_depuracion() {
        let mut i = Inicio::nuevo();
        i.orden(Orden::Arriba);
        assert_eq!(i.orden(Orden::Aceptar), Pantalla::Salir);
        assert_eq!(Inicio::omitido().pantalla(), Pantalla::Juego);
    }
    #[test]
    fn controles_no_cambian_de_pantalla() {
        let mut i = Inicio::nuevo();
        i.orden(Orden::Abajo);
        assert_eq!(i.orden(Orden::Aceptar), Pantalla::Menu);
        assert!(i.menu.controles_abiertos());
        assert_eq!(i.orden(Orden::Atras), Pantalla::Menu);
        assert!(!i.menu.controles_abiertos());
    }
}
