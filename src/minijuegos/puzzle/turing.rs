// Lógica independiente de raylib: una transición solo se aplica cuando termina
// su animación. Los errores no alteran la cinta, el cabezal ni el estado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Simbolo { Cero, Uno, Blanco }

impl Simbolo {
    pub fn texto(self) -> &'static str {
        match self { Self::Cero => "0", Self::Uno => "1", Self::Blanco => "_" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direccion { Izquierda, Derecha }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Accion {
    pub estado: usize,
    pub escribe: Simbolo,
    pub mueve: Direccion,
}

#[derive(Clone, Copy, Debug)]
pub struct Regla {
    pub desde: usize,
    pub lee: Simbolo,
    pub accion: Accion,
}

use Direccion::{Derecha, Izquierda};
use Simbolo::{Blanco, Cero, Uno};

pub const REGLAS: [Regla; 4] = [
    Regla { desde: 0, lee: Cero, accion: Accion { estado: 1, escribe: Uno, mueve: Derecha } },
    Regla { desde: 1, lee: Uno, accion: Accion { estado: 2, escribe: Cero, mueve: Derecha } },
    Regla { desde: 2, lee: Blanco, accion: Accion { estado: 3, escribe: Uno, mueve: Izquierda } },
    Regla { desde: 3, lee: Cero, accion: Accion { estado: 4, escribe: Cero, mueve: Derecha } },
];
pub const DURACION_TRANSICION: f32 = 1.0;
pub fn estado_texto(q: usize) -> &'static str { ["q0", "q1", "q2", "q3", "qf"][q] }

pub struct MaquinaTuring {
    cinta: [Simbolo; 5],
    cabezal: usize,
    estado: usize,
    pasos: usize,
    pendiente: Option<Accion>,
    animacion: f32,
    pub error: bool,
}

impl Default for MaquinaTuring {
    fn default() -> Self {
        Self { cinta: [Blanco, Cero, Uno, Blanco, Blanco], cabezal: 1, estado: 0,
            pasos: 0, pendiente: None, animacion: 0.0, error: false }
    }
}

impl MaquinaTuring {
    pub fn cinta(&self) -> &[Simbolo; 5] { &self.cinta }
    pub fn cabezal(&self) -> usize { self.cabezal }
    pub fn estado(&self) -> usize { self.estado }
    pub fn pasos(&self) -> usize { self.pasos }
    pub fn aceptada(&self) -> bool { self.estado == 4 }
    pub fn animando(&self) -> bool { self.pendiente.is_some() }
    pub fn progreso(&self) -> f32 { self.animacion / DURACION_TRANSICION }
    pub fn pendiente(&self) -> Option<Accion> { self.pendiente }

    pub fn ejecutar(&mut self, regla: usize, accion: Accion) -> bool {
        if self.animando() || self.aceptada() { return false; }
        let valida = REGLAS.get(regla).is_some_and(|r|
            r.desde == self.estado && r.lee == self.cinta[self.cabezal] && r.accion == accion);
        if !valida {
            self.error = true;
            return false;
        }
        self.error = false;
        self.pendiente = Some(accion);
        self.animacion = 0.0;
        true
    }

    pub fn actualizar(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 { return; }
        let Some(accion) = self.pendiente else { return; };
        self.animacion = (self.animacion + dt).min(DURACION_TRANSICION);
        if self.animacion < DURACION_TRANSICION { return; }
        self.cinta[self.cabezal] = accion.escribe;
        self.cabezal = match accion.mueve { Izquierda => self.cabezal - 1, Derecha => self.cabezal + 1 };
        self.estado = accion.estado;
        self.pasos += 1;
        self.pendiente = None;

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cuatro_transiciones_y_resultado_exacto() {
        let mut m = MaquinaTuring::default();
        for (i, regla) in REGLAS.iter().enumerate() {
            assert!(m.ejecutar(i, regla.accion));
            assert_eq!(m.pasos(), i);
            m.actualizar(1.0);
        }
        assert!(m.aceptada());
        assert_eq!(m.cinta(), &[Blanco, Uno, Cero, Uno, Blanco]);
        assert_eq!(m.cabezal(), 3);
        assert!(!m.ejecutar(3, REGLAS[3].accion));
    }
    #[test]
    fn errores_y_doble_ejecucion_no_modifican_la_maquina() {
        let mut m = MaquinaTuring::default();
        let original = *m.cinta();
        for regla in [1, 2, 3, 99] { assert!(!m.ejecutar(regla, REGLAS[0].accion)); }
        for accion in [
            Accion { estado: 0, ..REGLAS[0].accion },
            Accion { escribe: Blanco, ..REGLAS[0].accion },
            Accion { mueve: Izquierda, ..REGLAS[0].accion },
        ] { assert!(!m.ejecutar(0, accion)); }
        assert_eq!(m.cinta(), &original);
        assert_eq!((m.estado(), m.cabezal(), m.pasos()), (0, 1, 0));
        assert!(m.ejecutar(0, REGLAS[0].accion));
        assert!(!m.ejecutar(0, REGLAS[0].accion));
        m.actualizar(0.4);
        assert_eq!(m.cinta(), &original);
        m.actualizar(0.61);
        assert_eq!((m.estado(), m.cabezal(), m.pasos()), (1, 2, 1));
    }
    #[test]
    fn animacion_independiente_de_fps() {
        for fps in [30, 60, 144] {
            let mut m = MaquinaTuring::default();
            m.ejecutar(0, REGLAS[0].accion);
            for _ in 0..fps + 1 { m.actualizar(1.0 / fps as f32); }
            assert_eq!((m.estado(), m.cabezal(), m.pasos()), (1, 2, 1));
        }
    }
}
