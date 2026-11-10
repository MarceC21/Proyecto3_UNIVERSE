// Texturas horneadas de Tau Ceti. Mismo estilo "caricatura" que el sistema solar:
// pocos tonos planos con bordes suaves. Adrien usa deformación de dominio (warp)
// para lograr los remolinos verde/naranja.
use crate::math::*;
use crate::ruido::*;
use crate::textura::Textura;

fn escalon(x: f32, n: f32) -> f32 {
    (x * n).floor() / n
}

fn cuantizar_suave(x: f32, n: f32) -> f32 {
    let k = x * n;
    let f = k.floor();
    (f + smoothstep(0.42, 0.58, k - f)) / n
}

// Planeta principal: remolinos verdes y naranjas, con vetas oscuras e hilos claros.
pub fn adrien() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let w1 = v3(
            fbm(d * 1.6 + v3(1.0, 4.0, 2.0), 4, 801),
            fbm(d * 1.6 + v3(7.0, 1.0, 5.0), 4, 802),
            fbm(d * 1.6 + v3(3.0, 9.0, 6.0), 4, 803),
        );
        let w2 = fbm(d * 2.4 + w1 * 1.9 + v3(5.0, 2.0, 8.0), 4, 804);
        let q = d * 3.0 + v3(w2, w2 * 0.8, -w2) * 1.6 + w1;
        let n = fbm(q, 5, 805);
        let remolino = ruido(q * 2.2, 806).abs();
        let t = cuantizar_suave(clamp01(n * 0.9 + 0.5), 4.0);
        let verde = mezcla(rgb(20.0, 92.0, 28.0), rgb(150.0, 214.0, 40.0), t);
        let naranja = mezcla(rgb(196.0, 84.0, 24.0), rgb(255.0, 160.0, 48.0), t);
        let reparto = smoothstep(
            -0.05,
            0.12,
            fbm(d * 1.8 + w1 * 1.2 + v3(4.0, 1.0, 9.0), 4, 807),
        );
        let mut c = mezcla(verde, naranja, reparto);
        c = c * (1.0 - 0.35 * (1.0 - smoothstep(0.02, 0.09, remolino))); // vetas oscuras
        mezcla(
            c,
            rgb(214.0, 240.0, 120.0),
            (1.0 - smoothstep(0.0, 0.03, remolino)) * 0.35,
        ) // hilos claros
    })
}

// Gigante gaseoso azul (como Júpiter): bandas + gran tormenta oval.
pub fn gigante() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let t = fbm(d * 6.0, 4, 811);
        let bandas = (d.y * 34.0 + t * 2.4).sin() * 0.5 + 0.5;
        let mut c = mezcla(
            rgb(28.0, 76.0, 170.0),
            rgb(130.0, 190.0, 245.0),
            cuantizar_suave(bandas, 4.0),
        );
        let fino = (d.y * 90.0 + t * 3.0).sin() * 0.5 + 0.5;
        c = mezcla(
            c,
            rgb(18.0, 48.0, 128.0),
            smoothstep(0.75, 1.0, fino) * 0.35,
        );
        let m = ((d.x - 0.90) / 0.24).powi(2)
            + ((d.y + 0.30) / 0.13).powi(2)
            + ((d.z - 0.35) / 0.34).powi(2);
        c = mezcla(c, rgb(200.0, 228.0, 255.0), 1.0 - smoothstep(0.7, 1.1, m));
        mezcla(
            c,
            rgb(20.0, 40.0, 120.0),
            1.0 - smoothstep(0.35, 0.7, m + t * 0.15),
        )
    })
}

// El pequeño con anillos: bandas moradas.
pub fn saturn() -> Textura {
    Textura::generar(512, 256, &|d| {
        let bandas = (d.y * 26.0 + fbm(d * 4.0, 3, 851) * 1.0).sin() * 0.5 + 0.5;
        mezcla(
            rgb(96.0, 70.0, 150.0),
            rgb(196.0, 164.0, 228.0),
            cuantizar_suave(bandas, 4.0),
        )
    })
}

