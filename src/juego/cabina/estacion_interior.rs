// Interior de la estación espacial: núcleo circular, cuatro brazos y anillo exterior,
// con la misma paleta (casco gris, tiras cian) que el OBJ exterior. Aquí se recogen los materiales.
use crate::construccion::*;
use crate::dibujo_interior::Escenario;
use crate::jugador::{Mundo, Obst, Zona};
use crate::materiales::Mat;
use crate::math::*;
use crate::nave_interior::monitor;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::sync::Arc;

pub const R_HUB: f32 = 6.0;
pub const R0: f32 = 15.6;
pub const R1: f32 = 18.4;
pub const H: f32 = 3.6;
pub const SEMI_BRAZO: f32 = 1.4;
pub const FIN_TUBO: f32 = 23.0;

pub struct Recogible {
    pub pos: V3,
}

pub struct PuntosEstacion {
    pub recogibles: Vec<V3>,      // 5 materiales, en el orden de `Material`
    pub nave: V3,                 // puerta de regreso a la nave
    pub llegada: (f32, f32, f32), // x, z, yaw al salir de la nave
}

fn rot(p: (f32, f32), a: f32) -> (f32, f32) {
    (p.0 * a.cos() - p.1 * a.sin(), p.0 * a.sin() + p.1 * a.cos())
}

// Distancia perpendicular al eje a·(1,0) si el punto está en ese semieje; si no, infinito.
fn perp_eje(am: f32, r: f32) -> f32 {
    let mut mejor = f32::INFINITY;
    for k in 0..4 {
        let d = am - k as f32 * FRAC_PI_2;
        if d.cos() > 0.0 {
            mejor = mejor.min((r * d.sin()).abs());
        }
    }
    mejor
}

fn cadena(c: &mut Constructor, a: f32, r: f32) -> Marco {
    let d = 0.2;
    let (p0, p1) = (
        (r * (a - d).cos(), r * (a - d).sin()),
        (r * (a + d).cos(), r * (a + d).sin()),
    );
    let _ = c;
    Marco::de(p0, p1)
}

fn caja_pintada(c: &mut Constructor, col: V3, cx: f32, cz: f32, w: f32, h: f32, d: f32, yaw: f32) {
    c.pintura(col)
        .caja_rot(v3(cx, h / 2.0, cz), v3(w, h, d), yaw);
    c.g(Mat::Metal)
        .caja_rot(v3(cx, h * 0.5, cz), v3(w + 0.02, 0.06, d + 0.02), yaw);
}

