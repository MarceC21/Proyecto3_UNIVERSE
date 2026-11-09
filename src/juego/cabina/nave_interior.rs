// Interior de la nave: cabina octogonal (réplica del proyecto 2 en el rasterizador),
// esclusa trasera y túnel de atraque. Las ventanas son huecos: detrás se dibuja el exterior.
use crate::construccion::*;
use crate::dibujo_interior::Escenario;
use crate::jugador::{Mundo, Obst, Zona};
use crate::materiales::Mat;
use crate::math::*;
use std::sync::Arc;

pub const CONTORNO: [(f32, f32); 8] = [
    (-4.2, -15.0),
    (4.2, -15.0),
    (10.0, -13.2),
    (10.0, -3.8),
    (4.2, 8.0),
    (-4.2, 8.0),
    (-10.0, -3.8),
    (-10.0, -13.2),
];
pub const ALTO: f32 = 7.0;
pub const ALTO_ESCLUSA: f32 = 4.4;
pub const FIN_ESCLUSA: f32 = 14.0;
pub const FIN_TUNEL: f32 = 18.0;

pub struct Puntos {
    pub nucleo: V3,
    pub pilotaje: V3,
    pub esclusa: V3,
    pub muelle: V3,
    pub planetario: V3,
    pub puerta_centro: V3,
    pub puerta_dir: V3,
    pub puerta_yaw: f32,
    pub escotilla_centro: V3,
    pub escotilla_dir: V3,
    pub escotilla_yaw: f32,
}

fn xmax(z: f32) -> f32 {
    if z < -13.2 {
        4.2 + (z + 15.0) / 1.8 * 5.8
    } else if z <= -3.8 {
        10.0
    } else {
        10.0 - (z + 3.8) / 11.8 * 5.8
    }
}

pub fn monitor(c: &mut Constructor, m: &Marco, u: f32, ancho: f32, mat: Mat) {
    let alto = ancho / 1.6;
    let y = 0.95 + 0.23 + alto / 2.0;
    m.caja(
        c.g(Mat::MetalOscuro),
        u,
        0.95 + 0.12,
        0.29,
        ancho * 0.45,
        0.24,
        0.30,
    );
    m.caja(c.g(Mat::Metal), u, y, 0.28, ancho + 0.18, alto + 0.18, 0.18);
    m.caja(
        c.g(Mat::MetalOscuro),
        u,
        y,
        0.378,
        ancho + 0.06,
        alto + 0.06,
        0.025,
    );
    m.pantalla(c.g(mat), u, y, 0.40, ancho, alto);
}

pub fn panel(c: &mut Constructor, m: &Marco, u: f32, dep: f32, ancho: f32, fondo: f32, mat: Mat) {
    m.caja(
        c.g(Mat::MetalOscuro),
        u,
        0.985,
        dep,
        ancho + 0.1,
        0.07,
        fondo + 0.1,
    );
    m.plano(c.g(mat), u, 1.03, dep, ancho, fondo);
}

fn palanca(c: &mut Constructor, m: &Marco, u: f32, dep: f32) {
    m.caja(c.g(Mat::Metal), u, 1.0, dep, 0.36, 0.1, 0.48);
    m.caja(c.g(Mat::Metal), u, 1.21, dep, 0.065, 0.32, 0.065);
    m.caja(
        c.pintura(v3(0.72, 0.14, 0.1)),
        u,
        1.42,
        dep,
        0.26,
        0.14,
        0.16,
    );
}

fn consola(c: &mut Constructor, m: &Marco, u0: f32, u1: f32, obst: &mut Vec<Obst>) {
    let (w, uc) = (u1 - u0, (u0 + u1) / 2.0);
    m.caja(c.g(Mat::MetalOscuro), uc, 0.4, 0.75, w, 0.8, 1.5);
    m.caja(c.g(Mat::Metal), uc, 0.90, 0.75, w + 0.1, 0.1, 1.5);
    m.caja(c.g(Mat::Cian), uc, 0.70, 1.49, w * 0.72, 0.03, 0.03);
    m.caja(c.g(Mat::Metal), uc, 0.98, 0.07, w, 0.06, 0.1);
    m.caja(c.g(Mat::Metal), uc, 0.98, 1.43, w, 0.06, 0.1);
    let p = m.p(uc, 0.0, 0.75);
    obst.push(Obst {
        cx: p.x,
        cz: p.z,
        hx: w / 2.0 + 0.05,
        hz: 0.78,
        yaw: m.yaw,
    });
}

