// Skybox de Tau Ceti: nebulosas verde / violeta / azul sobre fondo azul noche.
// Se construye con los campos públicos de Cielo, sin tocar motor/cielo.rs.
use crate::cielo::{Cielo, Estrella};
use crate::math::*;
use crate::ruido::*;
use crate::textura::Textura;

fn nivel(x: f32, n: f32) -> f32 {
    let k = x.clamp(0.0, 0.999) * n;
    let f = k.floor();
    (f + smoothstep(0.40, 0.60, k - f)) / n
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn skybox_propio_con_estrellas_y_polvo() {
        let tau = Cielo::tau_ceti();
        for anterior in [Cielo::nuevo(), Cielo::gargantua()] {
            let diferencia: f32 = (0..8)
                .map(|i| {
                    (tau.nebulosa.muestrear(i as f32 / 8.0, 0.5, 0.0)
                        - anterior.nebulosa.muestrear(i as f32 / 8.0, 0.5, 0.0))
                    .len()
                })
                .sum();
            assert!(diferencia > 1.0);
        }
        assert_eq!(tau.estrellas.len(), 3200);
        assert_eq!(tau.polvo.len(), 320);
    }
}

impl Cielo {
    pub fn tau_ceti() -> Cielo {
        let eje = v3(-0.5, 0.6, 0.4).norm();
        let nebulosa = Textura::generar(1024, 512, &|d| {
            let g = d.y * 0.5 + 0.5;
            let mut c = mezcla(rgb(5.0, 8.0, 26.0), rgb(14.0, 10.0, 44.0), g);
            let m = (-(d.dot(eje)).powi(2) * 9.0).exp();
            let n1 = fbm(d * 1.8, 5, 901) + m * 0.5;
            let n2 = fbm(d * 2.4 + v3(5.0, 1.0, -3.0), 5, 902) + m * 0.45;
            let n3 = fbm(d * 3.4 + v3(-2.0, 7.0, 4.0), 4, 903) + m * 0.4;
            c += rgb(34.0, 150.0, 86.0) * (nivel(smoothstep(0.0, 0.6, n1), 4.0) * 0.55);
            c += rgb(110.0, 52.0, 170.0) * (nivel(smoothstep(0.02, 0.6, n2), 4.0) * 0.6);
            c += rgb(34.0, 92.0, 200.0) * (nivel(smoothstep(0.15, 0.75, n3), 3.0) * 0.45);
            c += rgb(120.0, 230.0, 255.0) * (m * m * 0.1);
            v3(c.x.min(1.0), c.y.min(1.0), c.z.min(1.0))
        });

        let mut rng = Rng::new(7777);
        let paleta = [
            rgb(255.0, 255.0, 255.0),
            rgb(190.0, 235.0, 255.0),
            rgb(200.0, 255.0, 225.0),
            rgb(225.0, 200.0, 255.0),
            rgb(255.0, 240.0, 200.0),
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
