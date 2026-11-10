// Presentación de la aventura dentro del bucle del juego. La escena 3D completa
// viene del rasterizador; raylib solo presenta el framebuffer y la interfaz.
use crate::{
    aventura::{Aventura, EntradaAventura, Evento, Fase},
    camara::Camara,
    cielo::Cielo,
    dibujo::Ajustes,
    escena::Escena,
    inventario::Material,
    jugador::EntradaJ,
    math::*,
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
    fn parrafo(&mut self, t: &str, x: f32, y: f32, ancho: usize, tam: f32, c: Color) {
        let mut linea = String::new();
        let mut fila = 0;
        for palabra in t.split_whitespace() {
            if !linea.is_empty() && linea.chars().count() + palabra.chars().count() + 1 > ancho {
                self.texto(&linea, x, y + fila as f32 * (tam + 5.0), tam, c);
                fila += 1;
                linea.clear();
            }
            if !linea.is_empty() {
                linea.push(' ');
            }
            linea.push_str(palabra);
        }
        self.texto(&linea, x, y + fila as f32 * (tam + 5.0), tam, c);
    }
}

pub struct VisorInterior {
    fb: Framebuffer,
    tex: Option<Texture2D>,
    inventario: bool,
    mouse: bool,
}
impl Default for VisorInterior {
    fn default() -> Self {
        Self {
            fb: Framebuffer::new(720, 450),
            tex: None,
            inventario: true,
            mouse: true,
        }
    }
}
impl VisorInterior {
    pub fn cuadro(
        &mut self,
        window: &mut RaylibHandle,
        thread: &RaylibThread,
        a: &mut Aventura,
        nave: &mut Nave,
        escena: &Escena,
        cielo: &Cielo,
        aj: &Ajustes,
        dt: f32,
    ) -> Evento {
        let salir = window.is_key_pressed(KeyboardKey::KEY_X)
            || window.is_key_pressed(KeyboardKey::KEY_ESCAPE)
            || (a.en_crucero() && window.is_key_pressed(KeyboardKey::KEY_H));
        if window.is_key_pressed(KeyboardKey::KEY_TAB) {
            self.mouse = !self.mouse;
        }
        if window.is_key_pressed(KeyboardKey::KEY_I) {
            self.inventario = !self.inventario;
        }
        if self.mouse {
            window.disable_cursor();
        } else {
            window.enable_cursor();
        }
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
        let eje =
            |mas, menos| (window.is_key_down(mas) as i32 - window.is_key_down(menos) as i32) as f32;
        let raton = if self.mouse {
            window.get_mouse_delta()
        } else {
            Vector2::zero()
        };
        let ent = EntradaAventura {
            j: EntradaJ {
                adelante: eje(KeyboardKey::KEY_W, KeyboardKey::KEY_S),
                lateral: eje(KeyboardKey::KEY_D, KeyboardKey::KEY_A),
                giro_x: raton.x + 420.0 * dt * eje(KeyboardKey::KEY_RIGHT, KeyboardKey::KEY_LEFT),
                giro_y: raton.y - 420.0 * dt * eje(KeyboardKey::KEY_UP, KeyboardKey::KEY_DOWN),
                correr: window.is_key_down(KeyboardKey::KEY_LEFT_SHIFT),
            },
            interactuar: window.is_key_pressed(KeyboardKey::KEY_E),
        };
        let evento = a.actualizar(&ent, dt);
        if a.fase == Fase::Atraque
            || evento == Evento::Desacoplar
            || evento == Evento::VolverAlVuelo
        {
            if let Some((pos, fwd)) = a.pose_atraque() {
                nave.teletransportar(pos, pos + fwd);
            }
            let mut cam = Camara::nueva(self.fb.width as f32, self.fb.height as f32);
            let lado = nave.fwd.cross(v3(0.0, 1.0, 0.0)).norm();
            cam.pos = nave.pos - nave.fwd * 12.0 + lado * 6.0 + v3(0.0, 5.0, 0.0);
            cam.mirar_a(nave.pos + nave.fwd * 3.0);
            crate::render::dibujar_con_nave(&mut self.fb, escena, cielo, &cam, 0.0, aj, Some(nave));
        } else {
            a.dibujar(&mut self.fb, escena, cielo, aj);
        }
        if self.tex.is_none() {
            let img =
                Image::gen_image_color(self.fb.width as i32, self.fb.height as i32, Color::BLACK);
            let tex = window
                .load_texture_from_image(thread, &img)
                .expect("textura de interiores");
            tex.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
            self.tex = Some(tex);
        }
        let tex = self.tex.as_mut().unwrap();
        tex.update_texture(&self.fb.pixels)
            .expect("framebuffer interior");
        let (sw, sh) = (
            window.get_screen_width() as f32,
            window.get_screen_height() as f32,
        );
        let s = (sw / 1280.0).min(sh / 800.0);
        let (ox, oy) = ((sw - 1280.0 * s) * 0.5, (sh - 800.0 * s) * 0.5);
        let hud = a.hud();
        let cian = Color::new(111, 232, 239, 255);
        let panel = Color::new(5, 17, 26, 220);
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
        l.rect(20.0, 20.0, 820.0, 115.0, panel);
        l.texto(
            match hud.fase {
                Fase::TunelBlip => "BLIP-A - PASILLO DE ENCUENTRO",
                Fase::Atraque if a.en_blip_a() => "BLIP-A - MANIOBRA DE ATRAQUE",
                Fase::Nave if a.en_blip_a() => "CABINA - ATRACADOS EN BLIP-A",
                Fase::Atraque => "ESTACION - MANIOBRA DE ATRAQUE",
                Fase::Estacion => "ESTACION - RECUPERAR PROVISIONES",
                _ => "NAVE - SISTEMAS Y PROVISIONES",
            },
            38.0,
            35.0,
            24.0,
            cian,
        );
        l.parrafo(
            if hud.fase == Fase::Atraque {
                "Acoplando. Al finalizar estaras dentro de la nave, junto a TARS."
            } else {
                &hud.objetivo
            },
            38.0,
            72.0,
            75,
            20.0,
            Color::WHITE,
        );
        if hud.fase != Fase::Atraque {
            l.rect(636.0, 396.0, 8.0, 2.0, cian);
            l.rect(639.0, 393.0, 2.0, 8.0, cian);
            if let Some(prompt) = &hud.prompt {
                l.rect(130.0, 608.0, 1020.0, 46.0, panel);
                l.texto(&format!("[E] {prompt}"), 154.0, 620.0, 22.0, cian);
            }
            if let Some(mensaje) = &hud.mensaje {
                l.rect(130.0, 664.0, 1020.0, 80.0, panel);
                l.parrafo(mensaje, 154.0, 678.0, 86, 21.0, Color::WHITE);
            }
            l.rect(20.0, 758.0, 1240.0, 30.0, panel);
            l.texto("WASD caminar | Raton/flechas mirar | Shift correr | E interactuar | I inventario | Tab cursor | X/Esc salir",30.0,765.0,16.0,Color::WHITE);
            if self.inventario {
                l.rect(865.0, 20.0, 395.0, 218.0, panel);
                l.texto(
                    &format!("SISTEMAS {}/5  [I] INVENTARIO", hud.instalados),
                    880.0,
                    34.0,
                    20.0,
                    cian,
                );
                for (i, m) in Material::TODOS.iter().enumerate() {
                    let instalado = a.inv.instalados.contains(m);
                    let mochila = a.inv.mochila.contains(m);
                    let col = m.color();
                    l.rect(
                        881.0,
                        73.0 + i as f32 * 31.0,
                        10.0,
                        10.0,
                        Color::new(
                            (col.x * 255.0) as u8,
                            (col.y * 255.0) as u8,
                            (col.z * 255.0) as u8,
                            255,
                        ),
                    );
                    l.texto(
                        &format!(
                            "{} {}",
                            if instalado {
                                "+"
                            } else if mochila {
                                "*"
                            } else {
                                "-"
                            },
                            m.nombre()
                        ),
                        902.0,
                        69.0 + i as f32 * 31.0,
                        18.0,
                        if instalado {
                            Color::GREEN
                        } else if mochila {
                            Color::WHITE
                        } else {
                            Color::GRAY
                        },
                    );
                }
            }
            if hud.fase == Fase::Estacion {
                l.rect(1030.0, 260.0, 230.0, 230.0, panel);
                l.texto("ESTACION / MATERIALES", 1042.0, 272.0, 16.0, cian);
                let (cx, cy, k) = (1145.0, 385.0, 3.6);
                // Esquema del núcleo, anillo y cuatro brazos, con puntos reales.
                for i in 0..64 {
                    let ang = i as f32 * std::f32::consts::TAU / 64.0;
                    l.rect(
                        cx + 17.0 * k * ang.cos(),
                        cy + 17.0 * k * ang.sin(),
                        2.0,
                        2.0,
                        Color::GRAY,
                    );
                }
                l.rect(cx - 60.0, cy, 120.0, 1.0, Color::GRAY);
                l.rect(cx, cy - 60.0, 1.0, 120.0, Color::GRAY);
                for (x, z) in &hud.pendientes {
                    l.rect(cx + x * k - 2.0, cy + z * k - 2.0, 5.0, 5.0, Color::YELLOW);
                }
                l.texto("N", cx + 23.0 * k, cy, 14.0, cian);
                let (x, z, yaw) = hud.jugador;
                l.rect(cx + x * k - 2.0, cy + z * k - 2.0, 5.0, 5.0, Color::WHITE);
                l.rect(
                    cx + x * k + yaw.sin() * 7.0,
                    cy + z * k - yaw.cos() * 7.0,
                    3.0,
                    3.0,
                    cian,
                );
            }
        }
        l.rect(
            0.0,
            0.0,
            1280.0,
            800.0,
            Color::new(0, 0, 0, (255.0 * hud.fundido) as u8),
        );
        drop(d); // EndDrawing consume teclado incluso al salir de la escena.
        if salir {
            let crucero = a.en_crucero();
            a.suspender();
            return if crucero {
                Evento::VolverAlVuelo
            } else {
                Evento::Desacoplar
            };
        }
        evento
    }
}
