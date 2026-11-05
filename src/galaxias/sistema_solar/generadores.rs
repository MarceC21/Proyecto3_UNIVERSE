// Generadores procedurales de texturas. Estilo "caricatura": los colores se
// cuantizan en pocos tonos planos con bordes marcados, en vez de degradados.
use crate::math::*;
use crate::ruido::*;
use crate::textura::Textura;

fn escalon(x: f32, n: f32) -> f32 { (x * n).floor() / n }

fn cuantizar_suave(x: f32, n: f32) -> f32 {
    // Escalones con transición corta (evita bordes dentados al muestrear).
    let k = x * n;
    let f = k.floor();
    (f + smoothstep(0.42, 0.58, k - f)) / n
}

pub fn tierra() -> Textura {
    Textura::generar(2048, 1024, &|d| {
        let h = fbm(d * 1.9, 6, 11);
        let lat = d.y.abs();
        let costa = 0.04;
        let hielo = lat > 0.86 + 0.05 * fbm(d * 6.0, 3, 4);
        if h < costa {
            if hielo { return rgb(236.0, 246.0, 255.0); }
            let prof = clamp01((h + 0.45) / (costa + 0.45));
            let mut c = mezcla(rgb(16.0, 56.0, 146.0), rgb(38.0, 148.0, 214.0), escalon(prof, 4.0));
            if h > costa - 0.03 { c = rgb(142.0, 224.0, 240.0); } // aguas someras
            c
        } else {
            if hielo { return rgb(244.0, 248.0, 252.0); }
            let alt = (h - costa) / 0.45;
            let seco = fbm(d * 3.1 + v3(5.0, 2.0, 9.0), 3, 5);
            let mut c = if seco > 0.12 && lat < 0.6 {
                mezcla(rgb(226.0, 190.0, 108.0), rgb(204.0, 150.0, 84.0), escalon(alt, 3.0)) // desierto
            } else {
                mezcla(rgb(92.0, 180.0, 74.0), rgb(40.0, 124.0, 64.0), escalon(clamp01(alt * 1.6), 3.0)) // pasto/bosque
            };
            if alt > 0.62 { c = rgb(140.0, 112.0, 92.0); } // montañas
            if alt > 0.92 { c = rgb(240.0, 240.0, 244.0); } // nieve
            if h < costa + 0.022 { c = c * 0.78; } // contorno oscuro de costa
            c
        }
    })
}

pub fn nubes() -> Textura {
    // Capa independiente: el canal R es la opacidad de las nubes.
    Textura::generar(1024, 512, &|d| {
        let w = fbm(d * 2.0 + v3(3.0, 1.0, 7.0), 3, 71) * 0.8;
        let n = fbm(d * 3.2 + v3(w, w * 0.7, -w), 5, 72);
        let a = clamp01((n - 0.12) * 4.0);
        let a = escalon(a, 3.0) * 0.5 + a * 0.5;
        v3(a, a, a)
    })
}

pub fn mercurio() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let t = escalon(clamp01(fbm(d * 2.6, 4, 21) * 0.9 + 0.5), 3.0);
        let mut c = mezcla(rgb(122.0, 114.0, 110.0), rgb(176.0, 166.0, 156.0), t);
        // Cráteres: celdas con una "boca" oscura y un borde claro, a 3 escalas.
        for (esc, sem) in [(2.2f32, 31u32), (5.5, 32), (13.0, 33)] {
            let (f1, _, id) = celdas(d * esc, sem);
            if id > 0.45 {
                let r = 0.16 + 0.22 * id;
                if f1 < r * 0.82 { c = c * 0.74; }
                else if f1 < r { c = c * 1.22; }
            }
        }
        c
    })
}

pub fn venus() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let warp = fbm(d * 2.0, 4, 41) * 1.8;
        let bandas = (d.y * 7.0 + warp * 3.2 + fbm(d * 5.0, 3, 42) * 0.7).sin() * 0.5 + 0.5;
        let q = cuantizar_suave(bandas, 3.0);
        let mut c = if q < 0.5 {
            mezcla(rgb(222.0, 138.0, 66.0), rgb(238.0, 184.0, 96.0), q * 2.0)
        } else {
            mezcla(rgb(238.0, 184.0, 96.0), rgb(250.0, 230.0, 168.0), (q - 0.5) * 2.0)
        };
        if fbm(d * 3.0 + v3(1.0, 8.0, 2.0), 3, 43) > 0.3 { c = c * 1.07; }
        c
    })
}

