// Carta de navegación 2D. Reutiliza el framebuffer del vuelo como fondo;
// los nodos bloqueados no muestran su nombre ni su ilustración.
use crate::progreso::{Galaxia, Progreso, GALAXIAS};
use crate::salto::Salto;
use raylib::prelude::*;

const POS: [(f32, f32); 4] = [(260.0, 235.0), (650.0, 350.0), (310.0, 525.0), (1010.0, 480.0)];
const CIAN: Color = Color::new(99, 226, 244, 255);
const TEXTO: Color = Color::new(227, 237, 244, 255);

#[derive(Default)]
pub struct Mapa { aviso_bloqueado: bool }

impl Mapa {
    pub fn cuadro(&mut self, window: &mut RaylibHandle, thread: &RaylibThread,
        progreso: &mut Progreso, salto: &Salto, tiempo: f32, exterior: Option<&Texture2D>, recien_abierto: bool) -> bool {
        let (sw, sh) = (window.get_screen_width() as f32, window.get_screen_height() as f32);
        let s = (sw / 1280.0).min(sh / 800.0);
        let (ox, oy) = ((sw - 1280.0 * s) * 0.5, (sh - 800.0 * s) * 0.5);
        let raton = window.get_mouse_position();
        let p = Vector2::new((raton.x - ox) / s, (raton.y - oy) / s);
        let clic = window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let cerrar = solicita_cierre(recien_abierto,
            window.is_key_pressed(KeyboardKey::KEY_M), window.is_key_pressed(KeyboardKey::KEY_X),
            window.is_key_pressed(KeyboardKey::KEY_ESCAPE),
            clic && p.x >= 990.0 && p.x <= 1210.0 && p.y >= 35.0 && p.y <= 75.0);
        if !cerrar && salto.solicitud().is_none() {
            for (i, tecla) in [KeyboardKey::KEY_ONE, KeyboardKey::KEY_TWO, KeyboardKey::KEY_THREE, KeyboardKey::KEY_FOUR].iter().enumerate() {
                let (x, y) = POS[i];
                let sobre = ((p.x - x) / 115.0).powi(2) + ((p.y - y) / 65.0).powi(2) <= 1.0;
                if (clic && sobre) || window.is_key_pressed(*tecla) {
                    let g = GALAXIAS[i];
                    if g != progreso.actual() { self.aviso_bloqueado = !progreso.seleccionar_ruta(g); }
                }
            }
        }
        let mut d = window.begin_drawing(thread);
        d.clear_background(Color::new(5, 8, 22, 255));
        if let Some(tex) = exterior {
            d.draw_texture_pro(tex, Rectangle::new(0.0, 0.0, tex.width as f32, tex.height as f32),
                Rectangle::new(ox, oy, 1280.0 * s, 800.0 * s), Vector2::zero(), 0.0, Color::new(90, 80, 150, 255));
        }
        let mut l = Carta { d: &mut d, s, ox, oy };
        l.rect(0.0, 0.0, 1280.0, 800.0, Color::new(4, 7, 24, 195));
        for i in 0..180 {
            let x = ((i * 157 + 91) % 1260) as f32 + 10.0;
            let y = ((i * 89 + 43) % 760) as f32 + 20.0;
            let a = (90.0 + 65.0 * (tiempo * 0.6 + i as f32).sin()) as u8;
            l.circulo(x, y, if i % 11 == 0 { 1.5 } else { 0.8 }, Color::new(200, 210, 255, a));
        }
        l.texto("MAPA INTERESTELAR", 52.0, 39.0, 28.0, TEXTO);
        l.texto(if progreso.mapa_activo() { "CARTA ACTIVA / SELECCIONA TU RUTA" }
            else { "CARTA BLOQUEADA / COMPLETA LA TERMINAL DE MARTE" }, 54.0, 82.0, 17.0, CIAN);
        l.rect(990.0, 35.0, 220.0, 40.0, Color::new(24, 39, 58, 255));
        l.texto("VOLVER [X / ESC]", 1010.0, 48.0, 17.0, TEXTO);

        for (a, b) in [(0, 1), (0, 2), (1, 2), (1, 3), (2, 3)] {
            let visible = progreso.desbloqueada(GALAXIAS[a]) && progreso.desbloqueada(GALAXIAS[b]);
            l.linea(POS[a], POS[b], if visible { CIAN } else { Color::new(37, 43, 59, 255) }, if visible { 3.0 } else { 1.0 });
        }
        for (i, g) in GALAXIAS.iter().enumerate() {
            let (x, y) = POS[i];
            let desbloqueada = progreso.desbloqueada(*g);
            let elegida = progreso.ruta() == Some(*g);
            l.elipse(x, y, 122.0, 66.0, Color::new(6, 11, 25, 255));
            if desbloqueada {
                // Espirales sencillas: puntos y brazos sobre una elipse.
                let color = if *g == Galaxia::Gargantua { Color::new(255, 188, 104, 255) } else { CIAN };
                for brazo in 0..3 {
                    for k in 0..100 {
                        let r = k as f32 / 99.0;
                        let ang = r * 8.0 + brazo as f32 * std::f32::consts::TAU / 3.0 + tiempo * 0.04;
                        l.circulo(x + ang.cos() * r * 102.0, y + ang.sin() * r * 47.0,
                            1.1 + (1.0 - r) * 1.7, Color::new(color.r, color.g, color.b, (220.0 - r * 130.0) as u8));
                    }
                }
                if *g == Galaxia::Gargantua {
                    l.elipse(x, y, 56.0, 15.0, Color::new(255, 187, 90, 220));
                    l.circulo(x, y - 3.0, 24.0, Color::BLACK);
                } else { l.elipse(x, y, 19.0, 9.0, Color::new(226, 239, 255, 255)); }
                l.texto(&format!("{}  {}", i + 1, g.nombre()), x - 110.0, y + 78.0, 18.0, if elegida { Color::new(139, 255, 192, 255) } else { TEXTO });
                l.texto(if *g == progreso.actual() { "UBICACION ACTUAL" } else if elegida { "RUTA TRAZADA" }
                    else if progreso.completada(*g) { "COMPLETADA" } else { "DISPONIBLE" }, x - 85.0, y + 103.0, 14.0, CIAN);
            } else {
                l.elipse(x, y, 109.0, 55.0, Color::BLACK);
                // Ningún nombre ni detalle del destino antes de desbloquearlo.
                l.texto("?", x - 10.0, y - 16.0, 32.0, Color::new(74, 82, 103, 255));
                l.texto("SIN DATOS", x - 48.0, y + 78.0, 16.0, Color::new(97, 108, 131, 255));
            }
            if elegida {
                l.linea((x - 123.0, y - 69.0), (x - 93.0, y - 69.0), CIAN, 3.0);
                l.linea((x - 123.0, y - 69.0), (x - 123.0, y - 42.0), CIAN, 3.0);
                l.linea((x + 123.0, y + 69.0), (x + 93.0, y + 69.0), CIAN, 3.0);
                l.linea((x + 123.0, y + 69.0), (x + 123.0, y + 42.0), CIAN, 3.0);
            }
        }
        l.rect(42.0, 670.0, 1196.0, 100.0, Color::new(8, 17, 32, 245));
        if let Some(solicitud) = salto.solicitud() {
            l.texto(&format!("MOTOR DE SALTO / {}", solicitud.destino.nombre()), 64.0, 688.0, 21.0, CIAN);
            l.rect(64.0, 727.0, 620.0, 7.0, Color::new(37, 53, 67, 255));
            l.rect(64.0, 727.0, 620.0 * salto.carga(), 7.0, CIAN);
            l.texto(if salto.listo() && !solicitud.destino.implementada() { "Escena de destino pendiente." }
                else if salto.listo() { "Enlace listo. Preparando llegada..." }
                else { "Sincronizando el motor desbloqueado en Marte..." }, 720.0, 715.0, 17.0, TEXTO);
        } else if !progreso.mapa_activo() {
            l.texto("TERMINAL DE MARTE PENDIENTE", 64.0, 690.0, 21.0, TEXTO);
            l.texto("Completa las cuatro transiciones para activar el mapa y el motor de salto.", 64.0, 728.0, 18.0, CIAN);
        } else {
            l.texto(&format!("DESTINO: {}", progreso.ruta().map(|g| g.nombre()).unwrap_or("SELECCIONA UNA RUTA")), 64.0, 689.0, 21.0, CIAN);
            l.texto(if self.aviso_bloqueado { "Destino bloqueado. Avanza en las misiones para revelar su identidad." }
                else { "Vuela hasta el limite del sistema para iniciar la preparacion del salto." }, 64.0, 728.0, 18.0, TEXTO);
        }
        l.texto("CLIC o 1-4: trazar ruta   /   M, X o ESC: volver al vuelo", 52.0, 780.0, 14.0, TEXTO);
        // El handle termina el cuadro incluso al cerrar. Sin EndDrawing la
        // pulsación seguiría activa y Escape cerraría también el juego.
        cerrar
    }
}

