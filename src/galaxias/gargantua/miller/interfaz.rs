// Controlador de superficie dentro del bucle principal existente. Raylib solo
// lee entradas y dibuja HUD; la geometría usa el rasterizador compartido.
use crate::{
    camara::{Camara, Entrada},
    math::*,
    mision_miller::*,
    nave::Nave,
    paisaje_miller::EscenaMiller,
    raster::Framebuffer,
};
use raylib::prelude::*;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const CIAN: Color = Color::new(115, 237, 237, 255);
const TEXTO: Color = Color::new(237, 244, 244, 255);
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

pub struct SuperficieMiller {
    pub activa: bool,
    pub mision: Option<MisionMiller>,
    escena: Option<EscenaMiller>,
    cam: Camara,
    fb: Framebuffer,
    tex: Option<Texture2D>,
    ultimo: Instant,
    config: Configuracion,
    mouse: bool,
}
impl Default for SuperficieMiller {
    fn default() -> Self {
        Self {
            activa: false,
            mision: None,
            escena: None,
            cam: EscenaMiller::camara(720, 450),
            fb: Framebuffer::new(720, 450),
            tex: None,
            ultimo: Instant::now(),
            config: Configuracion::default(),
            mouse: true,
        }
    }
}
impl SuperficieMiller {
    pub fn entrar(&mut self) {
        if self.escena.is_none() {
            self.escena = Some(EscenaMiller::nueva());
        }
        let semilla = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        self.mision = Some(MisionMiller::aterrizar(semilla, self.config));
        self.cam = EscenaMiller::camara(self.fb.width, self.fb.height);
        self.activa = true;
        self.mouse = true;
        self.ultimo = Instant::now();
    }
    pub fn exitosa(&self) -> bool {
        self.mision.as_ref().is_some_and(|m| m.exitosa())
    }

