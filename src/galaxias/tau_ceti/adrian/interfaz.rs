// Entrada y HUD dentro del bucle principal existente; nunca modifica la nave del sistema.
use crate::{
    escena_adrian::EscenaAdrian,
    mision_adrian::{Configuracion, Fase, MisionAdrian, Resultado},
    nave::Nave,
    raster::Framebuffer,
};
use raylib::prelude::*;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const CIAN: Color = Color::new(100, 226, 239, 255);
const VERDE: Color = Color::new(123, 245, 158, 255);
const TEXTO: Color = Color::new(235, 241, 242, 255);
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
}
fn semilla() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

pub struct OperacionAdrian {
    pub activa: bool,
    pub mision: Option<MisionAdrian>,
    escena: Option<EscenaAdrian>,
    fb: Framebuffer,
    tex: Option<Texture2D>,
    ultimo: Instant,
}
impl Default for OperacionAdrian {
    fn default() -> Self {
        Self {
            activa: false,
            mision: None,
            escena: None,
            fb: Framebuffer::new(720, 450),
            tex: None,
            ultimo: Instant::now(),
        }
    }
}
impl OperacionAdrian {
    pub fn entrar(&mut self, nave: &Nave) {
        if self.activa {
            return;
        }
        if self.escena.is_none() {
            self.escena = Some(EscenaAdrian::nueva());
        }
        self.mision = Some(MisionAdrian::nueva(
            semilla(),
            nave.fwd,
            nave.up,
            Configuracion::default(),
        ));
        self.activa = true;
        self.ultimo = Instant::now();
    }
    pub fn exitosa(&self) -> bool {
        self.mision.as_ref().is_some_and(|m| m.exitosa())
    }