fn silla(c: &mut Constructor, m: &Marco, u: f32, dep: f32, obst: &mut Vec<Obst>) {
    let tela = Mat::Tapizado;
    m.caja(c.g(Mat::MetalOscuro), u, 0.055, dep, 1.10, 0.11, 1.15);
    m.caja(c.g(Mat::Metal), u, 0.24, dep, 0.34, 0.27, 0.38);
    m.caja(c.g(Mat::MetalOscuro), u, 0.405, dep, 1.12, 0.13, 1.18);
    m.caja(c.g(tela), u, 0.54, dep - 0.035, 1.02, 0.16, 1.05);
    m.caja(c.g(Mat::MetalOscuro), u, 1.14, dep + 0.54, 1.12, 1.24, 0.25);
    m.caja(c.g(tela), u, 1.17, dep + 0.365, 0.98, 1.02, 0.18);
    m.caja(c.g(Mat::MetalOscuro), u, 1.89, dep + 0.50, 0.74, 0.34, 0.22);
    m.caja(c.g(tela), u, 1.89, dep + 0.355, 0.66, 0.27, 0.10);
    for lado in [-1.0f32, 1.0] {
        m.caja(
            c.g(Mat::Metal),
            u + lado * 0.55,
            0.665,
            dep + 0.05,
            0.10,
            0.43,
            0.13,
        );
        m.caja(
            c.g(Mat::MetalOscuro),
            u + lado * 0.55,
            0.88,
            dep - 0.07,
            0.20,
            0.10,
            0.92,
        );
        m.caja(
            c.g(tela),
            u + lado * 0.55,
            0.945,
            dep - 0.07,
            0.18,
            0.05,
            0.86,
        );
    }
    let p = m.p(u, 0.0, dep);
    obst.push(Obst {
        cx: p.x,
        cz: p.z,
        hx: 0.62,
        hz: 0.7,
        yaw: m.yaw,
    });
}

