//! Menú e introducción dentro del bucle principal. Raylib solo presenta el framebuffer y el HUD.
use crate::{
    cielo::Cielo,
    dialogos_intro as texto,
    escena::Escena,
    escena_inicio::EscenaInicio,
    inicio::{Inicio, Orden, Pantalla},
    nave::Nave,
    raster::Framebuffer,
};
use raylib::prelude::*;
struct Lienzo<'a, D: RaylibDraw> {
    d: &'a mut D,
    s: f32,
    ox: f32,
    oy: f32,
}
impl<D: RaylibDraw> Lienzo<'_, D> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        self.d.draw_rectangle_rec(
            Rectangle::new(
                self.ox + x * self.s,
                self.oy + y * self.s,
                w * self.s,
                h * self.s,
            ),
            c,
        );
    }
    fn texto(&mut self, t: &str, x: f32, y: f32, tam: f32, c: Color) {
        self.d.draw_text(
            t,
            (self.ox + x * self.s) as i32,
            (self.oy + y * self.s) as i32,
            (tam * self.s).max(1.0) as i32,
            c,
        );
    }
    fn parrafo(&mut self, t: &str, x: f32, y: f32, tam: f32, c: Color) {
        let mut linea = String::new();
        let mut fila = 0;
        for palabra in t.split_whitespace() {
            if linea.chars().count() + palabra.chars().count() > 86 {
                self.texto(&linea, x, y + fila as f32 * (tam + 7.0), tam, c);
                fila += 1;
                linea.clear();
            }
            if !linea.is_empty() {
                linea.push(' ');
            }
            linea.push_str(palabra);
        }
        self.texto(&linea, x, y + fila as f32 * (tam + 7.0), tam, c);
    }
}
pub struct VisorInicio {
    escena: Option<EscenaInicio>,
    fb: Framebuffer,
    tex: Option<Texture2D>,
    tiempo: f32,
}
impl Default for VisorInicio {
    fn default() -> Self {
        Self {
            escena: None,
            fb: Framebuffer::new(720, 450),
            tex: None,
            tiempo: 0.0,
        }
    }
}
impl VisorInicio {
    pub fn cuadro(
        &mut self,
        w: &mut RaylibHandle,
        thread: &RaylibThread,
        inicio: &mut Inicio,
        escena: &Escena,
        cielo: &Cielo,
        nave: &Nave,
        dt: f32,
    ) -> Pantalla {
        self.tiempo += dt;
        let (sw, sh) = (w.get_screen_width() as f32, w.get_screen_height() as f32);
        let s = (sw / 1280.0).min(sh / 800.0);
        let (ox, oy) = ((sw - 1280.0 * s) * 0.5, (sh - 800.0 * s) * 0.5);
        let mouse = w.get_mouse_position();
        let (mx, my) = ((mouse.x - ox) / s, (mouse.y - oy) / s);
        let clic = w.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let confirmar =
            w.is_key_pressed(KeyboardKey::KEY_ENTER) || w.is_key_pressed(KeyboardKey::KEY_SPACE);
        let atras = w.is_key_pressed(KeyboardKey::KEY_ESCAPE);
        match inicio.pantalla() {
            Pantalla::Menu => {
                if inicio.menu.controles_abiertos() {
                    if atras {
                        inicio.orden(Orden::Atras);
                    } else if confirmar
                        || (clic && (140.0..440.0).contains(&mx) && (635.0..690.0).contains(&my))
                    {
                        inicio.orden(Orden::Aceptar);
                    }
                } else {
                    let delta = w.get_mouse_delta();
                    if delta.x.abs() + delta.y.abs() > 0.0 || clic {
                        for i in 0..crate::menu::OPCIONES.len() {
                            if (65.0..465.0).contains(&mx)
                                && (345.0 + i as f32 * 78.0..405.0 + i as f32 * 78.0).contains(&my)
                            {
                                inicio.menu.elegir(i);
                            }
                        }
                    }
                    if atras {
                        inicio.orden(Orden::Atras);
                    } else if w.is_key_pressed(KeyboardKey::KEY_UP)
                        || w.is_key_pressed(KeyboardKey::KEY_W)
                    {
                        inicio.orden(Orden::Arriba);
                    } else if w.is_key_pressed(KeyboardKey::KEY_DOWN)
                        || w.is_key_pressed(KeyboardKey::KEY_S)
                    {
                        inicio.orden(Orden::Abajo);
                    } else if confirmar
                        || (clic
                            && (65.0..465.0).contains(&mx)
                            && (0..3).any(|i| {
                                (345.0 + i as f32 * 78.0..405.0 + i as f32 * 78.0).contains(&my)
                            }))
                    {
                        inicio.orden(Orden::Aceptar);
                    }
                }
            }
            Pantalla::Intro => {
                if atras {
                    inicio.orden(Orden::Atras);
                } else if confirmar || (clic && inicio.intro.en_titulo()) {
                    inicio.orden(Orden::Aceptar);
                }
            }
            _ => {}
        }
        inicio.actualizar(dt);
        let pantalla = inicio.pantalla();
        if pantalla == Pantalla::Menu {
            w.enable_cursor();
        } else {
            w.disable_cursor();
        }
        if matches!(pantalla, Pantalla::Menu | Pantalla::Intro) {
            self.escena.get_or_insert_with(EscenaInicio::nueva).dibujar(
                &mut self.fb,
                inicio,
                escena,
                cielo,
                nave,
            );
        } else {
            self.fb.pixels.fill(0);
        }
        if self.tex.is_none() {
            let img =
                Image::gen_image_color(self.fb.width as i32, self.fb.height as i32, Color::BLACK);
            let tex = w
                .load_texture_from_image(thread, &img)
                .expect("framebuffer del inicio");
            tex.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
            self.tex = Some(tex);
        }
        let tex = self.tex.as_mut().unwrap();
        tex.update_texture(&self.fb.pixels).unwrap();
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
        let mut l = Lienzo {
            d: &mut d,
            s,
            ox,
            oy,
        };
        let cian = Color::new(126, 237, 242, 255);
        let panel = Color::new(3, 10, 22, 235);
        match pantalla {
            Pantalla::Menu => {
                l.rect(0.0, 0.0, 550.0, 800.0, Color::new(2, 7, 19, 210));
                l.texto(texto::TITULO, 65.0, 140.0, 65.0, Color::WHITE);
                l.texto(texto::SUBTITULO, 68.0, 220.0, 21.0, cian);
                for (i, t) in crate::menu::OPCIONES.iter().enumerate() {
                    let y = 345.0 + i as f32 * 78.0;
                    let seleccionado = i == inicio.menu.seleccion();
                    l.rect(
                        65.0,
                        y,
                        400.0,
                        60.0,
                        if seleccionado {
                            Color::new(15, 70, 88, 230)
                        } else {
                            Color::new(3, 12, 26, 190)
                        },
                    );
                    if seleccionado {
                        l.rect(
                            65.0,
                            y,
                            4.0,
                            60.0,
                            Color::new(126, 237, 242, (170.0 + 85.0 * inicio.menu.pulso()) as u8),
                        );
                    }
                    l.texto(
                        t,
                        90.0,
                        y + 19.0,
                        24.0,
                        if seleccionado { cian } else { Color::WHITE },
                    );
                }
                l.texto(
                    "Flechas / W-S elegir | ENTER aceptar",
                    65.0,
                    680.0,
                    18.0,
                    Color::LIGHTGRAY,
                );
                l.texto(
                    "Tambien puedes usar el raton",
                    65.0,
                    711.0,
                    18.0,
                    Color::LIGHTGRAY,
                );
                if inicio.menu.controles_abiertos() {
                    l.rect(90.0, 110.0, 1100.0, 600.0, panel);
                    l.texto("CONTROLES", 140.0, 145.0, 32.0, cian);
                    for (i, (tecla, accion)) in crate::menu::CONTROLES.iter().enumerate() {
                        let x = 140.0 + (i / 8) as f32 * 525.0;
                        let y = 215.0 + (i % 8) as f32 * 48.0;
                        l.texto(tecla, x, y, 18.0, cian);
                        l.texto(accion, x + 165.0, y, 18.0, Color::WHITE);
                    }
                    l.rect(140.0, 635.0, 300.0, 55.0, Color::new(15, 70, 88, 255));
                    l.texto("VOLVER [ESC / ENTER]", 160.0, 653.0, 20.0, cian);
                }
            }
            Pantalla::Intro => {
                if inicio.intro.en_titulo() {
                    let alfa =
                        (255.0 * crate::math::smoothstep(0.0, 0.45, inicio.intro.progreso())) as u8;
                    l.texto(
                        texto::TITULO,
                        355.0,
                        300.0,
                        76.0,
                        Color::new(237, 245, 252, alfa),
                    );
                    l.texto(
                        texto::SUBTITULO,
                        410.0,
                        402.0,
                        25.0,
                        Color::new(126, 237, 242, alfa),
                    );
                    let pulso = (180.0 + 75.0 * (self.tiempo * 2.2).sin().abs()) as u8;
                    l.texto(
                        texto::PROMPT_TITULO,
                        515.0,
                        545.0,
                        24.0,
                        Color::new(230, 240, 250, pulso.min(alfa)),
                    );
                } else {
                    l.rect(0.0, 0.0, 1280.0, 64.0, Color::BLACK);
                    l.rect(0.0, 650.0, 1280.0, 150.0, Color::BLACK);
                    if let Some((sub, alfa)) = inicio.intro.subtitulo() {
                        let a = (alfa * 255.0) as u8;
                        if !sub.nombre.is_empty() {
                            l.texto(sub.nombre, 125.0, 658.0, 20.0, Color::new(126, 237, 242, a));
                        }
                        l.parrafo(sub.texto, 125.0, 690.0, 23.0, Color::new(240, 245, 250, a));
                    }
                    l.texto(texto::AYUDA_OMITIR, 890.0, 30.0, 16.0, Color::GRAY);
                }
                l.rect(
                    0.0,
                    0.0,
                    1280.0,
                    800.0,
                    Color::new(0, 0, 0, (inicio.intro.fundido() * 255.0) as u8),
                );
            }
            _ => {}
        }
        drop(d); // Consumir las teclas incluso al omitir la intro o salir del menú.
        pantalla
    }
}
