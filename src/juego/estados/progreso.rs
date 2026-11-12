// Progresión independiente de las escenas y de raylib. Completar Marte abre
// la primera ruta; las demás se habilitarán desde sus futuras misiones.
// No se guarda en disco: una nueva instancia empieza desde cero.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Galaxia { Solar, Gargantua, TauCeti, Eridani }

impl Default for Galaxia { fn default() -> Self { Self::Solar } }

pub const GALAXIAS: [Galaxia; 4] = [Galaxia::Solar, Galaxia::Gargantua, Galaxia::TauCeti, Galaxia::Eridani];
impl Galaxia {
    pub fn indice(self) -> usize { self as usize }
    pub fn nombre(self) -> &'static str {
        match self { Self::Solar => "SISTEMA SOLAR", Self::Gargantua => "GARGANTUA",
            Self::TauCeti => "TAU CETI", Self::Eridani => "40 ERIDANI A" }
    }
    pub fn implementada(self) -> bool { matches!(self, Self::Solar | Self::Gargantua | Self::TauCeti | Self::Eridani) }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Progreso {
    estacion: bool,
    datos_enviados: bool,
    nave_cafe_visitada: bool,
    completadas: [bool; 4],
    ruta: Option<Galaxia>,
    actual: Galaxia,
}

impl Progreso {
    pub fn datos_enviados(&self) -> bool { self.datos_enviados }
    pub fn transmitir_datos(&mut self) -> bool {
        if !self.completada(Galaxia::TauCeti) || self.datos_enviados { return false; }
        self.datos_enviados = true;
        self.ruta = Some(Galaxia::Eridani);
        true
    }
    pub fn nave_cafe_visitada(&self) -> bool { self.nave_cafe_visitada }
    pub fn adrien_disponible(&self) -> bool { self.nave_cafe_visitada }
    // La aventura lo concede al terminar el recorrido y la charla de partida de Rocky.
    pub fn completar_encuentro_blip(&mut self) -> bool {
        if self.actual != Galaxia::TauCeti || self.nave_cafe_visitada { return false; }
        self.nave_cafe_visitada = true;
        true
    }
    pub fn estacion_completada(&self) -> bool { self.estacion }
    pub fn completar_estacion(&mut self) { self.estacion = true; }
    pub fn marte_disponible(&self) -> bool { self.estacion }
    pub fn marte_completado(&self) -> bool { self.completadas[0] }
    pub fn mapa_activo(&self) -> bool { self.marte_completado() }
    pub fn motor_activo(&self) -> bool { self.marte_completado() }
    pub fn ruta(&self) -> Option<Galaxia> { self.ruta }
    pub fn actual(&self) -> Galaxia { self.actual }
    pub fn viajar(&mut self, destino: Galaxia) -> bool {
        if !self.motor_activo() || destino == self.actual || !destino.implementada() || !self.desbloqueada(destino) { return false; }
        self.actual = destino;
        // Preferir el siguiente sistema abierto; si falta completar la misión
        // actual, ofrecer el anterior. Viajar nunca completa una misión.
        self.ruta = GALAXIAS.get(destino.indice()+1).copied()
            .filter(|g| g.implementada() && self.desbloqueada(*g))
            .or_else(|| destino.indice().checked_sub(1).map(|i| GALAXIAS[i]));
        true
    }
    pub fn desbloqueada(&self, g: Galaxia) -> bool {
        g == Galaxia::Solar || (self.completadas[g.indice() - 1] && (g != Galaxia::Eridani || self.datos_enviados))
    }
    pub fn completada(&self, g: Galaxia) -> bool { self.completadas[g.indice()] }
    pub fn completar_marte(&mut self) -> bool { self.completar(Galaxia::Solar) }