pub fn construir() -> (Escenario, Puntos) {
    let mut c = Constructor::nuevo();
    let mut obst: Vec<Obst> = vec![];
    let m: Vec<Marco> = (0..8)
        .map(|i| Marco::de(CONTORNO[i], CONTORNO[(i + 1) % 8]))
        .collect();

    // ---------------- Paredes, ventanas y marcos ----------------
    let ventanas = [
        (0usize, 0.45f32, 7.95f32, 0.95f32, 5.85f32),
        (1, 0.43, 5.64, 0.9, 5.8),
        (7, 0.43, 5.64, 0.9, 5.8),
    ];
    for i in 0..8 {
        let ab: Vec<_> = match i {
            0 | 1 | 7 => ventanas
                .iter()
                .filter(|v| v.0 == i)
                .map(|v| (v.1, v.2, v.3, v.4))
                .collect(),
            4 => vec![(2.3, 6.1, 0.0, 4.5)],
            _ => vec![],
        };
        m[i].pared(c.g(Mat::Pared), ALTO, &ab);
    }
    for &(i, u0, u1, y0, y1) in &ventanas {
        let mm = m[i];
        let (uc, yc) = ((u0 + u1) / 2.0, (y0 + y1) / 2.0);
        mm.caja(
            c.g(Mat::MetalOscuro),
            uc,
            y0 + 0.06,
            -0.1,
            u1 - u0,
            0.12,
            0.3,
        );
        mm.caja(
            c.g(Mat::MetalOscuro),
            uc,
            y1 - 0.06,
            -0.1,
            u1 - u0,
            0.12,
            0.3,
        );
        mm.caja(
            c.g(Mat::MetalOscuro),
            u0 + 0.06,
            yc,
            -0.1,
            0.12,
            y1 - y0,
            0.3,
        );
        mm.caja(
            c.g(Mat::MetalOscuro),
            u1 - 0.06,
            yc,
            -0.1,
            0.12,
            y1 - y0,
            0.3,
        );
        for k in 1..3 {
            mm.caja(
                c.g(Mat::Metal),
                u0 + (u1 - u0) * k as f32 / 3.0,
                yc,
                -0.05,
                0.07,
                y1 - y0,
                0.14,
            );
        }
        mm.caja(
            c.g(Mat::Cian),
            uc,
            y1 + 0.12,
            0.02,
            (u1 - u0) * 0.8,
            0.04,
            0.03,
        );
    }

    // Puerta trasera (hacia la esclusa).
    let m4 = m[4];
    for u in [2.05f32, 6.35] {
        m4.caja(c.g(Mat::Refuerzo), u, 2.35, 0.0, 0.5, 4.7, 0.66);
    }
    m4.caja(c.g(Mat::Refuerzo), 4.2, 4.78, 0.0, 4.8, 0.56, 0.66);
    m4.caja(c.g(Mat::Cian), 4.2, 4.79, 0.34, 2.8, 0.08, 0.025);
    for u in [1.8f32, 6.6] {
        m4.caja(c.g(Mat::Cian), u, 2.35, 0.34, 0.045, 3.95, 0.025);
    }
    m4.caja(c.g(Mat::Ambar), 4.2, 5.35, 0.34, 1.6, 0.2, 0.025);

    // ---------------- Suelo, techo, vigas y lámparas ----------------
    c.g(Mat::Piso).poligono_h(&CONTORNO, 0.0, true);
    c.g(Mat::Techo).poligono_h(&CONTORNO, ALTO, false);
    // línea guía cian hacia la esclusa
    c.g(Mat::Cian).rect_h(-0.06, 0.06, 2.2, 8.0, 0.012, true);
    for z in [-12.0f32, -6.0, 0.0, 6.0] {
        let w = 2.0 * (xmax(z) - 0.3);
        c.g(Mat::Metal).caja(v3(0.0, 6.82, z), v3(w, 0.36, 0.5));
        c.g(Mat::Cian)
            .caja(v3(0.0, 6.62, z), v3(w * 0.8, 0.03, 0.12));
        for x in [-4.0f32, 4.0] {
            c.g(Mat::Luz).caja(v3(x, 6.62, z), v3(1.6, 0.04, 0.7));
            c.luz(v3(x, 6.1, z), v3(1.0, 0.96, 0.88), 0.55, 12.0);
        }
    }

    // ---------------- Consolas de mando bajo las ventanas ----------------
    consola(&mut c, &m[0], 0.0, 8.4, &mut obst);
    monitor(&mut c, &m[0], 4.2, 2.4, Mat::Radar);
    for u in [1.9f32, 6.5] {
        panel(&mut c, &m[0], u, 0.74, 1.2, 0.96, Mat::Botonera);
        m[0].caja(
            c.g(Mat::Metal),
            u + (if u < 4.2 { -0.95 } else { 0.95 }),
            1.05,
            0.91,
            0.24,
            0.2,
            0.24,
        );
    }
    panel(&mut c, &m[0], 4.2, 1.05, 0.9, 0.5625, Mat::Telemetria);
    palanca(&mut c, &m[0], 3.34, 1.08);
    palanca(&mut c, &m[0], 5.06, 1.08);
    silla(&mut c, &m[0], 2.7, 2.9, &mut obst);
    silla(&mut c, &m[0], 5.7, 2.9, &mut obst);

    consola(&mut c, &m[7], 0.35, m[7].len, &mut obst);
    monitor(&mut c, &m[7], 2.2, 1.9, Mat::Telemetria);
    panel(&mut c, &m[7], 4.9, 0.77, 1.1, 0.88, Mat::Botonera);
    consola(&mut c, &m[1], 0.0, m[1].len - 0.35, &mut obst);
    monitor(&mut c, &m[1], m[1].len - 2.2, 1.9, Mat::Telemetria);
    panel(&mut c, &m[1], 1.2, 0.77, 1.1, 0.88, Mat::Botonera);
    for (mi, u0) in [(7usize, 3.4f32), (1, 3.0)] {
        for k in 0..3 {
            let u = u0 + k as f32 * 0.3;
            m[mi].caja(c.g(Mat::MetalOscuro), u, 0.985, 1.04, 0.22, 0.07, 0.30);
            m[mi].caja(c.g(Mat::Metal), u, 1.07, 1.02, 0.06, 0.12, 0.09);
            m[mi].caja(c.g(Mat::Cian), u, 1.03, 1.15, 0.10, 0.018, 0.035);
        }
    }

    // ---------------- Mesa central con planetario ----------------
    let (hx, hz, top) = (2.6f32, 1.95f32, 0.95f32);
    c.g(Mat::MetalOscuro)
        .caja(v3(0.0, 0.06, 0.0), v3(3.4, 0.12, 2.6));
    c.g(Mat::MetalOscuro)
        .caja(v3(0.0, 0.32, 0.0), v3(1.1, 0.56, 0.9));
    c.g(Mat::MetalOscuro)
        .caja(v3(0.0, 0.62, 0.0), v3(2.0, 0.12, 1.6));
    c.g(Mat::MetalOscuro)
        .caja(v3(0.0, 0.765, 0.0), v3(5.0, 0.08, 3.7));
    c.g(Mat::Metal)
        .caja(v3(0.0, top - 0.1, 0.0), v3(2.0 * hx, 0.2, 2.0 * hz));
    c.g(Mat::Cian)
        .caja(v3(0.0, 0.765, hz - 0.1), v3(3.0, 0.02, 0.02));
    let fw = 0.4;
    c.g(Mat::Metal)
        .caja(v3(0.0, top + 0.035, hz - fw / 2.0), v3(2.0 * hx, 0.07, fw));
    c.g(Mat::Metal).caja(
        v3(0.0, top + 0.035, -(hz - fw / 2.0)),
        v3(2.0 * hx, 0.07, fw),
    );
    c.g(Mat::Metal).caja(
        v3(-(hx - fw / 2.0), top + 0.035, 0.0),
        v3(fw, 0.07, 2.0 * (hz - fw)),
    );
    c.g(Mat::Metal).caja(
        v3(hx - fw / 2.0, top + 0.035, 0.0),
        v3(fw, 0.07, 2.0 * (hz - fw)),
    );
    let (sx, sz) = (hx - fw, hz - fw);
    let g = c.g(Mat::Holo);
    g.quad(
        [
            v3(-sx, top + 0.02, -sz),
            v3(sx, top + 0.02, -sz),
            v3(sx, top + 0.02, sz),
            v3(-sx, top + 0.02, sz),
        ],
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        v3(0.0, 1.0, 0.0),
    );
    // módulo de controles en el lado -X
    c.g(Mat::MetalOscuro)
        .caja(v3(-hx - 0.25, 0.85, 0.5), v3(0.6, 0.3, 1.1));
    c.g(Mat::Botonera).quad(
        [
            v3(-hx - 0.47, 1.012, -0.0),
            v3(-hx - 0.03, 1.012, 0.0),
            v3(-hx - 0.03, 1.012, 0.9),
            v3(-hx - 0.47, 1.012, 0.9),
        ],
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        v3(0.0, 1.0, 0.0),
    );
    obst.push(Obst::caja(0.0, 0.0, 2.0 * hx + 0.1, 2.0 * hz + 0.1));
    obst.push(Obst::caja(-hx - 0.25, 0.5, 0.65, 1.15));
    c.luz(v3(0.0, 1.45, 0.0), v3(1.0, 0.8, 0.45), 1.0, 5.0);

    // ---------------- Muro de mando (diagonal trasera derecha) ----------------
    let m3 = m[3];
    let cols = [0.30f32, m3.len / 3.0, m3.len * 2.0 / 3.0, m3.len - 0.30];
    for (i, &u) in cols.iter().enumerate() {
        let alto = if i == 1 { 1.65 } else { 5.9 };
        m3.caja(c.g(Mat::Refuerzo), u, alto / 2.0, 0.31, 0.34, alto, 0.62);
        m3.caja(
            c.g(Mat::Cian),
            u,
            alto / 2.0,
            0.635,
            0.12,
            alto - 0.5,
            0.025,
        );
    }
    let uc = (cols[0] + cols[2]) / 2.0;
    let anch = cols[2] - cols[0] - 0.48;
    m3.caja(c.g(Mat::Refuerzo), uc, 3.7, 0.13, anch, 3.5, 0.22);
    m3.pantalla(c.g(Mat::Monitor), uc, 3.7, 0.25, anch - 0.2, 3.26);
    m3.caja(c.g(Mat::Refuerzo), uc, 5.68, 0.15, anch, 0.3, 0.3);
    for i in 0..2 {
        let u = (cols[i] + cols[i + 1]) / 2.0;
        let an = cols[i + 1] - cols[i] - 0.48;
        m3.caja(c.g(Mat::Refuerzo), u, 0.55, 0.5, an - 0.12, 1.1, 0.9);
        m3.caja(c.g(Mat::Metal), u, 1.16, 0.84, an, 0.2, 1.56);
        panel(&mut c, &m3, u, 0.9, an - 0.5, 1.1, Mat::Botonera);
        m3.caja(c.g(Mat::Cian), u, 1.73, 0.25, an - 0.24, 0.035, 0.035);
        let p = m3.p(u, 0.0, 0.7);
        obst.push(Obst {
            cx: p.x,
            cz: p.z,
            hx: an / 2.0,
            hz: 0.95,
            yaw: m3.yaw,
        });
    }
    // Núcleo de sistemas (terminal para entregar los materiales).
    let un = (cols[2] + cols[3]) / 2.0;
    m3.caja(c.g(Mat::Refuerzo), un, 2.18, 0.16, 1.65, 2.5, 0.26);
    m3.pantalla(c.g(Mat::Telemetria), un, 2.5, 0.30, 1.4, 0.95);
    m3.pantalla(c.g(Mat::Botonera), un, 1.55, 0.30, 1.4, 0.7);
    m3.caja(c.g(Mat::Rojo), un - 0.5, 3.18, 0.30, 0.3, 0.12, 0.04);
    m3.caja(c.g(Mat::Cian), un + 0.5, 3.18, 0.30, 0.3, 0.12, 0.04);
    c.luz(m3.p(uc, 2.6, 1.8), v3(0.3, 0.7, 1.0), 0.7, 9.0);
    c.luz(m3.p(un, 2.6, 1.2), v3(0.4, 0.8, 1.0), 0.6, 4.0);
    let pn = m3.p(un, 0.0, 0.7);
    obst.push(Obst {
        cx: pn.x,
        cz: pn.z,
        hx: 0.9,
        hz: 0.5,
        yaw: m3.yaw,
    });
    let nucleo = m3.p(un, 1.0, 1.2);

    // ---------------- Laboratorio (diagonal trasera izquierda) ----------------
    let m5 = m[5];
    let ul = m5.len / 2.0;
    m5.caja(
        c.g(Mat::MetalOscuro),
        ul,
        0.53,
        0.8,
        m5.len - 3.0,
        1.06,
        1.5,
    );
    m5.caja(c.g(Mat::Metal), ul, 1.12, 0.85, m5.len - 2.9, 0.12, 1.6);
    m5.caja(c.g(Mat::Refuerzo), ul, 3.3, 0.35, m5.len - 4.0, 1.5, 0.7);
    m5.caja(c.g(Mat::Luz), ul, 2.5, 0.55, m5.len - 4.4, 0.04, 0.3);
    for (k, col) in [
        v3(0.17, 0.72, 0.84),
        v3(0.47, 0.79, 0.29),
        v3(0.86, 0.62, 0.23),
    ]
    .iter()
    .enumerate()
    {
        let u = ul - 1.4 + k as f32 * 0.9;
        let h = 0.45 + 0.15 * k as f32;
        m5.caja(c.pintura(*col), u, 1.18 + h / 2.0, 0.85, 0.38, h, 0.38);
        m5.caja(c.g(Mat::Metal), u, 1.18 + h + 0.05, 0.85, 0.22, 0.08, 0.22);
    }
    m5.caja(c.g(Mat::Refuerzo), ul + 2.4, 1.45, 0.7, 0.8, 0.5, 0.5);
    m5.pantalla(c.g(Mat::Telemetria), ul + 2.4, 1.55, 0.96, 0.6, 0.34);
    c.luz(m5.p(ul, 2.2, 0.7), v3(1.0, 0.8, 0.5), 0.7, 5.5);
    let pl = m5.p(ul, 0.0, 0.8);
    obst.push(Obst {
        cx: pl.x,
        cz: pl.z,
        hx: (m5.len - 3.0) / 2.0,
        hz: 0.8,
        yaw: m5.yaw,
    });

    // ---------------- Costados: columnas, ductos, taquillas y pantalla ----------------
    for lado in [-1.0f32, 1.0] {
        for z in [-9.63f32, -6.06] {
            c.g(Mat::Refuerzo)
                .caja(v3(lado * 9.78, 2.95, z), v3(0.18, 5.9, 0.32));
            obst.push(Obst::caja(lado * 9.7, z, 0.4, 0.4));
        }
    }
    for (mi, ancho) in [(2usize, 9.4f32), (6, 9.4)] {
        m[mi].caja(
            c.g(Mat::MetalOscuro),
            ancho / 2.0,
            5.8,
            0.1,
            ancho,
            0.2,
            0.2,
        );
        m[mi].caja(c.g(Mat::Metal), ancho / 2.0, 6.15, 0.08, ancho, 0.1, 0.16);
        m[mi].caja(c.g(Mat::Cian), ancho / 2.0, 5.55, 0.02, ancho, 0.03, 0.03);
    }
    let m6 = m[6];
    for k in 0..8 {
        let u = 0.9 + k as f32 * 1.0;
        let col = if k % 2 == 0 {
            v3(0.33, 0.40, 0.48)
        } else {
            v3(0.28, 0.34, 0.42)
        };
        m6.caja(c.pintura(col), u, 1.0, 0.3, 0.92, 2.0, 0.6);
        m6.caja(c.g(Mat::Cian), u + 0.3, 1.2, 0.62, 0.05, 0.3, 0.02);
    }
    let p6 = m6.p(4.9, 0.0, 0.3);
    obst.push(Obst {
        cx: p6.x,
        cz: p6.z,
        hx: 4.5,
        hz: 0.33,
        yaw: m6.yaw,
    });
    let m2 = m[2];
    m2.caja(c.g(Mat::Refuerzo), 5.35, 3.2, 0.1, 3.3, 2.2, 0.2);
    m2.pantalla(c.g(Mat::Monitor), 5.35, 3.2, 0.21, 3.1, 2.0);

    // ---------------- Esclusa ----------------
    let al = ALTO_ESCLUSA;
    c.g(Mat::Piso)
        .rect_h(-2.0, 2.0, 8.0, FIN_ESCLUSA, 0.0, true);
    c.g(Mat::Peligro)
        .rect_h(-2.0, -1.62, 8.0, FIN_ESCLUSA, 0.006, true);
    c.g(Mat::Peligro)
        .rect_h(1.62, 2.0, 8.0, FIN_ESCLUSA, 0.006, true);
    c.g(Mat::Techo)
        .rect_h(-2.0, 2.0, 8.0, FIN_ESCLUSA, al, false);
    c.g(Mat::Pared)
        .pared((-2.0, 8.0), (-2.0, FIN_ESCLUSA), 0.0, al, (0.0, 11.0));
    c.g(Mat::Pared)
        .pared((2.0, 8.0), (2.0, FIN_ESCLUSA), 0.0, al, (0.0, 11.0));
    for z in [9.6f32, 12.4] {
        c.g(Mat::Luz)
            .caja(v3(0.0, al - 0.03, z), v3(1.6, 0.04, 0.4));
        c.luz(v3(0.0, al - 0.5, z), v3(1.0, 0.96, 0.9), 0.9, 7.0);
    }
    for z in [8.6f32, 10.0, 11.4, 12.8] {
        c.g(Mat::Refuerzo)
            .caja(v3(-1.94, al / 2.0, z), v3(0.12, al, 0.18));
        c.g(Mat::Refuerzo)
            .caja(v3(1.94, al / 2.0, z), v3(0.12, al, 0.18));
    }
    let me = Marco::de((2.0, FIN_ESCLUSA), (-2.0, FIN_ESCLUSA));
    me.pared(c.g(Mat::Pared), al, &[(0.8, 3.2, 0.0, 2.6)]);
    for u in [0.7f32, 3.3] {
        me.caja(c.g(Mat::Refuerzo), u, 1.65, 0.1, 0.3, 3.3, 0.4);
    }
    me.caja(c.g(Mat::Refuerzo), 2.0, 2.78, 0.1, 2.4, 0.36, 0.4);
    me.caja(c.g(Mat::Refuerzo), 2.0, 3.6, 0.1, 2.9, 0.3, 0.4);
    me.caja(c.g(Mat::Ambar), 2.0, 3.6, 0.31, 2.0, 0.07, 0.02);
    me.caja(c.g(Mat::Rojo), 0.4, 3.0, 0.31, 0.16, 0.16, 0.02);
    c.luz(v3(0.0, 2.6, 13.2), v3(1.0, 0.6, 0.2), 0.8, 4.5);
    let mr = Marco::de((2.0, 8.0), (2.0, FIN_ESCLUSA));
    mr.caja(c.g(Mat::Refuerzo), 3.0, 1.5, 0.12, 1.2, 1.7, 0.25);
    mr.pantalla(c.g(Mat::Telemetria), 3.0, 1.8, 0.26, 0.95, 0.6);
    mr.pantalla(c.g(Mat::Botonera), 3.0, 1.18, 0.26, 0.95, 0.5);
    mr.caja(c.g(Mat::Rojo), 3.0, 2.2, 0.26, 0.2, 0.08, 0.03);
    obst.push(Obst::caja(1.9, 11.0, 0.5, 1.4));
    let esclusa = mr.p(3.0, 1.0, 0.9);

    // Túnel de atraque
    c.g(Mat::Piso)
        .rect_h(-1.2, 1.2, FIN_ESCLUSA, FIN_TUNEL, 0.0, true);
    c.g(Mat::Techo)
        .rect_h(-1.2, 1.2, FIN_ESCLUSA, FIN_TUNEL, 2.7, false);
    c.g(Mat::Refuerzo).pared(
        (-1.2, FIN_ESCLUSA),
        (-1.2, FIN_TUNEL),
        0.0,
        2.7,
        (0.0, 16.0),
    );
    c.g(Mat::Refuerzo)
        .pared((1.2, FIN_ESCLUSA), (1.2, FIN_TUNEL), 0.0, 2.7, (0.0, 16.0));
    for z in [14.6f32, 15.6, 16.6, 17.6] {
        c.g(Mat::Metal).caja(v3(-1.15, 1.35, z), v3(0.15, 2.7, 0.3));
        c.g(Mat::Metal).caja(v3(1.15, 1.35, z), v3(0.15, 2.7, 0.3));
        c.g(Mat::Metal).caja(v3(0.0, 2.62, z), v3(2.4, 0.15, 0.3));
    }
    c.g(Mat::Refuerzo)
        .pared((-1.2, FIN_TUNEL), (1.2, FIN_TUNEL), 0.0, 2.7, (0.0, 16.0));
    let g = c.g(Mat::Cian);
    g.quad(
        [
            v3(-0.85, 0.15, FIN_TUNEL - 0.02),
            v3(0.85, 0.15, FIN_TUNEL - 0.02),
            v3(0.85, 2.35, FIN_TUNEL - 0.02),
            v3(-0.85, 2.35, FIN_TUNEL - 0.02),
        ],
        [[0.0, 0.0]; 4],
        v3(0.0, 0.0, -1.0),
    );
    c.luz(v3(0.0, 1.6, 16.8), v3(0.45, 0.85, 1.0), 1.0, 6.5);

    // ---------------- Colisiones ----------------
    let mundo = Mundo {
        zonas: vec![
            Zona::Poligono(CONTORNO.to_vec()),
            Zona::Rect {
                x0: -2.0,
                x1: 2.0,
                z0: 7.0,
                z1: FIN_ESCLUSA,
            },
            Zona::Rect {
                x0: -1.2,
                x1: 1.2,
                z0: FIN_ESCLUSA - 0.5,
                z1: FIN_TUNEL,
            },
        ],
        obst,
        dinamicos: vec![],
    };

    let luces = Arc::new(std::mem::take(&mut c.luces));
    let ambiente = v3(0.27, 0.29, 0.34);
    let partes = c.terminar(luces.clone(), ambiente);
    let puntos = Puntos {
        nucleo,
        pilotaje: m[0].p(4.2, 1.0, 1.7),
        esclusa,
        muelle: v3(0.0, 1.0, FIN_TUNEL - 0.8),
        planetario: v3(0.0, 1.45, 0.0),
        puerta_centro: m4.p(4.2, 2.25, -0.06),
        puerta_dir: v3(m4.d.0, 0.0, m4.d.1),
        puerta_yaw: m4.yaw,
        escotilla_centro: me.p(2.0, 1.3, -0.05),
        escotilla_dir: v3(me.d.0, 0.0, me.d.1),
        escotilla_yaw: me.yaw,
    };
    (
        Escenario {
            partes,
            mundo,
            luces,
            ambiente,
        },
        puntos,
    )
}