    pub fn cuadro(
        &mut self,
        window: &mut RaylibHandle,
        thread: &RaylibThread,
        nave: &Nave,
    ) -> bool {
        let ahora = Instant::now();
        let dt = ahora.duration_since(self.ultimo).as_secs_f64();
        self.ultimo = ahora;
        let mut salir = window.is_key_pressed(KeyboardKey::KEY_X)
            || window.is_key_pressed(KeyboardKey::KEY_ESCAPE);
        for (key, w, h) in [
            (KeyboardKey::KEY_F1, 960, 600),
            (KeyboardKey::KEY_F2, 720, 450),
            (KeyboardKey::KEY_F3, 480, 300),
        ] {
            if window.is_key_pressed(key) && self.fb.width != w {
                self.fb.redimensionar(w, h);
                self.tex = None;
            }
        }
        let m = self.mision.as_mut().expect("mision activa");
        let fase_entrada = m.fase;
        let cambio_pausa = window.is_key_pressed(KeyboardKey::KEY_P);
        if cambio_pausa {
            m.alternar_pausa();
        }
        let control = crate::entrada_nave::leer(window, (0.0, 0.0));
        if !cambio_pausa {
            m.actualizar(dt, &control);
        }
        let modal = m.pausada || matches!(m.fase, Fase::Instrucciones | Fase::Final(_));
        if modal {
            window.enable_cursor();
        } else {
            window.disable_cursor();
        }
        let (sw, sh) = (
            window.get_screen_width() as f32,
            window.get_screen_height() as f32,
        );
        let s = (sw / 1280.0).min(sh / 800.0);
        let (ox, oy) = ((sw - 1280.0 * s) * 0.5, (sh - 800.0 * s) * 0.5);
        let mouse = window.get_mouse_position();
        let (mx, my) = ((mouse.x - ox) / s, (mouse.y - oy) / s);
        let clic = window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let sobre =
            |x: f32, y: f32, w: f32, h: f32| mx >= x && mx <= x + w && my >= y && my <= y + h;
        if m.fase == Fase::Instrucciones
            && fase_entrada == Fase::Instrucciones
            && (window.is_key_pressed(KeyboardKey::KEY_ENTER)
                || (clic && sobre(440.0, 636.0, 400.0, 52.0)))
        {
            m.comenzar();
            self.ultimo = Instant::now();
            window.disable_cursor();
        }
        let reiniciar = matches!(fase_entrada, Fase::Final(_))
            && (window.is_key_pressed(KeyboardKey::KEY_R)
                || (clic && sobre(260.0, 620.0, 340.0, 52.0)));
        if matches!(fase_entrada, Fase::Final(_)) && clic && sobre(650.0, 620.0, 370.0, 52.0) {
            salir = true;
        }
        if m.pausada && clic && sobre(440.0, 636.0, 400.0, 52.0) {
            m.alternar_pausa();
            self.ultimo = Instant::now();
        }
        self.escena.as_ref().unwrap().dibujar(&mut self.fb, m, nave);
        if self.tex.is_none() {
            let img =
                Image::gen_image_color(self.fb.width as i32, self.fb.height as i32, Color::BLACK);
            let tex = window
                .load_texture_from_image(thread, &img)
                .expect("textura de operacion");
            tex.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
            self.tex = Some(tex);
        }
        let tex = self.tex.as_mut().unwrap();
        tex.update_texture(&self.fb.pixels)
            .expect("framebuffer de operacion");
        let mut d = window.begin_drawing(thread);
        d.clear_background(Color::BLACK);
        d.draw_texture_pro(
            &*tex,
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
        if m.destello_verde > 0.0 {
            l.rect(
                0.0,
                0.0,
                1280.0,
                800.0,
                Color::new(35, 230, 100, (m.destello_verde * 150.0) as u8),
            );
        }
        if m.destello_rojo > 0.0 {
            l.rect(
                0.0,
                0.0,
                1280.0,
                800.0,
                Color::new(240, 20, 35, (m.destello_rojo * 200.0) as u8),
            );
        }
        if matches!(m.fase, Fase::Recoleccion | Fase::Extraccion) {
            l.rect(20.0, 20.0, 915.0, 76.0, Color::new(4, 14, 26, 215));
            l.texto(
                &format!("MUESTRAS: {} / {}", m.muestras, m.config.muestras),
                38.0,
                36.0,
                23.0,
                VERDE,
            );
            l.texto(
                &format!("INTEGRIDAD: {} / 3", m.integridad),
                296.0,
                36.0,
                23.0,
                if m.integridad == 1 {
                    Color::ORANGE
                } else {
                    TEXTO
                },
            );
            let tiempo = m.restante().ceil() as u64;
            l.texto(
                &format!("TIEMPO RESTANTE: {:02}:{:02}", tiempo / 60, tiempo % 60),
                600.0,
                36.0,
                23.0,
                if tiempo <= 15 { Color::ORANGE } else { CIAN },
            );
            l.texto(
                if m.fase == Fase::Extraccion {
                    "RECOLECCION COMPLETA - DIRIGETE AL ANILLO CON W"
                } else {
                    "INTERCEPTA LAS ESFERAS VERDES / ESQUIVA LAS ROCAS"
                },
                38.0,
                69.0,
                16.0,
                CIAN,
            );
            l.rect(20.0, 750.0, 1240.0, 32.0, Color::new(4, 14, 26, 215));
            l.texto(if m.fase==Fase::Extraccion { "W/S avanzar y frenar | A/D lateral | Espacio/Ctrl subir/bajar | P pausa | X/Esc salir" }
                else { "A/D lateral | Espacio/Ctrl subir/bajar | P pausa | X/Esc salir | F1-F3 calidad" },35.0,758.0,17.0,TEXTO);
        }
        if m.fase == Fase::Petrova {
            l.rect(0.0, 0.0, 1280.0, 52.0, Color::BLACK);
            l.rect(0.0, 700.0, 1280.0, 100.0, Color::new(0, 0, 0, 225));
            l.texto(
                "TAU CETI - REGION DE RECOLECCION",
                240.0,
                714.0,
                29.0,
                TEXTO,
            );
            l.texto(
                "LINEA PETROVA",
                510.0,
                758.0,
                20.0,
                Color::new(255, 126, 128, 255),
            );
        }
        if m.fase == Fase::Instrucciones || m.pausada || matches!(m.fase, Fase::Final(_)) {
            let finalizada = matches!(m.fase, Fase::Final(_));
            l.rect(110.0, 120.0, 1060.0, 585.0, Color::new(5, 20, 35, 246));
            l.rect(110.0, 120.0, 1060.0, 6.0, VERDE);
            if m.pausada {
                l.texto("OPERACION EN PAUSA", 200.0, 190.0, 34.0, CIAN);
                l.texto(
                    "El tiempo, los objetos y la nave estan detenidos.",
                    200.0,
                    280.0,
                    25.0,
                    TEXTO,
                );
                l.texto(
                    "P para continuar o X/Esc para volver al espacio.",
                    200.0,
                    337.0,
                    23.0,
                    TEXTO,
                );
            } else if !finalizada {
                l.texto("OPERACION TAUMOEBA", 170.0, 164.0, 36.0, VERDE);
                l.texto(
                    "Recolecta las muestras y evita los obstaculos.",
                    170.0,
                    220.0,
                    25.0,
                    TEXTO,
                );
                for (i, linea) in [
                    "1. Los objetos aparecen delante y se acercan a tu nave.",
                    "2. Esferas verdes con membrana cian: muestras biologicas.",
                    "3. Rocas irregulares: obstaculos. Puedes resistir tres impactos.",
                    "4. Usa A/D y Espacio/Ctrl. El vuelo permanece estabilizado.",
                    "5. Reune 8 muestras y atraviesa el anillo antes de 90 segundos.",
                    "6. En extraccion, W/S controla el avance hacia el anillo.",
                ]
                .iter()
                .enumerate()
                {
                    l.texto(linea, 170.0, 290.0 + i as f32 * 43.0, 22.0, TEXTO);
                }
                l.texto(
                    "P pausa la operacion. X/Esc permite volver al espacio.",
                    170.0,
                    570.0,
                    20.0,
                    CIAN,
                );
            } else {
                let (titulo, detalle, color) = match m.fase {
                    Fase::Final(Resultado::Victoria) => (
                        "MISION COMPLETADA",
                        "Las muestras de taumoebas han sido recuperadas.",
                        VERDE,
                    ),
                    Fase::Final(Resultado::Danos) => (
                        "MISION FALLIDA",
                        "La nave sufrio demasiados danos durante la recoleccion.",
                        Color::ORANGE,
                    ),
                    _ => (
                        "TIEMPO AGOTADO",
                        "No se completo la operacion de recoleccion.",
                        Color::ORANGE,
                    ),
                };
                l.texto(titulo, 170.0, 180.0, 35.0, color);
                l.texto(detalle, 170.0, 250.0, 24.0, TEXTO);
                l.texto(
                    &format!(
                        "Muestras recuperadas: {} / {}",
                        m.muestras, m.config.muestras
                    ),
                    170.0,
                    345.0,
                    26.0,
                    VERDE,
                );
                l.texto(
                    &format!("Tiempo utilizado: {:.1} segundos", m.tiempo),
                    170.0,
                    400.0,
                    26.0,
                    TEXTO,
                );
                l.texto(
                    &format!("Integridad restante: {} / 3", m.integridad),
                    170.0,
                    455.0,
                    26.0,
                    TEXTO,
                );
                l.rect(260.0, 620.0, 340.0, 52.0, Color::new(31, 75, 95, 255));
                l.texto("REINICIAR [R]", 315.0, 637.0, 24.0, TEXTO);
                l.rect(650.0, 620.0, 370.0, 52.0, Color::new(29, 100, 69, 255));
                l.texto("VOLVER AL ESPACIO [X]", 681.0, 637.0, 23.0, TEXTO);
            }
            if !finalizada {
                l.rect(440.0, 636.0, 400.0, 52.0, Color::new(29, 100, 69, 255));
                l.texto(
                    if m.pausada {
                        "CONTINUAR [P]"
                    } else {
                        "COMENZAR [ENTER]"
                    },
                    493.0,
                    653.0,
                    24.0,
                    TEXTO,
                );
            }
        }
        drop(d); // Siempre EndDrawing antes de salir: consume las teclas de la pantalla.
        if reiniciar && !salir {
            m.reiniciar(semilla());
            self.ultimo = Instant::now();
        }
        if salir {
            self.activa = false;
        }
        salir
    }
}
