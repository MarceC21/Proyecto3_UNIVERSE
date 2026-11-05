// Progresión independiente de las escenas y de raylib. Completar Marte abre
// la primera ruta; las demás se habilitarán desde sus futuras misiones.
// No se guarda en disco: una nueva instancia empieza desde cero.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Galaxia { Solar, Gargantua, TauCeti, Eridani }

pub const GALAXIAS: [Galaxia; 4] = [Galaxia::Solar, Galaxia::Gargantua, Galaxia::TauCeti, Galaxia::Eridani];
impl Galaxia {
    pub fn indice(self) -> usize { self as usize }
    pub fn nombre(self) -> &'static str {
        match self { Self::Solar => "SISTEMA SOLAR", Self::Gargantua => "GARGANTUA",
            Self::TauCeti => "TAU CETI", Self::Eridani => "40 ERIDANI A" }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Progreso {
    completadas: [bool; 4],
    ruta: Option<Galaxia>,
}

impl Progreso {
    pub fn marte_completado(&self) -> bool { self.completadas[0] }
    pub fn mapa_activo(&self) -> bool { self.marte_completado() }
    pub fn motor_activo(&self) -> bool { self.marte_completado() }
    pub fn ruta(&self) -> Option<Galaxia> { self.ruta }
    pub fn desbloqueada(&self, g: Galaxia) -> bool {
        g == Galaxia::Solar || self.completadas[g.indice() - 1]
    }
    pub fn completada(&self, g: Galaxia) -> bool { self.completadas[g.indice()] }
    pub fn completar_marte(&mut self) -> bool { self.completar(Galaxia::Solar) }

    // Las próximas escenas llamarán aquí solo al terminar su misión.
    pub fn completar(&mut self, g: Galaxia) -> bool {
        if !self.desbloqueada(g) || self.completada(g) { return false; }
        self.completadas[g.indice()] = true;
        if let Some(siguiente) = GALAXIAS.get(g.indice() + 1) { self.ruta = Some(*siguiente); }
        true
    }
    pub fn seleccionar_ruta(&mut self, g: Galaxia) -> bool {
        if !self.mapa_activo() || g == Galaxia::Solar || !self.desbloqueada(g) { return false; }
        self.ruta = Some(g);
        true
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marte_desbloquea_solo_el_primer_destino_y_no_se_repite() {
        let mut p = Progreso::default();
        assert!(!p.mapa_activo());
        assert!(!p.seleccionar_ruta(Galaxia::Gargantua));
        assert!(p.completar_marte());
        assert!(p.mapa_activo() && p.motor_activo());
        assert_eq!(p.ruta(), Some(Galaxia::Gargantua));
        assert!(!p.desbloqueada(Galaxia::TauCeti));
        assert!(!p.desbloqueada(Galaxia::Eridani));
        assert!(!p.seleccionar_ruta(Galaxia::TauCeti));
        assert!(!p.completar_marte());
    }
    #[test]
    fn escenas_futuras_revelan_destinos_en_orden() {
        let mut p = Progreso::default();
        assert!(!p.completar(Galaxia::TauCeti));
        p.completar_marte();
        p.completar(Galaxia::Gargantua);
        assert!(p.desbloqueada(Galaxia::TauCeti));
        assert!(!p.desbloqueada(Galaxia::Eridani));
        p.completar(Galaxia::TauCeti);
        assert!(p.desbloqueada(Galaxia::Eridani));
    }
    #[test]
    fn victoria_real_de_turing_cierra_marte_y_habilita_salto_en_el_borde() {
        let mut m = crate::mision_marte::MisionMarte::default();
        m.observar(3.0, 0.0, true);
        m.actualizar(5.1);
        m.comenzar();
        for (i, regla) in crate::turing::REGLAS.iter().enumerate() {
            assert!(m.maquina.ejecutar(i, regla.accion));
            m.actualizar(1.1);
        }
        let mut p = Progreso::default();
        assert!(m.maquina.aceptada());
        p.completar_marte();
        m.marcar_completada();
        m.despegar();
        m.observar(30.0, 0.0, true);
        assert!(!m.observar(3.0, 0.0, true));
        let mut salto = crate::salto::Salto::default();
        let radio = crate::escena::RADIO_LIMITE;
        assert!(!salto.observar(&p, radio - 330.0, true));
        assert!(salto.observar(&p, radio - (radio - 1.6), true));
        salto.actualizar(2.0);
        assert!(salto.listo());
        assert_eq!(salto.solicitud().unwrap().destino, Galaxia::Gargantua);
    }
    #[test]
    fn una_nueva_partida_reinicia_marte_motor_mapa_y_ruta() {
        let mut anterior = Progreso::default();
        anterior.completar_marte();
        assert!(anterior.marte_completado() && anterior.motor_activo());
        assert!(anterior.mapa_activo());
        let nueva = Progreso::default();
        assert!(!nueva.marte_completado());
        assert!(!nueva.motor_activo());
        assert!(!nueva.mapa_activo());
        assert_eq!(nueva.ruta(), None);
        assert!(!nueva.desbloqueada(Galaxia::Gargantua));
        // Cerrar un mapa o cambiar de vista no crea una nueva partida.
        assert!(anterior.marte_completado());
    }

}
