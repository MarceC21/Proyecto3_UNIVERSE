// 40 Eridani hereda el skybox anterior de Gargantua: petroleo, turquesa y crema.
use crate::cielo::{Cielo, Estrella};
use crate::math::*;
use crate::ruido::*;
use crate::textura::Textura;

impl Cielo {
    pub fn eridani() -> Cielo {
        // Antiguo skybox de Gargantua: azul petroleo, turquesa y luz crema.
        let nebulosa = Textura::generar(1024, 512, &|d| {
            // Paleta de Miller: azul petróleo, turquesa, gris y luz crema.
            let banda = (-(d.dot(v3(0.2, 0.9, -0.3).norm())).powi(2) * 5.0).exp();
            let a = smoothstep(-0.35, 0.55, fbm(d * 2.2, 5, 701));
            let b = smoothstep(-0.1, 0.65, fbm(d * 4.2 + v3(5.0, 1.0, 2.0), 4, 702));
            let luz = smoothstep(0.25, 0.9, d.dot(v3(-0.7, 0.15, 0.6).norm()));
            rgb(7.0, 18.0, 31.0)
                + rgb(37.0, 91.0, 114.0) * a
                + rgb(53.0, 105.0, 109.0) * (b * banda)
                + rgb(174.0, 153.0, 112.0) * (luz * banda * a * 0.65)
        });
        let mut rng = Rng::new(7026);
        let paleta = [
            rgb(235.0, 241.0, 230.0),
            rgb(238.0, 215.0, 166.0),
            rgb(154.0, 217.0, 227.0),
            rgb(187.0, 218.0, 210.0),
            rgb(240.0, 231.0, 199.0),
        ];
        let mut estrellas = Vec::new();
        for i in 0..3200 {
            let tipo = if i < 10 {
                3
            } else if i < 70 {
                2
            } else if i < 600 {
                1
            } else {
                0
            };
            estrellas.push(Estrella {
                dir: rng.direccion(),
                brillo: match tipo {
                    0 => rng.rango(0.35, 0.8),
                    1 => rng.rango(0.6, 1.0),
                    _ => 1.0,
                },
                color: paleta[(rng.siguiente() % paleta.len() as u64) as usize],
                tipo,
                fase: rng.rango(0.0, 6.28),
                vel: rng.rango(0.8, 3.0),
            });
        }
        let polvo = (0..320).map(|_| v3(rng.f(), rng.f(), rng.f())).collect();
        Cielo {
            nebulosa,
            estrellas,
            polvo,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cielo_del_hogar_distinto_a_tau_ceti() {
        let e = Cielo::eridani();
        let tau = Cielo::tau_ceti();
        let diferencia: f32 = (0..16)
            .map(|i| {
                (e.nebulosa.muestrear(i as f32 / 16.0, 0.5, 0.0)
                    - tau.nebulosa.muestrear(i as f32 / 16.0, 0.5, 0.0))
                .len()
            })
            .sum();
        assert!(diferencia > 1.0);
        assert_eq!(e.estrellas.len(), 3200);
        assert_eq!(e.polvo.len(), 320);
    }
}
