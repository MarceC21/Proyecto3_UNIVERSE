// Skybox de 40 Eridani: nebulosas verde, turquesa y violeta sobre azul noche.
use crate::cielo::{Cielo, Estrella};
use crate::math::*;
use crate::ruido::*;
use crate::textura::Textura;

fn nivel(x: f32, n: f32) -> f32 {
    let k = x.clamp(0.0, 0.999) * n;
    let f = k.floor();
    (f + smoothstep(0.40, 0.60, k - f)) / n
}

impl Cielo {
    pub fn eridani() -> Cielo {
        let eje = v3(0.6, 0.3, -0.5).norm();
        let nebulosa = Textura::generar(1024, 512, &|d| {
            let g = d.y * 0.5 + 0.5;
            let mut c = mezcla(rgb(4.0, 8.0, 24.0), rgb(10.0, 12.0, 40.0), g);
            let m = (-(d.dot(eje)).powi(2) * 8.0).exp();
            let n1 = fbm(d * 1.7, 5, 1201) + m * 0.5;
            let n2 = fbm(d * 2.5 + v3(5.0, 1.0, -3.0), 5, 1202) + m * 0.45;
            let n3 = fbm(d * 3.6 + v3(-2.0, 7.0, 4.0), 4, 1203) + m * 0.4;
            c += rgb(34.0, 150.0, 98.0) * (nivel(smoothstep(0.0, 0.6, n1), 4.0) * 0.6);
            c += rgb(30.0, 118.0, 150.0) * (nivel(smoothstep(0.05, 0.62, n2), 4.0) * 0.5);
            c += rgb(88.0, 50.0, 140.0) * (nivel(smoothstep(0.18, 0.75, n3), 3.0) * 0.45);
            c += rgb(210.0, 120.0, 50.0) * (m * m * 0.08);
            v3(c.x.min(1.0), c.y.min(1.0), c.z.min(1.0))
        });

        let mut rng = Rng::new(8888);
        let paleta = [
            rgb(255.0, 255.0, 255.0),
            rgb(255.0, 226.0, 180.0),
            rgb(190.0, 255.0, 230.0),
            rgb(200.0, 220.0, 255.0),
            rgb(235.0, 210.0, 255.0),
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
