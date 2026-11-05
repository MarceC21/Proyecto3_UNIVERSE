// Texturas horneadas una vez: identidad propia con el ruido compartido.
use crate::math::*;
use crate::ruido::*;
use crate::textura::Textura;

pub fn miller() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let n = fbm(d * 4.0, 4, 301);
        let olas = ruido(d * 14.0 + v3(n * 2.3, n * 1.4, -n * 1.8), 302).abs();
        let agua = mezcla(rgb(17.0, 73.0, 110.0), rgb(60.0, 157.0, 176.0), n * 0.45 + 0.5);
        mezcla(agua, rgb(203.0, 230.0, 218.0), (1.0 - smoothstep(0.015, 0.075, olas)) * 0.65)
    })
}

pub fn mann() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let n = fbm(d * 3.2, 5, 311);
        let c = mezcla(rgb(74.0, 130.0, 163.0), rgb(217.0, 239.0, 235.0), smoothstep(-0.3, 0.3, n));
        if ruido(d * 8.0, 312).abs() < 0.028 { c * 0.58 } else { c }
    })
}

pub fn edmunds() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let n = fbm(d * 3.0, 5, 321);
        let mut c = mezcla(rgb(123.0, 57.0, 38.0), rgb(222.0, 157.0, 87.0), smoothstep(-0.3, 0.3, n));
        if ruido(d * 4.0 + v3(4.0, 1.0, 2.0), 322).abs() < 0.045 { c = rgb(66.0, 42.0, 39.0); }
        c
    })
}

pub fn thalos() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let n = fbm(d * 4.0, 4, 331);
        let bandas = (d.y * 33.0 + n * 2.5).sin() * 0.5 + 0.5;
        mezcla(rgb(69.0, 81.0, 120.0), rgb(185.0, 191.0, 211.0), smoothstep(0.2, 0.8, bandas))
    })
}

pub fn nyx() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let n = fbm(d * 3.0, 4, 341);
        let (a, b, _) = celdas(d * 5.0, 342);
        let roca = mezcla(rgb(43.0, 37.0, 66.0), rgb(119.0, 100.0, 147.0), n * 0.5 + 0.5);
        mezcla(roca, rgb(218.0, 139.0, 99.0), (1.0 - smoothstep(0.015, 0.07, b - a)) * 0.85)
    })
}

pub fn aurora() -> Textura {
    Textura::generar(1024, 512, &|d| {
        let n = fbm(d * 2.8, 5, 351);
        let mar = mezcla(rgb(23.0, 88.0, 93.0), rgb(60.0, 153.0, 132.0), n * 0.5 + 0.5);
        if n > 0.13 { mezcla(rgb(151.0, 167.0, 114.0), rgb(221.0, 216.0, 170.0), smoothstep(0.13, 0.45, n)) }
        else if d.y.abs() > 0.88 { rgb(214.0, 236.0, 215.0) } else { mar }
    })
}
