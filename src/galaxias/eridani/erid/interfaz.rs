//! Presentación del epílogo. EndDrawing siempre consume las teclas al salir.
use crate::{
    camara::Camara,
    elenco_erid::{camara_descenso, PlayaErid},
    final_erid::{Fase, FinalErid, DIALOGOS},
    jugador::EntradaJ,
    nave::Nave,
    raster::Framebuffer,
};
use raylib::prelude::*;
pub struct SuperficieErid {
    pub activa: bool,
    pub mision: Option<FinalErid>,
    escena: Option<PlayaErid>,
    fb: Framebuffer,
    tex: Option<Texture2D>,
    mouse: bool,
}
impl Default for SuperficieErid {
    fn default() -> Self {
        Self {
            activa: false,
            mision: None,
            escena: None,
            fb: Framebuffer::new(720, 450),
            tex: None,
            mouse: true,
        }
    }
}
impl SuperficieErid {
    pub fn entrar(&mut self, completado: bool) {
        if self.escena.is_none() {
            self.escena = Some(PlayaErid::nueva());
        }
        self.mision = Some(FinalErid::nuevo(completado));
        self.activa = true;
        self.mouse = true;
    }
    pub fn completado(&self) -> bool {
        self.mision.as_ref().is_some_and(|m| m.completado())
    }
    pub fn cuadro(
        &mut self,
        w: &mut RaylibHandle,
        thread: &RaylibThread,
        nave: &Nave,
        dt: f32,
    ) -> bool {
        let salir =
            w.is_key_pressed(KeyboardKey::KEY_X) || w.is_key_pressed(KeyboardKey::KEY_ESCAPE);
        if w.is_key_pressed(KeyboardKey::KEY_TAB) {
            self.mouse = !self.mouse;
        }
        for (key, x, y) in [
            (KeyboardKey::KEY_F1, 960, 600),
            (KeyboardKey::KEY_F2, 720, 450),
            (KeyboardKey::KEY_F3, 480, 300),
        ] {
            if w.is_key_pressed(key) && self.fb.width != x {
                self.fb.redimensionar(x, y);
                self.tex = None;
            }
        }
        let m = self.mision.as_mut().unwrap();
        let fase_entrada = m.fase;
        let modal = m.fase != Fase::Libre;
        if modal || !self.mouse {
            w.enable_cursor();
        } else {
            w.disable_cursor();
        }
        let eje = |a, b| (w.is_key_down(a) as i32 - w.is_key_down(b) as i32) as f32;
        let mouse = if !modal && self.mouse {
            w.get_mouse_delta()
        } else {
            Vector2::zero()
        };
        let entrada = EntradaJ {
            adelante: eje(KeyboardKey::KEY_W, KeyboardKey::KEY_S),
            lateral: eje(KeyboardKey::KEY_D, KeyboardKey::KEY_A),
            giro_x: mouse.x + 420.0 * dt * eje(KeyboardKey::KEY_RIGHT, KeyboardKey::KEY_LEFT),
            giro_y: mouse.y - 420.0 * dt * eje(KeyboardKey::KEY_UP, KeyboardKey::KEY_DOWN),
            correr: w.is_key_down(KeyboardKey::KEY_LEFT_SHIFT),
        };
        m.actualizar(dt, &entrada);
        // No saltar la primera línea por la tecla que acaba de terminar el descenso.
        if fase_entrada == m.fase
            && (w.is_key_pressed(KeyboardKey::KEY_E) || w.is_key_pressed(KeyboardKey::KEY_ENTER))
        {
            m.continuar();
        }
        let mut cam = Camara::nueva(self.fb.width as f32, self.fb.height as f32);
        if m.fase == Fase::Descenso {
            cam = camara_descenso(self.fb.width, self.fb.height, m.descenso);
        } else {
            m.jugador.camara(&mut cam);
        }
        self.escena
            .as_ref()
            .unwrap()
            .dibujar_vivo(&mut self.fb, &cam, nave, m);
        if self.tex.is_none() {
            let img =
                Image::gen_image_color(self.fb.width as i32, self.fb.height as i32, Color::BLACK);
            let tex = w
                .load_texture_from_image(thread, &img)
                .expect("framebuffer Erid");
            tex.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
            self.tex = Some(tex);
        }
        let tex = self.tex.as_mut().unwrap();
        tex.update_texture(&self.fb.pixels).unwrap();
        let (sw, sh) = (w.get_screen_width() as f32, w.get_screen_height() as f32);
        let s = (sw / 1280.0).min(sh / 800.0);
        let (ox, oy) = ((sw - 1280.0 * s) * 0.5, (sh - 800.0 * s) * 0.5);
        let mut d = w.begin_drawing(thread);
        d.clear_background(Color::BLACK);
        d.draw_texture_pro(
            tex,
            Rectangle::new(0.0, 0.0, self.fb.width as f32, self.fb.height as f32),
            Rectangle::new(ox, oy, 1280.0 * s, 800.0 * s),
            Vector2::zero(),
            0.0,
            Color::WHITE,
        );
        let rect = |d: &mut RaylibDrawHandle, x: f32, y: f32, wi: f32, h: f32, c| {
            d.draw_rectangle_rec(Rectangle::new(ox + x * s, oy + y * s, wi * s, h * s), c)
        };
        let texto = |d: &mut RaylibDrawHandle, t: &str, x: f32, y: f32, tam: f32, c| {
            d.draw_text(
                t,
                (ox + x * s) as i32,
                (oy + y * s) as i32,
                (tam * s).max(1.0) as i32,
                c,
            )
        };
        let panel = Color::new(8, 24, 34, 230);
        let cian = Color::new(130, 239, 223, 255);
        rect(&mut d, 20.0, 20.0, 800.0, 65.0, panel);
        texto(&mut d, "ERID / HOGAR DE ROCKY", 40.0, 38.0, 27.0, cian);
        match m.fase {
            Fase::Descenso => {
                rect(&mut d, 0.0, 700.0, 1280.0, 100.0, Color::BLACK);
                texto(
                    &mut d,
                    "ATERRIZANDO EN LA BAHIA...",
                    65.0,
                    735.0,
                    25.0,
                    Color::WHITE,
                );
            }
            Fase::Dialogo(i) => {
                rect(&mut d, 90.0, 535.0, 1100.0, 205.0, panel);
                texto(&mut d, DIALOGOS[i].0, 120.0, 555.0, 27.0, cian);
                let mut linea = String::new();
                let mut fila = 0;
                for palabra in DIALOGOS[i].1.split_whitespace() {
                    if linea.chars().count() + palabra.chars().count() > 73 {
                        texto(
                            &mut d,
                            &linea,
                            120.0,
                            598.0 + fila as f32 * 30.0,
                            23.0,
                            Color::WHITE,
                        );
                        linea.clear();
                        fila += 1;
                    }
                    if !linea.is_empty() {
                        linea.push(' ');
                    }
                    linea.push_str(palabra);
                }
                texto(
                    &mut d,
                    &linea,
                    120.0,
                    598.0 + fila as f32 * 30.0,
                    23.0,
                    Color::WHITE,
                );
                texto(&mut d, "E / ENTER continuar", 120.0, 705.0, 18.0, cian);
            }
            Fase::Cierre => {
                rect(&mut d, 190.0, 220.0, 900.0, 360.0, panel);
                texto(
                    &mut d,
                    "UN HOGAR ENTRE LAS ESTRELLAS",
                    225.0,
                    260.0,
                    34.0,
                    cian,
                );
                texto(
                    &mut d,
                    "Datos enviados. Rocky y TARS a salvo. Mision cumplida.",
                    225.0,
                    330.0,
                    23.0,
                    Color::WHITE,
                );
                texto(
                    &mut d,
                    "ENTER / E  Quedarme en la playa",
                    225.0,
                    425.0,
                    25.0,
                    Color::WHITE,
                );
                texto(
                    &mut d,
                    "X / ESC    Volver a la nave y seguir explorando",
                    225.0,
                    480.0,
                    23.0,
                    cian,
                );
            }
            Fase::Libre => {
                rect(&mut d, 20.0, 733.0, 1240.0, 45.0, panel);
                texto(&mut d,"WASD caminar/nadar | Raton/flechas mirar | E hablar | X/Esc despegar | F1-F3 calidad",35.0,748.0,18.0,Color::WHITE);
                if m.nadando() {
                    texto(
                        &mut d,
                        "NADANDO / WASD para volver a la orilla",
                        40.0,
                        100.0,
                        20.0,
                        cian,
                    );
                }
                if let Some(t) = m.comentario() {
                    rect(&mut d, 70.0, 650.0, 1140.0, 60.0, panel);
                    texto(&mut d, t, 90.0, 669.0, 21.0, cian);
                }
            }
        }
        drop(d);
        if salir {
            self.activa = false;
        }
        salir
    }
}