fn solicita_cierre(recien_abierto: bool, m: bool, x: bool, escape: bool, clic: bool) -> bool {
    x || escape || clic || (m && !recien_abierto)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pulsacion_que_abre_el_mapa_no_lo_cierra_en_el_mismo_cuadro() {
        assert!(!solicita_cierre(true, true, false, false, false));
        assert!(!solicita_cierre(false, false, false, false, false));
        assert!(solicita_cierre(false, true, false, false, false));
        assert!(solicita_cierre(false, false, true, false, false));
        assert!(solicita_cierre(false, false, false, true, false));
        assert!(solicita_cierre(false, false, false, false, true));
    }
}

struct Carta<'a, D: RaylibDraw> { d: &'a mut D, s: f32, ox: f32, oy: f32 }
impl<D: RaylibDraw> Carta<'_, D> {
    fn punto(&self, x: f32, y: f32) -> Vector2 { Vector2::new(self.ox + x * self.s, self.oy + y * self.s) }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        self.d.draw_rectangle_rec(Rectangle::new(self.ox + x * self.s, self.oy + y * self.s, w * self.s, h * self.s), c);
    }
    fn texto(&mut self, t: &str, x: f32, y: f32, tam: f32, c: Color) {
        self.d.draw_text(t, (self.ox + x * self.s) as i32, (self.oy + y * self.s) as i32, (tam * self.s).max(1.0) as i32, c);
    }
    fn circulo(&mut self, x: f32, y: f32, r: f32, c: Color) { self.d.draw_circle_v(self.punto(x, y), r * self.s, c); }
    fn elipse(&mut self, x: f32, y: f32, rx: f32, ry: f32, c: Color) {
        let p = self.punto(x, y);
        self.d.draw_ellipse(p.x as i32, p.y as i32, rx * self.s, ry * self.s, c);
    }
    fn linea(&mut self, a: (f32, f32), b: (f32, f32), c: Color, ancho: f32) {
        self.d.draw_line_ex(self.punto(a.0, a.1), self.punto(b.0, b.1), ancho * self.s, c);
    }
}
