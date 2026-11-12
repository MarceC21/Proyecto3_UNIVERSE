//! Paneles opcionales del vuelo. No interrumpen la simulación ni cambian los controles.
use crate::{
    dibujo::{Ajustes, Stats},
    nave::Nave,
};
use raylib::prelude::*;
#[derive(Clone, Copy, Default, PartialEq)]
pub enum PanelHud {
    #[default]
    Ninguno,
    Estadisticas,
    Controles,
}
impl PanelHud {
    pub fn alternar(&mut self, panel: Self) {
        *self = if *self == panel { Self::Ninguno } else { panel };
    }
    pub fn dibujar(
        &self,
        d: &mut RaylibDrawHandle,
        fps: u32,
        nave: &Nave,
        st: &Stats,
        aj: &Ajustes,
        camara_libre: bool,
    ) {
        etiqueta(
            d,
            "[F4] Estadisticas   [F5] Controles",
            16,
            20,
            17,
            Color::new(153, 227, 239, 255),
        );
        if *self == Self::Ninguno {
            return;
        }
        let (sw, sh) = (d.get_screen_width() as f32, d.get_screen_height() as f32);
        let s = (sw / 1280.0).min(sh / 800.0).min(1.0);
        let (w, h) = if *self == Self::Controles {
            (900.0, 580.0)
        } else {
            (690.0, 310.0)
        };
        let (x, y) = ((sw - w * s) * 0.5, (sh - h * s) * 0.5);
        d.draw_rectangle_rec(
            Rectangle::new(x, y, w * s, h * s),
            Color::new(4, 13, 23, 252),
        );
        d.draw_rectangle_lines(
            x as i32,
            y as i32,
            (w * s) as i32,
            (h * s) as i32,
            Color::new(78, 155, 175, 255),
        );
        let mut texto = |t: &str, px: f32, py: f32, tam: f32, c: Color| {
            d.draw_text(
                t,
                (x + px * s) as i32,
                (y + py * s) as i32,
                (tam * s).max(1.0) as i32,
                c,
            )
        };
        let cian = Color::new(153, 227, 239, 255);
        texto(
            if *self == Self::Controles {
                "CONTROLES DE VUELO"
            } else {
                "ESTADISTICAS"
            },
            24.0,
            20.0,
            26.0,
            cian,
        );
        if *self == Self::Estadisticas {
            for (i, linea) in [
                format!("FPS: {fps}"),
                format!(
                    "Velocidad: {:.1} / {:.0}",
                    nave.velocidad(),
                    nave.velocidad_max
                ),
                format!(
                    "Objetos visibles: {} / {}",
                    st.objetos_enviados, st.objetos_total
                ),
                format!(
                    "Triangulos enviados: {} / {} sin optimizaciones",
                    st.tris_enviados, st.tris_sin_optimizar
                ),
                format!(
                    "Culling: {}    LOD: {}",
                    if aj.culling { "ON" } else { "OFF" },
                    if aj.lod { "ON" } else { "OFF" }
                ),
                format!(
                    "Orbitas: {}    Frontera: {}    Camara: {}",
                    if aj.orbitas { "ON" } else { "OFF" },
                    if aj.limite { "ON" } else { "OFF" },
                    if camara_libre { "libre" } else { "nave" }
                ),
            ]
            .iter()
            .enumerate()
            {
                texto(linea, 24.0, 76.0 + i as f32 * 29.0, 19.0, Color::WHITE);
            }
            texto(
                "F4 / ESC cerrar    F5 ver controles",
                24.0,
                270.0,
                18.0,
                cian,
            );
        } else {
            for (i, (tecla, accion)) in crate::menu::CONTROLES.iter().enumerate() {
                texto(tecla, 26.0, 70.0 + i as f32 * 27.0, 19.0, cian);
                texto(accion, 250.0, 70.0 + i as f32 * 27.0, 19.0, Color::WHITE);
            }
            texto(
                "C: culling   L: LOD   O: orbitas   B: frontera",
                26.0,
                497.0,
                18.0,
                Color::LIGHTGRAY,
            );
            texto(
                "F5 / ESC cerrar    F4 ver estadisticas",
                26.0,
                541.0,
                18.0,
                cian,
            );
        }
    }
}
// Medición real de la fuente, ajuste de líneas y fondo opaco para avisos del juego.
pub fn etiqueta(d: &mut RaylibDrawHandle, t: &str, x: i32, y: i32, tam: i32, color: Color) {
    let maximo = (d.get_screen_width() - 32).max(1);
    let mut lineas = vec![];
    let mut linea = String::new();
    for palabra in t.split_whitespace() {
        let candidato = if linea.is_empty() {
            palabra.to_string()
        } else {
            format!("{linea} {palabra}")
        };
        if !linea.is_empty() && d.measure_text(&candidato, tam) > maximo - 20 {
            lineas.push(linea);
            linea = palabra.into();
        } else {
            linea = candidato;
        }
    }
    if !linea.is_empty() {
        lineas.push(linea);
    }
    let ancho = (lineas
        .iter()
        .map(|t| d.measure_text(t, tam))
        .max()
        .unwrap_or(0)
        + 20)
        .min(maximo);
    let alto = lineas.len() as i32 * (tam + 4) + 12;
    let x = (x - 8).clamp(8, (d.get_screen_width() - ancho - 8).max(8));
    let y = (y - 6).clamp(8, (d.get_screen_height() - alto - 8).max(8));
    d.draw_rectangle(x, y, ancho, alto, Color::new(4, 13, 23, 245));
    d.draw_rectangle_lines(x, y, ancho, alto, Color::new(37, 75, 90, 255));
    for (i, linea) in lineas.iter().enumerate() {
        d.draw_text(linea, x + 10, y + 6 + i as i32 * (tam + 4), tam, color);
    }
}