    // Retorna salir después de EndDrawing, para consumir siempre la entrada.
    pub fn cuadro(
        &mut self,
        window: &mut RaylibHandle,
        thread: &RaylibThread,
        nave: &Nave,
    ) -> bool {
        let ahora = Instant::now();
        let dt = ahora.duration_since(self.ultimo).as_secs_f64();
        self.ultimo = ahora;
        let salir = window.is_key_pressed(KeyboardKey::KEY_X)
            || window.is_key_pressed(KeyboardKey::KEY_ESCAPE);
        if window.is_key_pressed(KeyboardKey::KEY_TAB) {
            self.mouse = !self.mouse;
        }
        let m = self.mision.as_mut().expect("misión iniciada");
        let fase_entrada = m.fase;
        let estaba_final = matches!(m.fase, Fase::Final(_));
        let modal = matches!(
            m.fase,
            Fase::Descenso
                | Fase::Instrucciones
                | Fase::Cuestionario
                | Fase::Estrategia
                | Fase::Final(_)
        );
        if modal || !self.mouse {
            window.enable_cursor();
        } else {
            window.disable_cursor();
        }
        let eje = |a, b| (window.is_key_down(a) as i32 - window.is_key_down(b) as i32) as f32;
        let raton = if !modal && self.mouse {
            window.get_mouse_delta()
        } else {
            Vector2::zero()
        };
        let cols = self.escena.as_ref().unwrap().colisionadores();
        let mut restante = dt;
        // Mismos controles de cámara, con subpasos para cajas y ola. Incluso
        // con FPS bajos se consume todo el tiempo medido por el reloj monotónico.
        while restante > 0.0 && !matches!(m.fase, Fase::Final(_)) {
            let paso_reloj = restante.min(0.02);
            let paso = paso_reloj as f32;
            let movil = matches!(m.fase, Fase::Explorando | Fase::Regreso);
            if movil {
                let mut entrada = Entrada {
                    adelante: eje(KeyboardKey::KEY_W, KeyboardKey::KEY_S),
                    lateral: eje(KeyboardKey::KEY_D, KeyboardKey::KEY_A),
                    vertical: 0.0,
                    giro_x: raton.x * (paso as f64 / dt) as f32
                        + 420.0 * paso * eje(KeyboardKey::KEY_RIGHT, KeyboardKey::KEY_LEFT),
                    giro_y: raton.y * (paso as f64 / dt) as f32
                        - 420.0 * paso * eje(KeyboardKey::KEY_UP, KeyboardKey::KEY_DOWN),
                    turbo: false,
                };
                if m.estrategia == Some(Estrategia::Tars) {
                    let dir = direccion_regreso(self.cam.pos, &cols);
                    let (f, r, _) = self.cam.base();
                    entrada.adelante = dir.dot(v3(f.x, 0.0, f.z).norm());
                    entrada.lateral = dir.dot(r);
                }
                self.cam
                    .actualizar_superficie(&entrada, paso, &cols, m.velocidad());
            } else {
                self.cam.vel = V3::ZERO;
            }
            m.actualizar(paso_reloj, self.cam.pos);
            restante -= paso_reloj;
        }
        let (sw, sh) = (
            window.get_screen_width() as f32,
            window.get_screen_height() as f32,
        );
        let s = (sw / 1280.0).min(sh / 800.0);
        let (ox, oy) = ((sw - 1280.0 * s) * 0.5, (sh - 800.0 * s) * 0.5);
        let p = window.get_mouse_position();
        let p = Vector2::new((p.x - ox) / s, (p.y - oy) / s);
        let clic = window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let opcion = [
            KeyboardKey::KEY_ONE,
            KeyboardKey::KEY_TWO,
            KeyboardKey::KEY_THREE,
        ]
        .iter()
        .position(|k| window.is_key_pressed(*k))
        .or_else(|| {
            if clic && (160.0..1120.0).contains(&p.x) && (388.0..592.0).contains(&p.y) {
                let i = ((p.y - 388.0) / 68.0) as usize;
                (p.y < 388.0 + i as f32 * 68.0 + 56.0).then_some(i)
            } else {
                None
            }
        });
        // Una acción por cuadro: responder nunca selecciona también la
        // estrategia que acaba de aparecer bajo el mismo botón.
        match m.fase {
            Fase::Instrucciones if fase_entrada == Fase::Instrucciones => {
                if window.is_key_pressed(KeyboardKey::KEY_ENTER)
                    || (clic && (400.0..880.0).contains(&p.x) && (620.0..676.0).contains(&p.y))
                {
                    m.comenzar();
                    self.ultimo = Instant::now();
                    window.disable_cursor();
                }
            }
            Fase::Explorando if window.is_key_pressed(KeyboardKey::KEY_E) => {
                if let Some(i) = m.cercana(self.cam.pos) {
                    m.inspeccionar(i, self.cam.pos);
                    self.cam.vel = V3::ZERO;
                }
            }
            Fase::Cuestionario => {
                if let Some(i) = opcion {
                    m.responder(i);
                }
            }
            Fase::Estrategia => {
                if let Some(i) = opcion {
                    m.elegir(
                        [Estrategia::Tars, Estrategia::Correr, Estrategia::Equipo][i],
                        self.cam.pos,
                    );
                }
            }
            _ => {}
        }
        let reiniciar = estaba_final && window.is_key_pressed(KeyboardKey::KEY_R);
        let escena = self.escena.as_mut().unwrap();
        escena.radio_ola = m.radio_ola().max(0.1);
        escena.inspeccionadas = m.inspeccionadas;
        if matches!(m.fase, Fase::Descenso | Fase::Instrucciones) {
            escena.dibujar_descenso(&mut self.fb, nave, (m.descenso / DURACION_DESCENSO) as f32);
        } else {
            escena.dibujar_con_nave(&mut self.fb, &self.cam, m.tiempo as f32, Some(nave));
        }
        if self.tex.is_none() {
            let img =
                Image::gen_image_color(self.fb.width as i32, self.fb.height as i32, Color::BLACK);
            let tex = window
                .load_texture_from_image(thread, &img)
                .expect("textura de Miller");
            tex.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
            self.tex = Some(tex);
        }
        let tex = self.tex.as_mut().unwrap();
        tex.update_texture(&self.fb.pixels)
            .expect("framebuffer de Miller");
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
        if matches!(m.fase, Fase::Descenso | Fase::Instrucciones) {
            if m.fase == Fase::Descenso {
                l.rect(180.0, 650.0, 920.0, 104.0, Color::new(9, 28, 48, 230));
                l.texto("MILLER - SECUENCIA DE ATERRIZAJE", 240.0, 669.0, 28.0, CIAN);
                l.texto(
                    "Aproximacion, descenso y contacto sobre el agua.",
                    240.0,
                    710.0,
                    20.0,
                    TEXTO,
                );
            } else {
                let azul = Color::new(136, 195, 255, 255);
                l.rect(120.0, 120.0, 1040.0, 580.0, Color::new(12, 27, 62, 246));
                l.rect(120.0, 120.0, 1040.0, 7.0, azul);
                l.texto("EXPEDICION MILLER", 170.0, 155.0, 34.0, azul);
                l.texto(
                    "Recupera los datos cientificos y regresa a la nave.",
                    170.0,
                    210.0,
                    23.0,
                    TEXTO,
                );
                for (i, texto) in [
                    "1. Explora los tres contenedores. Solo uno tiene el modulo.",
                    "2. Acercate y presiona E para inspeccionar cada caja.",
                    "3. Responde cuatro preguntas de Graficas por Computadora.",
                    "4. Elige como regresar: TARS, correr o llevar equipo extra.",
                    "5. Alcanza la baliza de la nave antes de que llegue la ola.",
                ]
                .iter()
                .enumerate()
                {
                    l.texto(texto, 170.0, 275.0 + i as f32 * 43.0, 21.0, TEXTO);
                }
                l.texto(
                    "WASD: caminar | Raton / flechas: mirar | Tab: cursor",
                    170.0,
                    520.0,
                    21.0,
                    azul,
                );
                l.texto(
                    "Respuestas y estrategia: clic o teclas 1, 2, 3.",
                    170.0,
                    556.0,
                    21.0,
                    TEXTO,
                );
                l.texto(
                    "El reloj y la ola comienzan cuando pulses Comenzar.",
                    170.0,
                    588.0,
                    20.0,
                    azul,
                );
                l.rect(400.0, 620.0, 480.0, 56.0, Color::new(37, 83, 148, 255));
                l.texto("COMENZAR EXPEDICION  [ENTER]", 444.0, 638.0, 23.0, TEXTO);
            }
            l.texto("X / Esc: volver al espacio", 30.0, 770.0, 18.0, TEXTO);
            drop(d);
            if salir {
                self.activa = false;
            }
            return salir;
        }
        l.rect(20.0, 20.0, 650.0, 98.0, Color::new(5, 22, 30, 220));
        let segundos = m.tiempo.floor() as u64;
        l.texto(
            &format!(
                "TIEMPO EN MILLER: {:02}:{:02}",
                segundos / 60,
                segundos % 60
            ),
            38.0,
            34.0,
            26.0,
            TEXTO,
        );
        l.texto(
            "WASD mover | Raton / flechas mirar | E inspeccionar",
            38.0,
            72.0,
            17.0,
            CIAN,
        );
        l.texto(
            "X / Esc: volver al espacio | Tab: cursor",
            38.0,
            95.0,
            15.0,
            TEXTO,
        );
        // Localizador sencillo, compartiendo exactamente las posiciones físicas.
        l.rect(1050.0, 20.0, 210.0, 175.0, Color::new(5, 22, 30, 210));
        l.texto("LOCALIZADOR", 1070.0, 30.0, 16.0, CIAN);
        for (i, pos) in CAJAS.iter().enumerate() {
            l.texto(
                &format!("{}{}", i + 1, if m.inspeccionadas[i] { " +" } else { "" }),
                1150.0 + pos.x * 3.5,
                156.0 + pos.z * 2.6,
                20.0,
                if m.inspeccionadas[i] {
                    Color::GREEN
                } else {
                    Color::ORANGE
                },
            );
        }
        l.texto("NAVE", 1130.0, 160.0, 16.0, CIAN);
        l.rect(
            1150.0 + self.cam.pos.x * 3.5,
            156.0 + self.cam.pos.z * 2.6,
            5.0,
            5.0,
            Color::WHITE,
        );
        for (i, pos) in CAJAS.iter().enumerate() {
            if let Some((x, y, _)) = self.cam.proyectar(*pos + v3(0.0, 1.3, 0.0)) {
                l.texto(
                    &format!(
                        "CAJA {}{}",
                        i + 1,
                        if m.inspeccionadas[i] {
                            " - REVISADA"
                        } else {
                            ""
                        }
                    ),
                    x / self.fb.width as f32 * 1280.0 - 45.0,
                    y / self.fb.height as f32 * 800.0,
                    17.0,
                    CIAN,
                );
            }
        }
        if let Some((x, y, _)) = self.cam.proyectar(NAVE + v3(0.0, 1.0, 0.0)) {
            l.texto(
                &format!("NAVE - {:.0} m", (self.cam.pos - NAVE).len()),
                x / self.fb.width as f32 * 1280.0 - 45.0,
                y / self.fb.height as f32 * 800.0,
                20.0,
                CIAN,
            );
        }
        l.rect(20.0, 715.0, 1240.0, 65.0, Color::new(5, 22, 30, 225));
        l.texto(m.mensaje, 38.0, 730.0, 18.0, TEXTO);
        if m.cercana(self.cam.pos).is_some() {
            l.texto("Presiona E para inspeccionar.", 440.0, 660.0, 24.0, CIAN);
        }
        if let Some(e) = m.estrategia {
            l.texto(e.nombre(), 38.0, 755.0, 16.0, CIAN);
        }
        match m.fase {
            Fase::Cuestionario | Fase::Estrategia => {
                let escape = m.fase == Fase::Estrategia;
                let acento = if escape {
                    Color::new(255, 193, 108, 255)
                } else {
                    Color::new(196, 166, 255, 255)
                };
                l.rect(
                    120.0,
                    230.0,
                    1040.0,
                    440.0,
                    if escape {
                        Color::new(58, 29, 12, 247)
                    } else {
                        Color::new(31, 19, 56, 247)
                    },
                );
                l.rect(120.0, 230.0, 1040.0, 7.0, acento);
                let opciones;
                if m.fase == Fase::Cuestionario {
                    let q = &PREGUNTAS[m.pregunta];
                    l.texto(
                        &format!("VERIFICACION CIENTIFICA - PREGUNTA {} / 4", m.pregunta + 1),
                        160.0,
                        250.0,
                        24.0,
                        acento,
                    );
                    for (i, t) in q.lineas.iter().enumerate() {
                        l.texto(t, 160.0, 300.0 + i as f32 * 28.0, 24.0, TEXTO);
                    }
                    opciones = q.opciones;
                } else {
                    l.texto(
                        "CUESTIONARIO COMPLETADO - DATOS RECUPERADOS",
                        160.0,
                        250.0,
                        24.0,
                        acento,
                    );
                    l.texto(
                        "Ahora debes regresar a la nave. Elige como escapar de la ola.",
                        160.0,
                        308.0,
                        22.0,
                        TEXTO,
                    );
                    opciones = [
                        "Solicitar ayuda de TARS - transporte asistido rapido.",
                        "Correr hacia la nave - tu controlas el regreso.",
                        "Recuperar equipo adicional - mas peso, menos velocidad.",
                    ];
                }
                for (i, t) in opciones.iter().enumerate() {
                    let y = 388.0 + i as f32 * 68.0;
                    let hover = p.x >= 160.0 && p.x <= 1120.0 && p.y >= y && p.y <= y + 56.0;
                    l.rect(
                        160.0,
                        y,
                        960.0,
                        56.0,
                        if hover {
                            if escape {
                                Color::new(135, 73, 27, 255)
                            } else {
                                Color::new(90, 59, 133, 255)
                            }
                        } else {
                            if escape {
                                Color::new(86, 46, 20, 255)
                            } else {
                                Color::new(54, 34, 82, 255)
                            }
                        },
                    );
                    l.texto(&format!("{}. {}", i + 1, t), 180.0, y + 17.0, 20.0, TEXTO);
                }
                l.texto(
                    "Selecciona con clic o con las teclas 1, 2, 3.",
                    160.0,
                    623.0,
                    18.0,
                    acento,
                );
            }
            Fase::Final(r) => {
                l.rect(120.0, 230.0, 1040.0, 440.0, Color::new(5, 22, 30, 245));
                let (titulo, detalle) = match r {
                    Resultado::Exito => (
                        "MISION COMPLETADA",
                        "Datos cientificos recuperados correctamente.",
                    ),
                    Resultado::Tardio => (
                        "MISION FUERA DE TIEMPO",
                        "Regresaste con los datos, pero excediste el limite de la expedicion.",
                    ),
                    Resultado::Ola => (
                        "MISION FALLIDA - OLA GIGANTE",
                        "La expedicion no logro regresar a tiempo.",
                    ),
                };
                l.texto(
                    titulo,
                    160.0,
                    270.0,
                    30.0,
                    if r == Resultado::Exito {
                        Color::GREEN
                    } else {
                        Color::ORANGE
                    },
                );
                l.texto(detalle, 160.0, 330.0, 21.0, TEXTO);
                l.texto(
                    &format!("Tiempo en Miller: {:.1} segundos.", m.tiempo),
                    160.0,
                    408.0,
                    25.0,
                    TEXTO,
                );
                l.texto(
                    &format!(
                        "Tiempo transcurrido fuera del planeta: {:.2} dias.",
                        m.dias_exteriores()
                    ),
                    160.0,
                    458.0,
                    25.0,
                    CIAN,
                );
                l.texto(
                    "1 hora en Miller equivale a 7 anios fuera del planeta.",
                    160.0,
                    512.0,
                    19.0,
                    TEXTO,
                );
                l.texto(
                    "R: reiniciar la mision | X / Esc: volver al espacio",
                    160.0,
                    600.0,
                    22.0,
                    CIAN,
                );
            }
            _ => {}
        }
        drop(d);
        if reiniciar && !salir {
            self.entrar();
        }
        if salir {
            self.activa = false;
        }
        salir
    }
}