pub fn marte() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let lat = d.y.abs();
        let t = escalon(clamp01(fbm(d * 2.5, 4, 51) * 0.9 + 0.5), 3.0);
        let mut c = mezcla(rgb(148.0, 58.0, 38.0), rgb(206.0, 98.0, 58.0), t);
        if fbm(d * 1.5 + v3(4.0, 0.0, 1.0), 4, 52) > 0.22 { c = rgb(112.0, 48.0, 38.0); } // mares oscuros
        if fbm(d * 3.0 + v3(0.0, 3.0, 6.0), 3, 53) > 0.32 { c = rgb(232.0, 152.0, 96.0); } // dunas claras
        if ruido(d * 3.4 + v3(7.0, 2.0, 1.0), 54).abs() < 0.022 && lat < 0.45 { c = rgb(84.0, 34.0, 28.0); } // cañón
        if lat > 0.9 - 0.035 * fbm(d * 7.0, 3, 55) { c = rgb(248.0, 242.0, 236.0); } // casquete polar
        c
    })
}

// Gigantes gaseosos: bandas por latitud perturbadas con el mismo ruido 3D
// de los planetas interiores. Se hornean una vez y conservan los mipmaps.
pub fn jupiter() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let turbulencia = fbm(d * 6.0, 4, 101);
        let bandas = (d.y * 38.0 + turbulencia * 2.2).sin() * 0.5 + 0.5;
        let mut c = mezcla(rgb(170.0, 100.0, 65.0), rgb(245.0, 220.0, 180.0), cuantizar_suave(bandas, 4.0));
        // Óvalo localizado sobre la esfera: Gran Mancha Roja con borde claro.
        let mancha = ((d.x - 0.93) / 0.22).powi(2)
            + ((d.y + 0.22) / 0.12).powi(2) + ((d.z - 0.28) / 0.33).powi(2);
        let borde = 1.0 - smoothstep(0.85, 1.25, mancha);
        c = mezcla(c, rgb(241.0, 189.0, 132.0), borde);
        mezcla(c, rgb(183.0, 66.0, 43.0), 1.0 - smoothstep(0.45, 0.85, mancha + turbulencia * 0.16))
    })
}

pub fn saturno() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let bandas = (d.y * 30.0 + fbm(d * 4.0, 3, 111) * 0.9).sin() * 0.5 + 0.5;
        let c = mezcla(rgb(188.0, 153.0, 91.0), rgb(245.0, 224.0, 170.0), cuantizar_suave(bandas, 4.0));
        c * (0.94 + 0.06 * (1.0 - d.y.abs()))
    })
}

pub fn urano() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let bandas = (d.y * 24.0 + fbm(d * 3.0, 3, 121) * 0.5).sin() * 0.5 + 0.5;
        let c = mezcla(rgb(104.0, 192.0, 201.0), rgb(159.0, 225.0, 227.0), bandas * 0.45);
        mezcla(c, rgb(195.0, 239.0, 234.0), smoothstep(0.55, 0.95, d.y) * 0.65)
    })
}

pub fn neptuno() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let n = fbm(d * 5.0, 4, 131);
        let bandas = (d.y * 28.0 + n * 1.5).sin() * 0.5 + 0.5;
        let mut c = mezcla(rgb(30.0, 67.0, 174.0), rgb(61.0, 128.0, 230.0), cuantizar_suave(bandas, 3.0));
        let tormenta = ((d.x - 0.94) / 0.25).powi(2)
            + ((d.y + 0.27) / 0.13).powi(2) + ((d.z - 0.20) / 0.30).powi(2);
        c = mezcla(c, rgb(19.0, 38.0, 109.0), 1.0 - smoothstep(0.5, 1.0, tormenta + n * 0.2));
        let nube = smoothstep(0.30, 0.43, n) * (1.0 - smoothstep(0.4, 0.65, d.y.abs()));
        mezcla(c, rgb(184.0, 217.0, 255.0), nube * 0.7)
    })
}

pub fn sol() -> Textura {
    // Valor 0..1: gránulos tipo "celdas" + manchas. El shader lo anima y lo colorea.
    Textura::generar(1024, 512, &|d| {
        let (f1, f2, _) = celdas(d * 7.0, 61);
        let grano = clamp01((f2 - f1) * 2.2);
        let v = grano * 0.55 + (fbm(d * 3.0, 4, 62) * 0.5 + 0.5) * 0.55;
        v3(v, v, v)
    })
}

pub fn asteroide() -> Textura {
    Textura::generar(256, 128, &|d| {
        let t = escalon(clamp01(fbm(d * 3.0, 4, 81) * 0.9 + 0.5), 3.0);
        let mut c = mezcla(rgb(96.0, 86.0, 82.0), rgb(158.0, 146.0, 136.0), t);
        let (f1, _, id) = celdas(d * 6.0, 82);
        if id > 0.5 && f1 < 0.2 { c = c * 0.7; } // hoyos
        c
    })
}
