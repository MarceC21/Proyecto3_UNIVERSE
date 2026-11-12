// Aproximación de juego, en unidades del mundo: atracción radial con caída
// inversa al cuadrado, alcance finito y aceleración acotada
use crate::math::*;

#[derive(Clone, Copy, Debug)]
pub struct CampoGravitatorio {
    pub centro: V3,
    pub radio_horizonte: f32,
    pub radio_influencia: f32,
    pub mu: f32,
    pub aceleracion_max: f32,
}

impl CampoGravitatorio {
    pub fn aceleracion(&self, pos: V3) -> V3 {
        let delta = self.centro - pos;
        let distancia = delta.len();
        if distancia < 1e-5 || distancia >= self.radio_influencia { return V3::ZERO; }
        let caida = 1.0 - smoothstep(self.radio_horizonte, self.radio_influencia, distancia);
        let fuerza = (self.mu / distancia.max(self.radio_horizonte).powi(2) * caida).min(self.aceleracion_max);
        delta * (fuerza / distancia)
    }
    pub fn intensidad(&self, pos: V3) -> f32 {
        (self.aceleracion(pos).len() / self.aceleracion_max).clamp(0.0, 1.0)
    }
    pub fn captura(&self, pos: V3) -> bool { (pos - self.centro).len() <= self.radio_horizonte }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fuerza_apunta_al_centro_crece_y_no_explota() {
        let g = CampoGravitatorio { centro: V3::ZERO, radio_horizonte: 26.0,
            radio_influencia: 150.0, mu: 18000.0, aceleracion_max: 22.0 };
        assert_eq!(g.aceleracion(v3(200.0, 0.0, 0.0)), V3::ZERO);
        assert!(g.aceleracion(v3(80.0, 0.0, 0.0)).x < 0.0);
        assert!(g.aceleracion(v3(32.0, 0.0, 0.0)).len() > g.aceleracion(v3(80.0, 0.0, 0.0)).len());
        assert!(g.aceleracion(v3(0.001, 0.0, 0.0)).len() <= 22.001);
        assert_eq!(g.aceleracion(V3::ZERO), V3::ZERO);
    }
}
