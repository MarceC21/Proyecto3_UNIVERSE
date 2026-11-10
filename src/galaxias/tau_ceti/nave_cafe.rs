// Modelo de encuentro estático. Comparte cargador, materiales y emisor con la estación.
use crate::{escena::Cuerpo, estacion::Estacion, math::*, nave::ESCALA_NAVE, obj::Modelo};

fn dimension(modelo: &Modelo) -> f32 {
    (0..3)
        .map(|eje| {
            let min = modelo
                .vertices
                .iter()
                .map(|p| p[eje])
                .fold(f32::INFINITY, f32::min);
            let max = modelo
                .vertices
                .iter()
                .map(|p| p[eje])
                .fold(f32::NEG_INFINITY, f32::max);
            max - min
        })
        .fold(0.0, f32::max)
}

pub fn crear(cuerpos: &[Cuerpo]) -> Estacion {
    let modelo = Modelo::desde_textos(
        include_str!("../../../assets/galaxias/tau_ceti/nave_cafe/nave_cafe.obj"),
        include_str!("../../../assets/galaxias/tau_ceti/nave_cafe/nave_cafe.mtl"),
    )
    .expect("nave_cafe.obj valido");
    let jugador = Modelo::desde_textos(
        include_str!("../../../assets/compartidos/nave/nave.obj"),
        include_str!("../../../assets/compartidos/nave/nave.mtl"),
    )
    .expect("modelo de referencia valido");
    let escala = 6.0 * dimension(&jugador) * ESCALA_NAVE / dimension(&modelo);
    let (centro, _) = modelo.centro_y_radio();
    let centro = v3(centro[0], centro[1], centro[2]);
    let radio = modelo
        .vertices
        .iter()
        .map(|p| (v3(p[0], p[1], p[2]) - centro).len())
        .fold(0.0, f32::max)
        * escala;
    // Baricentro del triángulo formado por los tres planetas de referencia.
    let mut pos = V3::ZERO;
    for nombre in ["Adrien", "Gigante", "Oceanus"] {
        pos += cuerpos
            .iter()
            .find(|c| c.nombre == nombre)
            .expect("planeta de referencia")
            .pos
            * (1.0 / 3.0);
    }
    Estacion::desde_modelo(modelo, radio, pos, Rot::rot_y(-0.45).mul(&Rot::rot_z(0.06)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modelo_sextuple_entre_planetas_con_colision_y_visita_antes_de_adrien() {
        use crate::{
            escena::Escena,
            guia,
            nave::{Control, Mundo, Nave},
            progreso::{Galaxia, Progreso},
        };
        let e = Escena::tau_ceti();
        let n = e.nave_cafe.as_ref().unwrap();
        let mut centro = V3::ZERO;
        for nombre in ["Adrien", "Gigante", "Oceanus"] {
            let c = e.cuerpos.iter().find(|c| c.nombre == nombre).unwrap();
            centro += c.pos * (1.0 / 3.0);
            assert!((n.pos - c.pos).len() > n.radio + c.radio + 10.0);
        }
        assert!((n.pos - centro).len() < 0.001);
        let original = Modelo::desde_textos(
            include_str!("../../../assets/compartidos/nave/nave.obj"),
            include_str!("../../../assets/compartidos/nave/nave.mtl"),
        )
        .unwrap();
        let medida = (0..3)
            .map(|eje| {
                let coordenada = |p: V3| [p.x, p.y, p.z][eje];
                let max = n
                    .partes
                    .iter()
                    .flat_map(|(m, _)| m.pos.iter())
                    .map(|p| coordenada(*p))
                    .fold(f32::NEG_INFINITY, f32::max);
                let min = n
                    .partes
                    .iter()
                    .flat_map(|(m, _)| m.pos.iter())
                    .map(|p| coordenada(*p))
                    .fold(f32::INFINITY, f32::min);
                max - min
            })
            .fold(0.0, f32::max);
        assert!((medida - 6.0 * dimension(&original) * ESCALA_NAVE).abs() < 0.001);
        let cuerpos = e.esferas();
        assert!(cuerpos.iter().any(|(p, r)| *p == n.pos && *r == n.radio));
        let mundo = Mundo {
            cuerpos: &cuerpos,
            asteroides: e.colisionadores_asteroides(),
            radio_limite: e.radio_limite,
            alto_limite: e.alto_limite,
            cinturon: e.cinturon,
            gravedad: None,
        };
        let mut jugador = Nave::fisica();
        jugador.teletransportar(n.pos + v3(0.0, 0.0, n.radio + 5.0), n.pos);
        for _ in 0..240 {
            jugador.actualizar(
                &Control {
                    empuje: 1.0,
                    ..Default::default()
                },
                1.0 / 60.0,
                &mundo,
            );
            assert!((jugador.pos - n.pos).len() >= n.radio + 0.8);
        }
        let mut p = Progreso::default();
        p.completar_estacion();
        p.completar_marte();
        p.viajar(Galaxia::Gargantua);
        p.completar(Galaxia::Gargantua);
        p.viajar(Galaxia::TauCeti);
        assert_eq!(guia::objetivo(&e, &p, n.pos).unwrap().nombre, "Blip-A");
        assert!(!p.nave_cafe_visitada()); // la proximidad exterior no completa el encuentro
        assert!(p.completar_encuentro_blip());
        assert_eq!(guia::objetivo(&e, &p, n.pos).unwrap().nombre, "ADRIEN");
        p.viajar(Galaxia::Gargantua);
        p.viajar(Galaxia::TauCeti);
        assert!(p.nave_cafe_visitada());
        assert!(!Progreso::default().nave_cafe_visitada());
    }
}