// Mundo templado azul/verde azulado con casquetes polares.
pub fn ceti_nova() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let h = fbm(d * 2.0, 5, 821);
        let lat = d.y.abs();
        let costa = 0.03;
        if lat > 0.88 + 0.04 * fbm(d * 6.0, 3, 822) {
            return rgb(232.0, 246.0, 250.0);
        }
        if h < costa {
            let prof = clamp01((h + 0.45) / (costa + 0.45));
            let mut c = mezcla(
                rgb(12.0, 58.0, 138.0),
                rgb(36.0, 170.0, 204.0),
                escalon(prof, 4.0),
            );
            if h > costa - 0.03 {
                c = rgb(130.0, 228.0, 224.0);
            }
            c
        } else {
            let alt = (h - costa) / 0.45;
            let mut c = mezcla(
                rgb(88.0, 184.0, 112.0),
                rgb(26.0, 108.0, 94.0),
                escalon(clamp01(alt * 1.6), 3.0),
            );
            if alt > 0.65 {
                c = rgb(150.0, 132.0, 120.0);
            }
            if alt > 0.92 {
                c = rgb(238.0, 242.0, 246.0);
            }
            if h < costa + 0.022 {
                c = c * 0.78;
            }
            c
        }
    })
}

// Rocoso marrón con cráteres.
pub fn ceti_nova_ii() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let t = escalon(clamp01(fbm(d * 2.6, 4, 831) * 0.9 + 0.5), 3.0);
        let mut c = mezcla(rgb(140.0, 92.0, 66.0), rgb(206.0, 152.0, 108.0), t);
        if fbm(d * 1.6 + v3(2.0, 5.0, 1.0), 4, 832) > 0.2 {
            c = rgb(96.0, 62.0, 52.0);
        }
        for (esc, sem) in [(2.4f32, 833u32), (6.0, 834), (14.0, 835)] {
            let (f1, _, id) = celdas(d * esc, sem);
            if id > 0.45 {
                let r = 0.16 + 0.22 * id;
                if f1 < r * 0.82 {
                    c = c * 0.74;
                } else if f1 < r {
                    c = c * 1.2;
                }
            }
        }
        c
    })
}

// Mundo océano: agua profunda turquesa, espuma e islas pequeñas.
pub fn oceanus() -> Textura {
    Textura::generar(512, 256, &|d| {
        let n = fbm(d * 3.0, 4, 841);
        let prof = escalon(clamp01(n * 0.9 + 0.5), 4.0);
        let mut c = mezcla(rgb(8.0, 52.0, 104.0), rgb(34.0, 150.0, 176.0), prof);
        let olas = ruido(d * 12.0 + v3(n * 2.0, n * 1.2, -n * 1.6), 842).abs();
        c = mezcla(
            c,
            rgb(214.0, 240.0, 236.0),
            (1.0 - smoothstep(0.015, 0.06, olas)) * 0.5,
        );
        if n > 0.3 {
            c = mezcla(
                rgb(236.0, 214.0, 150.0),
                rgb(96.0, 170.0, 96.0),
                smoothstep(0.3, 0.45, n),
            );
        }
        c
    })
}

// Rocoso rojizo con cañones y manchas verdosas.
pub fn veridia() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let t = escalon(clamp01(fbm(d * 2.5, 4, 861) * 0.9 + 0.5), 3.0);
        let mut c = mezcla(rgb(146.0, 70.0, 60.0), rgb(214.0, 130.0, 96.0), t);
        if fbm(d * 1.8 + v3(3.0, 1.0, 6.0), 4, 862) > 0.22 {
            c = mezcla(rgb(92.0, 128.0, 84.0), rgb(140.0, 168.0, 100.0), t);
        }
        if ruido(d * 3.4 + v3(7.0, 2.0, 1.0), 863).abs() < 0.022 {
            c = rgb(84.0, 40.0, 40.0);
        }
        c
    })
}

// Rocoso marrón/oliva con cráteres.
pub fn aethel() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let t = escalon(clamp01(fbm(d * 2.8, 4, 871) * 0.9 + 0.5), 3.0);
        let mut c = mezcla(rgb(112.0, 86.0, 56.0), rgb(168.0, 150.0, 92.0), t);
        if fbm(d * 2.0 + v3(1.0, 8.0, 3.0), 4, 872) > 0.15 {
            c = mezcla(rgb(62.0, 104.0, 56.0), rgb(110.0, 150.0, 76.0), t);
        }
        for (esc, sem) in [(3.0f32, 873u32), (8.0, 874)] {
            let (f1, _, id) = celdas(d * esc, sem);
            if id > 0.5 && f1 < 0.18 {
                c = c * 0.75;
            }
        }
        c
    })
}
