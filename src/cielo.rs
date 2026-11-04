// Skybox estilo caricatura: nebulosas de colores planos + estrellas.
// La nebulosa se hornea una vez en una textura (cambia muy suavemente, así que
// la deformación equirectangular no importa). Las estrellas son puntos 3D que
// se proyectan cada cuadro: sin deformación y con parpadeo animado.
use crate::math::*;
use crate::ruido::*;
use crate::textura::Textura;

pub struct Estrella {
    pub dir: V3,
    pub brillo: f32,
    pub color: V3,
    pub tipo: u8, // 0 = punto, 1 = punto grande, 2 = destello en cruz, 3 = estrella grande
    pub fase: f32,
    pub vel: f32,
}

pub struct Cielo {
    pub nebulosa: Textura,
    pub estrellas: Vec<Estrella>,
}

fn nivel(x: f32, n: f32) -> f32 {
    let k = x.clamp(0.0, 0.999) * n;
    let f = k.floor();
    (f + smoothstep(0.40, 0.60, k - f)) / n
}

impl Cielo {
    pub fn nuevo() -> Cielo {
        let eje_galaxia = v3(0.35, 0.85, 0.30).norm();
        let nebulosa = Textura::generar(1024, 512, &|d| {
            // Degradado base: azul noche -> violeta oscuro.
            let g = d.y * 0.5 + 0.5;
            let mut c = mezcla(rgb(7.0, 5.0, 26.0), rgb(20.0, 9.0, 50.0), g);
            // Banda galáctica: más densidad cerca de un gran círculo.
            let m = (-(d.dot(eje_galaxia)).powi(2) * 12.0).exp();
            let n1 = fbm(d * 1.7, 5, 101) + m * 0.55;
            let n2 = fbm(d * 2.6 + v3(5.2, 1.3, -3.1), 5, 102) + m * 0.45;
            let n3 = fbm(d * 3.8 + v3(-2.0, 7.7, 4.4), 4, 103) + m * 0.5;
            c += rgb(98.0, 44.0, 168.0) * (nivel(smoothstep(-0.05, 0.55, n1), 4.0) * 0.75);
            c += rgb(206.0, 58.0, 152.0) * (nivel(smoothstep(0.08, 0.65, n2), 3.0) * 0.55);
            c += rgb(36.0, 168.0, 206.0) * (nivel(smoothstep(0.20, 0.75, n3), 3.0) * 0.45);
            // Núcleo cálido de la galaxia.
            c += rgb(255.0, 150.0, 90.0) * (m * m * 0.18);
            v3(c.x.min(1.0), c.y.min(1.0), c.z.min(1.0))
        });

        let mut rng = Rng::new(2026);
        let paleta = [rgb(255.0, 255.0, 255.0), rgb(255.0, 236.0, 190.0), rgb(190.0, 220.0, 255.0),
                      rgb(255.0, 190.0, 220.0), rgb(190.0, 255.0, 240.0)];
        let mut estrellas = Vec::new();
        for i in 0..3200 {
            let tipo = if i < 10 { 3 } else if i < 70 { 2 } else if i < 600 { 1 } else { 0 };
            estrellas.push(Estrella {
                dir: rng.direccion(),
                brillo: match tipo { 0 => rng.rango(0.35, 0.8), 1 => rng.rango(0.6, 1.0), _ => 1.0 },
                color: paleta[(rng.siguiente() % paleta.len() as u64) as usize],
                tipo,
                fase: rng.rango(0.0, 6.28),
                vel: rng.rango(0.8, 3.0),
            });
        }
        Cielo { nebulosa, estrellas }
    }
}