    // Las próximas escenas llamarán aquí solo al terminar su misión.
    pub fn completar(&mut self, g: Galaxia) -> bool {
        if g == Galaxia::Solar && !self.marte_disponible() { return false; }
        if !self.desbloqueada(g) || self.completada(g) { return false; }
        self.completadas[g.indice()] = true;
        if let Some(siguiente) = GALAXIAS.get(g.indice() + 1).filter(|s| self.desbloqueada(**s)) { self.ruta = Some(*siguiente); }
        true
    }
    pub fn seleccionar_ruta(&mut self, g: Galaxia) -> bool {
        if !self.mapa_activo() || g == self.actual || !self.desbloqueada(g) { return false; }
        self.ruta = Some(g);
        true
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn taumoeba_habilita_salto_a_eridani_y_retorno_sin_completar_el_hogar() {
        let mut p = Progreso::default();
        p.completar_estacion(); p.completar_marte();
        p.completar(Galaxia::Gargantua); p.viajar(Galaxia::TauCeti);
        assert!(!p.viajar(Galaxia::Eridani));
        p.completar_encuentro_blip();
        assert!(!p.desbloqueada(Galaxia::Eridani), "Rocky no completa Taumoeba");
        assert!(p.completar(Galaxia::TauCeti));
        assert!(!p.desbloqueada(Galaxia::Eridani));
        assert!(p.transmitir_datos());
        assert!(!p.transmitir_datos());
        assert_eq!(p.ruta(),Some(Galaxia::Eridani));
        let mut salto = crate::salto::Salto::default();
        assert!(salto.observar(&p,2.0,true)); salto.actualizar(2.1);
        assert!(salto.listo());
        assert_eq!(salto.solicitud().unwrap().destino,Galaxia::Eridani);
        assert!(p.viajar(Galaxia::Eridani));
        assert!(!p.completada(Galaxia::Eridani));
        assert_eq!(p.ruta(),Some(Galaxia::TauCeti));
        let objetivo = crate::guia::objetivo(&crate::escena::Escena::eridani(),&p,crate::eridani::INICIO).unwrap();
        assert_eq!(objetivo.nombre,"ERID");
        assert!(p.viajar(Galaxia::TauCeti));
        assert_eq!(p.ruta(),Some(Galaxia::Eridani));
        assert!(p.adrien_disponible());
        assert!(!Progreso::default().desbloqueada(Galaxia::Eridani));
    }
    #[test]
    fn miller_habilita_salto_a_tau_ceti_y_permite_regresar_sin_revelar_eridani() {
        let mut p = Progreso::default();
        p.completar_estacion();
        p.completar_marte();
        p.viajar(Galaxia::Gargantua);
        assert!(!p.viajar(Galaxia::TauCeti));
        assert!(p.completar(Galaxia::Gargantua));
        assert_eq!(p.ruta(),Some(Galaxia::TauCeti));
        let mut salto = crate::salto::Salto::default();
        assert!(salto.observar(&p,2.0,true));
        salto.actualizar(2.1);
        assert!(salto.listo());
        assert_eq!(salto.solicitud().unwrap().destino,Galaxia::TauCeti);
        assert!(p.viajar(Galaxia::TauCeti));
        assert_eq!(p.ruta(),Some(Galaxia::Gargantua));
        assert!(!p.completada(Galaxia::TauCeti));
        assert!(!p.desbloqueada(Galaxia::Eridani));
        assert!(p.viajar(Galaxia::Gargantua));
        assert_eq!(p.ruta(),Some(Galaxia::TauCeti));
        assert!(p.seleccionar_ruta(Galaxia::Solar));
        assert!(p.viajar(Galaxia::Solar));
        assert!(p.estacion_completada() && p.marte_completado());
    }
    #[test]
    fn estacion_abre_marte_sin_adelantar_motor_y_el_reinicio_borra_todo() {
        let mut p = Progreso::default();
        assert!(!p.marte_disponible());
        assert!(!p.completar_marte());
        p.completar_estacion();
        assert!(p.marte_disponible());
        assert!(!p.motor_activo() && !p.mapa_activo());
        assert!(p.completar_marte());
        assert!(p.motor_activo());
        assert!(!Progreso::default().estacion_completada());
    }
    #[test]
    fn viajar_a_gargantua_y_volver_no_completa_miller_ni_revela_tau_ceti() {
        let mut p = Progreso::default();
        assert!(!p.viajar(Galaxia::Gargantua));
        p.completar_estacion();
        p.completar_marte();
        assert!(p.viajar(Galaxia::Gargantua));
        assert_eq!(p.actual(), Galaxia::Gargantua);
        assert_eq!(p.ruta(), Some(Galaxia::Solar));
        assert!(!p.desbloqueada(Galaxia::TauCeti));
        assert!(!p.completada(Galaxia::Gargantua));
        assert!(!p.viajar(Galaxia::TauCeti));
        let mut salto = crate::salto::Salto::default();
        assert!(salto.observar(&p, 2.0, true));
        assert_eq!(salto.solicitud().unwrap().origen, Galaxia::Gargantua);
        assert_eq!(salto.solicitud().unwrap().destino, Galaxia::Solar);
        assert!(p.viajar(Galaxia::Solar));
        assert!(p.marte_completado());
        assert_eq!(p.ruta(), Some(Galaxia::Gargantua));
    }
    #[test]
    fn marte_desbloquea_solo_el_primer_destino_y_no_se_repite() {
        let mut p = Progreso::default();
        assert!(!p.mapa_activo());
        assert!(!p.seleccionar_ruta(Galaxia::Gargantua));
        p.completar_estacion();
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
        p.completar_estacion();
        p.completar_marte();
        p.completar(Galaxia::Gargantua);
        assert!(p.desbloqueada(Galaxia::TauCeti));
        assert!(!p.desbloqueada(Galaxia::Eridani));
        p.completar(Galaxia::TauCeti);
        assert!(!p.desbloqueada(Galaxia::Eridani));
        assert!(p.transmitir_datos());
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
        p.completar_estacion();
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
        anterior.completar_estacion();
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
