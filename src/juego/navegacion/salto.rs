use crate::progreso::{Galaxia, Progreso};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolicitudSalto { pub origen: Galaxia, pub destino: Galaxia }

#[derive(Default)]
pub struct Salto {
    solicitud: Option<SolicitudSalto>,
    carga: f32,
    // Tras cerrar el aviso, hay que alejarse del borde para activar otro salto.
    inhibido: bool,
}
impl Salto {
    pub fn solicitud(&self) -> Option<SolicitudSalto> { self.solicitud }
    pub fn carga(&self) -> f32 { self.carga }
    pub fn listo(&self) -> bool { self.solicitud.is_some() && self.carga >= 1.0 }
    pub fn observar(&mut self, progreso: &Progreso, distancia_borde: f32, en_vuelo: bool) -> bool {
        if !en_vuelo || self.solicitud.is_some() { return false; }
        if distancia_borde > 15.0 { self.inhibido = false; }
        if self.inhibido || !progreso.motor_activo() || !(0.0..=6.0).contains(&distancia_borde) { return false; }
        let Some(destino) = progreso.ruta().filter(|g| progreso.desbloqueada(*g) && *g != progreso.actual()) else { return false; };
        self.solicitud = Some(SolicitudSalto { origen: progreso.actual(), destino });
        self.carga = 0.0;
        true
    }
    pub fn actualizar(&mut self, dt: f32) {
        if self.solicitud.is_some() && dt.is_finite() && dt > 0.0 {
            self.carga = (self.carga + dt / 2.0).min(1.0);
        }
    }
    pub fn volver(&mut self) {
        self.solicitud = None;
        self.carga = 0.0;
        self.inhibido = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn limite_requiere_motor_ruta_y_nave_y_deja_solicitud_para_futura_escena() {
        let mut p = Progreso::default();
        let mut s = Salto::default();
        assert!(!s.observar(&p, 2.0, true));
        p.completar_estacion();
        p.completar_marte();
        assert!(!s.observar(&p, 2.0, false));
        assert!(!s.observar(&p, 20.0, true));
        assert!(s.observar(&p, 2.0, true));
        s.actualizar(2.1);
        assert!(s.listo());
        assert_eq!(s.solicitud().unwrap().destino, Galaxia::Gargantua);
        assert!(!s.observar(&p, 2.0, true));
        s.volver();
        assert!(!s.observar(&p, 2.0, true));
        s.observar(&p, 20.0, true);
        assert!(s.observar(&p, 2.0, true));
    }
}
