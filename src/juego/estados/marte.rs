use crate::turing::MaquinaTuring;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaseMarte { Espacio, Descenso, Instrucciones, Terminal }

pub struct MisionMarte {
    pub maquina: MaquinaTuring,
    pub fase: FaseMarte,
    pub descenso: f32,
    armada: bool,
}

impl Default for MisionMarte {
    fn default() -> Self {
        Self { maquina: MaquinaTuring::default(), fase: FaseMarte::Espacio, descenso: 0.0, armada: true }
    }
}

impl MisionMarte {
    // Altura desde la superficie de Marte; jamás se usa la cámara libre para aterrizar.
    pub fn observar(&mut self, altura: f32, velocidad: f32, en_vuelo: bool) -> bool {
        if self.fase != FaseMarte::Espacio || !en_vuelo { return false; }
        if altura > 10.0 { self.armada = true; }
        if self.armada && altura <= 6.5 && altura >= 0.0 && velocidad <= 6.0 {
            self.armada = false;
            self.descenso = 0.0;
            self.fase = FaseMarte::Descenso;
            return true;
        }
        false
    }
    pub fn actualizar(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 { return; }
        if self.fase == FaseMarte::Descenso {
            self.descenso = (self.descenso + dt / 5.0).min(1.0);
            if self.descenso >= 1.0 { self.fase = FaseMarte::Instrucciones; }
        } else if self.fase == FaseMarte::Terminal { self.maquina.actualizar(dt); }
    }
    pub fn comenzar(&mut self) {
        if self.fase == FaseMarte::Instrucciones { self.fase = FaseMarte::Terminal; }
    }
    pub fn despegar(&mut self) {
        // Una selección ya validada se completa al salir, sin dejar la
        // animación a medias ni obligar al jugador a esperar para salir.
        self.maquina.actualizar(crate::turing::DURACION_TRANSICION);
        self.fase = FaseMarte::Espacio;
        self.armada = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn llegada_requiere_nave_cercana_lenta_y_no_reabre_al_salir() {
        let mut m = MisionMarte::default();
        assert!(!m.observar(3.0, 0.0, false));
        assert!(!m.observar(20.0, 0.0, true));
        assert!(!m.observar(3.0, 12.0, true));
        assert!(m.observar(6.0, 2.0, true));
        m.actualizar(5.1);
        assert_eq!(m.fase, FaseMarte::Instrucciones);
        m.comenzar();
        assert_eq!(m.fase, FaseMarte::Terminal);
        m.despegar();
        assert!(!m.observar(3.0, 0.0, true));
        m.observar(12.0, 0.0, true);
        assert!(m.observar(3.0, 0.0, true));
    }

    #[test]
    fn instrucciones_esperan_confirmacion_y_salir_completa_la_accion_validada() {
        let mut m = MisionMarte::default();
        m.observar(3.0, 0.0, true);
        m.actualizar(5.1);
        m.actualizar(20.0);
        assert_eq!(m.fase, FaseMarte::Instrucciones);
        assert_eq!(m.maquina.pasos(), 0);
        m.comenzar();
        assert!(m.maquina.ejecutar(0, crate::turing::REGLAS[0].accion));
        m.actualizar(0.2);
        m.despegar();
        assert_eq!(m.fase, FaseMarte::Espacio);
        assert_eq!(m.maquina.pasos(), 1);
        assert!(!m.maquina.animando());
    }

    #[test]
    fn atajo_de_marte_inicia_mision_y_el_progreso_sobrevive_a_otras_visitas() {
        let escena = crate::escena::Escena::nueva();
        let marte = escena.cuerpos.iter().find(|c| c.nombre == "Marte").unwrap();
        let vista = escena.vistas().into_iter().find(|v| v.0 == "marte").unwrap();
        let altura = (vista.1 - marte.pos).len() - marte.radio;
        let mut m = MisionMarte::default();
        assert!(m.observar(altura, 0.0, true));
        m.actualizar(5.1);
        assert_eq!(m.fase, FaseMarte::Instrucciones);
        m.comenzar();
        for (i, regla) in crate::turing::REGLAS.iter().enumerate() {
            assert!(m.maquina.ejecutar(i, regla.accion));
            m.actualizar(1.1);
        }
        m.despegar();
        m.observar(12.0, 0.0, true);
        m.observar(altura, 0.0, true);
        m.actualizar(5.1);
        assert!(m.maquina.aceptada());
        assert_eq!(m.maquina.pasos(), 4);
    }
}