pub fn construir() -> (Escenario, PuntosEstacion) {
    let mut c = Constructor::nuevo();
    let mut obst: Vec<Obst> = vec![];
    let o = V3::ZERO;

    // ---------------- Núcleo (hub) ----------------
    c.g(Mat::Pared).cilindro(o, R_HUB, 48, true, &|_, am| {
        if perp_eje(am, R_HUB) < 1.5 {
            vec![]
        } else {
            vec![(0.0, H)]
        }
    });
    c.g(Mat::Cian)
        .cilindro(o, R_HUB - 0.02, 48, true, &|_, am| {
            if perp_eje(am, R_HUB) < 1.5 {
                vec![]
            } else {
                vec![(1.0, 1.1)]
            }
        });
    c.g(Mat::Piso).anillo_h(o, 0.0, R_HUB, 0.0, 48, true);
    c.g(Mat::Techo).anillo_h(o, 0.0, R_HUB, H, 48, false);
    c.g(Mat::Cian)
        .anillo_h(v3(0.0, 0.0, 0.0), 2.6, 2.7, 0.012, 48, true);
    c.g(Mat::Luz).anillo_h(o, 1.6, 2.0, H - 0.02, 48, false);
    c.luz(v3(0.0, 3.0, 0.0), v3(0.95, 0.97, 1.0), 0.8, 10.0);
    // mesa holográfica central
    c.g(Mat::MetalOscuro)
        .caja(v3(0.0, 0.3, 0.0), v3(1.2, 0.6, 1.0));
    c.g(Mat::Metal).caja(v3(0.0, 0.64, 0.0), v3(2.4, 0.1, 1.7));
    c.g(Mat::Holo).quad(
        [
            v3(-1.1, 0.7, -0.75),
            v3(1.1, 0.7, -0.75),
            v3(1.1, 0.7, 0.75),
            v3(-1.1, 0.7, 0.75),
        ],
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        v3(0.0, 1.0, 0.0),
    );
    obst.push(Obst::caja(0.0, 0.0, 2.5, 1.8));
    c.luz(v3(0.0, 1.0, 0.0), v3(0.3, 0.7, 1.0), 0.7, 3.5);
    // consolas contra la pared, entre las puertas
    let mut consolas = vec![];
    for k in 0..4 {
        let a = PI / 4.0 + k as f32 * FRAC_PI_2;
        let m = cadena(&mut c, a, R_HUB - 0.15);
        let uc = m.len / 2.0;
        m.caja(c.g(Mat::MetalOscuro), uc, 0.5, 0.55, 1.9, 1.0, 1.1);
        m.caja(c.g(Mat::Metal), uc, 1.0, 0.55, 2.0, 0.1, 1.15);
        m.caja(c.g(Mat::Cian), uc, 0.8, 1.11, 1.5, 0.03, 0.03);
        let mat = [Mat::Radar, Mat::Telemetria, Mat::Monitor, Mat::Telemetria][k];
        monitor(&mut c, &m, uc, 1.7, mat);
        let p = m.p(uc, 0.0, 0.55);
        obst.push(Obst {
            cx: p.x,
            cz: p.z,
            hx: 1.0,
            hz: 0.6,
            yaw: m.yaw,
        });
        consolas.push(m.p(uc, 1.0, 0.7));
    }
    // marcos de puerta del núcleo
    for k in 0..4 {
        let a = k as f32 * FRAC_PI_2;
        for lado in [-1.0f32, 1.0] {
            let p = rot((R_HUB - 0.05, lado * 1.5), a);
            c.g(Mat::Refuerzo)
                .caja_rot(v3(p.0, H / 2.0, p.1), v3(0.4, H, 0.3), -a);
        }
        let p = rot((R_HUB - 0.05, 0.0), a);
        c.g(Mat::Refuerzo)
            .caja_rot(v3(p.0, H - 0.3, p.1), v3(0.4, 0.6, 3.3), -a);
        c.g(Mat::Cian).caja_rot(
            v3(p.0 - 0.2 * a.cos(), H - 0.65, p.1 - 0.2 * a.sin()),
            v3(0.05, 0.05, 2.6),
            -a,
        );
    }

    // ---------------- Brazos ----------------
    for k in 0..4 {
        let a = k as f32 * FRAC_PI_2;
        let (a0, a1) = (R_HUB - 0.5, R0 + 0.3);
        let (m1, m2) = (
            Marco::de(rot((a0, -SEMI_BRAZO), a), rot((a1, -SEMI_BRAZO), a)),
            Marco::de(rot((a1, SEMI_BRAZO), a), rot((a0, SEMI_BRAZO), a)),
        );
        m1.pared(c.g(Mat::Pared), H, &[]);
        m2.pared(c.g(Mat::Pared), H, &[]);
        let pts: Vec<(f32, f32)> = [
            (a0, -SEMI_BRAZO),
            (a1, -SEMI_BRAZO),
            (a1, SEMI_BRAZO),
            (a0, SEMI_BRAZO),
        ]
        .iter()
        .map(|&p| rot(p, a))
        .collect();
        c.g(Mat::Piso).poligono_h(&pts, 0.0, true);
        c.g(Mat::Techo).poligono_h(&pts, H, false);
        // tira cian en el suelo y en la pared
        for m in [m1, m2] {
            m.caja(
                c.g(Mat::Cian),
                m.len / 2.0,
                1.05,
                0.02,
                m.len - 0.4,
                0.05,
                0.03,
            );
        }
        // luz de techo y conductos
        let pm = rot(((a0 + a1) / 2.0, 0.0), a);
        c.g(Mat::Luz)
            .caja_rot(v3(pm.0, H - 0.02, pm.1), v3(1.8, 0.04, 0.5), -a);
        c.luz(v3(pm.0, 3.0, pm.1), v3(0.9, 0.95, 1.0), 0.6, 9.0);
        for lado in [-1.0f32, 1.0] {
            let p = rot(((a0 + a1) / 2.0, lado * 1.25), a);
            c.g(Mat::MetalOscuro)
                .caja_rot(v3(p.0, H - 0.3, p.1), v3(a1 - a0 - 0.5, 0.2, 0.2), -a);
        }
        // marcos de puerta con el anillo
        for lado in [-1.0f32, 1.0] {
            let p = rot((R0 + 0.1, lado * 1.5), a);
            c.g(Mat::Refuerzo)
                .caja_rot(v3(p.0, H / 2.0, p.1), v3(0.4, H, 0.3), -a);
        }
    }

    // ---------------- Anillo exterior ----------------
    let hueco_tubo = |am: f32| am.cos() > 0.0 && (R1 * am.sin()).abs() < 1.3;
    c.g(Mat::Pared).cilindro(o, R0, 96, false, &|_, am| {
        if perp_eje(am, R0) < 1.5 {
            vec![]
        } else {
            vec![(0.0, H)]
        }
    });
    c.g(Mat::Pared).cilindro(o, R1, 96, true, &|k, am| {
        if hueco_tubo(am) {
            vec![]
        } else if (2..6).contains(&(k % 8)) {
            vec![(0.0, 1.0), (2.6, H)]
        } else {
            vec![(0.0, H)]
        }
    });
    c.g(Mat::Metal).cilindro(o, R1 - 0.03, 96, true, &|k, am| {
        if hueco_tubo(am) || !(2..6).contains(&(k % 8)) {
            vec![]
        } else {
            vec![(1.0, 1.08), (2.52, 2.6)]
        }
    });
    c.g(Mat::MetalOscuro)
        .cilindro(o, R1 - 0.06, 96, true, &|k, am| {
            if hueco_tubo(am) || !(k % 8 == 2 || k % 8 == 6) {
                vec![]
            } else {
                vec![(1.0, 2.6)]
            }
        });
    c.g(Mat::Cian).cilindro(o, R0 + 0.02, 96, false, &|_, am| {
        if perp_eje(am, R0) < 1.5 {
            vec![]
        } else {
            vec![(1.0, 1.1)]
        }
    });
    c.g(Mat::Piso).anillo_h(o, R0, R1, 0.0, 96, true);
    c.g(Mat::Techo).anillo_h(o, R0, R1, H, 96, false);
    c.g(Mat::Cian)
        .anillo_h(o, R0 + 0.45, R0 + 0.5, 0.012, 96, true);
    c.g(Mat::Cian)
        .anillo_h(o, R1 - 0.5, R1 - 0.45, 0.012, 96, true);
    for k in 0..12 {
        let a = k as f32 * TAU / 12.0 + TAU / 24.0;
        let (x, z) = (17.0 * a.cos(), 17.0 * a.sin());
        c.g(Mat::Luz)
            .caja_rot(v3(x, H - 0.02, z), v3(0.5, 0.04, 1.8), -a);
        c.luz(v3(x, 3.0, z), v3(0.92, 0.96, 1.0), 0.55, 8.5);
    }

    // ---------------- Tubo de atraque (conecta con la nave) ----------------
    let tw = 1.2;
    c.g(Mat::Piso)
        .rect_h(R1 - 0.4, FIN_TUBO, -tw, tw, 0.0, true);
    c.g(Mat::Techo)
        .rect_h(R1 - 0.4, FIN_TUBO, -tw, tw, 3.0, false);
    c.g(Mat::Peligro)
        .rect_h(R1 - 0.4, FIN_TUBO, -tw, -tw + 0.35, 0.006, true);
    c.g(Mat::Peligro)
        .rect_h(R1 - 0.4, FIN_TUBO, tw - 0.35, tw, 0.006, true);
    c.g(Mat::Refuerzo)
        .pared((R1 - 0.4, -tw), (FIN_TUBO, -tw), 0.0, 3.0, (20.0, 0.0));
    c.g(Mat::Refuerzo)
        .pared((R1 - 0.4, tw), (FIN_TUBO, tw), 0.0, 3.0, (20.0, 0.0));
    c.g(Mat::Refuerzo)
        .pared((FIN_TUBO, -tw), (FIN_TUBO, tw), 0.0, 3.0, (20.0, 0.0));
    for x in [19.4f32, 20.6, 21.8] {
        c.g(Mat::Metal).caja(v3(x, 1.5, -1.15), v3(0.3, 3.0, 0.15));
        c.g(Mat::Metal).caja(v3(x, 1.5, 1.15), v3(0.3, 3.0, 0.15));
        c.g(Mat::Metal).caja(v3(x, 2.92, 0.0), v3(0.3, 0.15, 2.4));
    }
    let g = c.g(Mat::Cian);
    g.quad(
        [
            v3(FIN_TUBO - 0.02, 0.15, -0.85),
            v3(FIN_TUBO - 0.02, 0.15, 0.85),
            v3(FIN_TUBO - 0.02, 2.6, 0.85),
            v3(FIN_TUBO - 0.02, 2.6, -0.85),
        ],
        [[0.0, 0.0]; 4],
        v3(-1.0, 0.0, 0.0),
    );
    c.g(Mat::Ambar)
        .caja(v3(FIN_TUBO - 0.05, 2.8, 0.0), v3(0.06, 0.1, 1.8));
    c.luz(v3(21.5, 2.4, 0.0), v3(0.5, 0.85, 1.0), 0.9, 6.0);

    // ---------------- Cajas, estantes y materiales ----------------
    let marron = v3(0.52, 0.38, 0.22);
    let azul = v3(0.25, 0.35, 0.5);
    let verde = v3(0.25, 0.42, 0.33);
    let mut recog = vec![];
    // 1: celda de energía sobre la consola del núcleo (NE)
    recog.push(consolas[0] + v3(0.0, 0.32, 0.0));
    // 2: cable de fibra sobre cajas en el brazo -Z
    caja_pintada(&mut c, marron, -0.6, -11.0, 0.9, 0.8, 0.9, 0.2);
    obst.push(Obst::caja(-0.6, -11.0, 1.0, 1.0));
    caja_pintada(&mut c, azul, 0.6, -12.2, 0.8, 0.6, 0.8, -0.3);
    obst.push(Obst::caja(0.6, -12.2, 0.9, 0.9));
    recog.push(v3(-0.6, 1.2, -11.0));
    // 3: cristal de navegación sobre un pedestal en el anillo (135°)
    let a3 = 0.75 * PI;
    let p3 = (17.45 * a3.cos(), 17.45 * a3.sin());
    c.g(Mat::MetalOscuro)
        .caja_rot(v3(p3.0, 0.45, p3.1), v3(0.8, 0.9, 0.8), -a3);
    c.g(Mat::Cian)
        .caja_rot(v3(p3.0, 0.92, p3.1), v3(0.9, 0.04, 0.9), -a3);
    obst.push(Obst::caja(p3.0, p3.1, 1.0, 1.0));
    recog.push(v3(p3.0, 1.35, p3.1));
    c.luz(v3(p3.0, 1.6, p3.1), v3(0.4, 0.9, 1.0), 0.5, 4.0);
    // 4: tanque de oxígeno entre cajas del anillo (225°)
    let a4 = 1.25 * PI;
    let p4 = (17.4 * a4.cos(), 17.4 * a4.sin());
    caja_pintada(&mut c, verde, p4.0, p4.1, 0.9, 0.85, 0.9, -a4);
    obst.push(Obst::caja(p4.0, p4.1, 1.0, 1.0));
    recog.push(v3(p4.0, 1.25, p4.1));
    // 5: chip cuántico sobre cajas en el brazo +Z
    caja_pintada(&mut c, azul, 0.6, 10.8, 0.9, 0.8, 0.9, 0.4);
    obst.push(Obst::caja(0.6, 10.8, 1.0, 1.0));
    caja_pintada(&mut c, marron, -0.7, 12.0, 0.8, 0.55, 0.8, 0.0);
    obst.push(Obst::caja(-0.7, 12.0, 0.9, 0.9));
    recog.push(v3(0.6, 1.2, 10.8));
    // decorado: cajas en el anillo y brazo -X, con taquillas
    for (ang, col) in [
        (0.35 * PI, marron),
        (1.7 * PI, azul),
        (1.05 * PI, verde),
        (1.5 * PI, marron),
    ] {
        let (x, z) = (17.3 * ang.cos(), 17.3 * ang.sin());
        caja_pintada(&mut c, col, x, z, 0.9, 0.9, 0.9, -ang);
        caja_pintada(
            &mut c,
            azul,
            x + 0.2 * ang.sin(),
            z - 0.2 * ang.cos(),
            0.7,
            0.6,
            0.7,
            0.5,
        );
        obst.push(Obst::caja(x, z, 1.2, 1.2));
    }
    for k in 0..6 {
        let x = -8.0 - k as f32 * 1.1;
        let col = if k % 2 == 0 {
            v3(0.33, 0.40, 0.48)
        } else {
            v3(0.28, 0.34, 0.42)
        };
        c.pintura(col).caja(v3(x, 1.0, -1.1), v3(0.95, 2.0, 0.5));
        c.g(Mat::Cian)
            .caja(v3(x + 0.3, 1.2, -0.84), v3(0.05, 0.3, 0.02));
    }
    obst.push(Obst::caja(-10.9, -1.12, 6.8, 0.55));

    // ---------------- Colisiones ----------------
    let mut zonas = vec![
        Zona::Disco {
            cx: 0.0,
            cz: 0.0,
            r: R_HUB,
        },
        Zona::Anillo {
            cx: 0.0,
            cz: 0.0,
            r0: R0,
            r1: R1,
        },
        Zona::Rect {
            x0: R1 - 0.6,
            x1: FIN_TUBO,
            z0: -tw,
            z1: tw,
        },
    ];
    zonas.push(Zona::Rect {
        x0: 5.0,
        x1: R0 + 0.5,
        z0: -SEMI_BRAZO,
        z1: SEMI_BRAZO,
    });
    zonas.push(Zona::Rect {
        x0: -R0 - 0.5,
        x1: -5.0,
        z0: -SEMI_BRAZO,
        z1: SEMI_BRAZO,
    });
    zonas.push(Zona::Rect {
        x0: -SEMI_BRAZO,
        x1: SEMI_BRAZO,
        z0: 5.0,
        z1: R0 + 0.5,
    });
    zonas.push(Zona::Rect {
        x0: -SEMI_BRAZO,
        x1: SEMI_BRAZO,
        z0: -R0 - 0.5,
        z1: -5.0,
    });
    let mundo = Mundo {
        zonas,
        obst,
        dinamicos: vec![],
    };

    let luces = Arc::new(std::mem::take(&mut c.luces));
    let ambiente = v3(0.22, 0.25, 0.31);
    let partes = c.terminar(luces.clone(), ambiente);
    let puntos = PuntosEstacion {
        recogibles: recog,
        nave: v3(FIN_TUBO - 0.9, 1.0, 0.0),
        llegada: (FIN_TUBO - 1.2, 0.0, -FRAC_PI_2),
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
