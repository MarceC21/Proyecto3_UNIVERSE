// Texturas horneadas de 40 Eridani. Mismo estilo "caricatura": pocos tonos
// planos con bordes suaves.
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

// Erid: azul profundo con bandas finas y remolinos de nubes blancas.
pub fn erid() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let w = fbm(d * 2.2 + v3(2.0, 5.0, 1.0), 4, 1101);
        let t = fbm(d * 5.0, 4, 1102);
        let bandas = (d.y * 20.0 + w * 3.0 + t * 0.8).sin() * 0.5 + 0.5;
        let mut c = mezcla(
            rgb(8.0, 26.0, 84.0),
            rgb(44.0, 110.0, 196.0),
            cuantizar_suave(bandas, 4.0),
        );
        let fino = (d.y * 70.0 + t * 4.0).sin() * 0.5 + 0.5;
        c = mezcla(
            c,
            rgb(120.0, 180.0, 235.0),
            smoothstep(0.8, 1.0, fino) * 0.3,
        );
        let n = fbm(
            d * 3.5 + v3(w * 1.5, w, -w * 1.2) + v3(4.0, 1.0, 7.0),
            5,
            1103,
        );
        let nube = cuantizar_suave(smoothstep(0.12, 0.30, n), 3.0);
        mezcla(c, rgb(232.0, 244.0, 255.0), nube * 0.85)
    })
}

// Gigante gaseoso blanco-azulado pálido, bandas suaves.
pub fn gigante() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let t = fbm(d * 5.0, 4, 1111);
        let bandas = (d.y * 26.0 + t * 2.0).sin() * 0.5 + 0.5;
        let mut c = mezcla(
            rgb(150.0, 200.0, 226.0),
            rgb(240.0, 249.0, 252.0),
            cuantizar_suave(bandas, 4.0),
        );
        let fino = (d.y * 80.0 + t * 3.0).sin() * 0.5 + 0.5;
        c = mezcla(
            c,
            rgb(112.0, 170.0, 210.0),
            smoothstep(0.78, 1.0, fino) * 0.3,
        );
        mezcla(
            c,
            rgb(214.0, 238.0, 250.0),
            smoothstep(0.6, 0.95, d.y.abs()) * 0.4,
        )
    })
}

// 40 Eri A 2: mundo templado tostado, mares azules y nubes blancas.
pub fn templado() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let h = fbm(d * 2.0, 5, 1121);
        let costa = 0.02;
        let mut c = if h < costa {
            let prof = clamp01((h + 0.45) / (costa + 0.45));
            let mut m = mezcla(
                rgb(26.0, 62.0, 130.0),
                rgb(70.0, 140.0, 190.0),
                escalon(prof, 4.0),
            );
            if h > costa - 0.03 {
                m = rgb(150.0, 206.0, 214.0);
            }
            m
        } else {
            let alt = clamp01((h - costa) / 0.45);
            let mut t = mezcla(
                rgb(196.0, 150.0, 96.0),
                rgb(140.0, 96.0, 62.0),
                escalon(alt * 1.6, 3.0),
            );
            if alt > 0.7 {
                t = rgb(120.0, 100.0, 90.0);
            }
            if h < costa + 0.022 {
                t = t * 0.78;
            }
            t
        };
        if d.y.abs() > 0.9 {
            c = rgb(238.0, 244.0, 248.0);
        }
        let n = fbm(d * 3.2 + v3(6.0, 2.0, 1.0), 5, 1122);
        mezcla(
            c,
            rgb(246.0, 248.0, 250.0),
            cuantizar_suave(smoothstep(0.15, 0.32, n), 3.0) * 0.8,
        )
    })
}

// Rocoso con cráteres; cambia color y semilla.
fn rocoso(semilla: u32, oscuro: V3, claro: V3, manchas: V3) -> Textura {
    Textura::generar(1024, 512, &move |d| {
        let t = escalon(clamp01(fbm(d * 2.6, 4, semilla) * 0.9 + 0.5), 3.0);
        let mut c = mezcla(oscuro, claro, t);
        if fbm(d * 1.7 + v3(2.0, 5.0, 1.0), 4, semilla + 1) > 0.2 {
            c = manchas;
        }
        for (i, esc) in [2.4f32, 6.0, 14.0].into_iter().enumerate() {
            let (f1, _, id) = celdas(d * esc, semilla + 2 + i as u32);
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

// 40 Eri A 1: rojo ladrillo.
pub fn rojo_a1() -> Textura {
    rocoso(
        1131,
        rgb(120.0, 40.0, 34.0),
        rgb(196.0, 84.0, 60.0),
        rgb(86.0, 30.0, 30.0),
    )
}

// 40 Eri A 2b: rojizo-marrón más claro.
pub fn rojo_a2b() -> Textura {
    rocoso(
        1141,
        rgb(138.0, 62.0, 44.0),
        rgb(214.0, 118.0, 80.0),
        rgb(104.0, 52.0, 44.0),
    )
}
