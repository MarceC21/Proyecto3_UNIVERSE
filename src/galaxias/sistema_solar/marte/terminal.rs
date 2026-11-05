// Presentación e interacción de la terminal. Raylib solo dibuja interfaz 2D;
// el paisaje tridimensional viene del framebuffer del rasterizador de Rust.
use crate::mision_marte::{FaseMarte, MisionMarte};
use crate::paisaje_marte::{dibujar_paisaje, dibujar_aterrizaje};
use crate::raster::Framebuffer;
use crate::nave::Nave;
use crate::turing::{estado_texto, Accion, Direccion, Simbolo, REGLAS};
use raylib::prelude::*;

const ESTADOS: [Simbolo; 3] = [Simbolo::Cero, Simbolo::Uno, Simbolo::Blanco];
const CIAN: Color = Color::new(95, 223, 231, 255);
const VERDE: Color = Color::new(131, 239, 163, 255);
const TEXTO: Color = Color::new(231, 234, 227, 255);
const TENUE: Color = Color::new(155, 170, 175, 255);

fn boton(fila: usize, i: usize) -> Rectangle {
    match fila {
        0 => Rectangle::new(84.0, 434.0 + i as f32 * 46.0, 450.0, 38.0),
        1 => Rectangle::new(620.0 + i as f32 * 114.0, 436.0, 102.0, 38.0),
        2 => Rectangle::new(620.0 + i as f32 * 190.0, 504.0, 178.0, 38.0),
        _ => Rectangle::new(620.0 + i as f32 * 285.0, 572.0, 273.0, 38.0),
    }
}
fn contiene(r: Rectangle, p: Vector2) -> bool {
    p.x >= r.x && p.y >= r.y && p.x <= r.x + r.width && p.y <= r.y + r.height
}
fn suave(t: f32) -> f32 { let t = t.clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }

struct Lienzo<'a, D: RaylibDraw> { d: &'a mut D, s: f32, ox: f32, oy: f32 }
impl<D: RaylibDraw> Lienzo<'_, D> {
    fn rect(&mut self, r: Rectangle, c: Color) {
        self.d.draw_rectangle_rec(Rectangle::new(self.ox + r.x * self.s, self.oy + r.y * self.s,
            r.width * self.s, r.height * self.s), c);
    }
    fn borde(&mut self, r: Rectangle, c: Color) {
        self.d.draw_rectangle_lines_ex(Rectangle::new(self.ox + r.x * self.s, self.oy + r.y * self.s,
            r.width * self.s, r.height * self.s), self.s.max(1.0), c);
    }
    fn texto(&mut self, t: &str, x: f32, y: f32, tam: f32, c: Color) {
        self.d.draw_text(t, (self.ox + x * self.s) as i32, (self.oy + y * self.s) as i32,
            (tam * self.s).round().max(1.0) as i32, c);
    }
    fn celda(&mut self, simbolo: Simbolo, x: f32, y: f32, c: Color) {
        if simbolo == Simbolo::Blanco {
            self.borde(Rectangle::new(x + 12.0, y + 6.0, 16.0, 19.0), c);
        } else { self.texto(simbolo.texto(), x + 10.0, y, 30.0, c); }
    }
}

pub struct TerminalMarte {
    fondo: Option<Texture2D>,
    paisaje: Option<Framebuffer>,
    aterrizaje: Option<Texture2D>,
    regla: usize,
    estado: usize,
    simbolo: usize,
    direccion: usize,
    foco: usize,
}
impl Default for TerminalMarte {
    fn default() -> Self {
        Self { fondo: None, paisaje: None, aterrizaje: None, regla: 0, estado: 0, simbolo: 0, direccion: 0, foco: 0 }
    }
}

impl TerminalMarte {
    // Retorna true al elegir despegar. La máquina se conserva entre visitas.
    pub fn cuadro(&mut self, window: &mut RaylibHandle, thread: &RaylibThread,
        mision: &mut MisionMarte, nave: &Nave, tiempo: f32, exterior: Option<&Texture2D>) -> bool {
        if self.fondo.is_none() {
            let fb = dibujar_paisaje(960, 600);
            let img = Image::gen_image_color(960, 600, Color::BLACK);
            let mut tex = window.load_texture_from_image(thread, &img).expect("textura de Marte");
            tex.update_texture(&fb.pixels).expect("framebuffer de Marte");
            tex.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
            self.fondo = Some(tex);
            self.paisaje = Some(fb);
        }
        if mision.fase == FaseMarte::Descenso {
            let fb = dibujar_aterrizaje(self.paisaje.as_ref().unwrap(), nave, mision.descenso);
            if self.aterrizaje.is_none() {
                let img = Image::gen_image_color(960, 600, Color::BLACK);
                let tex = window.load_texture_from_image(thread, &img).expect("textura de aterrizaje");
                tex.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
                self.aterrizaje = Some(tex);
            }
            self.aterrizaje.as_mut().unwrap().update_texture(&fb.pixels).expect("aterrizaje RGBA");
        }
        let (sw, sh) = (window.get_screen_width() as f32, window.get_screen_height() as f32);
        let s = (sw / 1280.0).min(sh / 800.0);
        let (ox, oy) = ((sw - 1280.0 * s) * 0.5, (sh - 800.0 * s) * 0.5);
        let p = window.get_mouse_position();
        let p = Vector2::new((p.x - ox) / s, (p.y - oy) / s);
        let clic = window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let salir = Rectangle::new(985.0, 116.0, 217.0, 34.0);
        let comenzar = Rectangle::new(484.0, 656.0, 312.0, 46.0);
        let ejecutar = Rectangle::new(934.0, 656.0, 252.0, 44.0);
        let activa = mision.fase == FaseMarte::Terminal;
        let bloqueada = mision.maquina.animando() || mision.maquina.aceptada();
        if (clic && contiene(salir, p)) || window.is_key_pressed(KeyboardKey::KEY_ESCAPE)
            || window.is_key_pressed(KeyboardKey::KEY_X) { return true; }
        if mision.fase == FaseMarte::Instrucciones &&
            ((clic && contiene(comenzar, p)) || window.is_key_pressed(KeyboardKey::KEY_ENTER)) {
            mision.comenzar();
        }
        if activa && !bloqueada {
            if window.is_key_pressed(KeyboardKey::KEY_TAB) || window.is_key_pressed(KeyboardKey::KEY_DOWN) {
                self.foco = (self.foco + 1) % 4;
            }
            if window.is_key_pressed(KeyboardKey::KEY_UP) { self.foco = (self.foco + 3) % 4; }
            let cambio = window.is_key_pressed(KeyboardKey::KEY_RIGHT) as i32
                - window.is_key_pressed(KeyboardKey::KEY_LEFT) as i32;
            let cantidades = [4, 5, 3, 2];
            let seleccion = match self.foco { 0 => &mut self.regla, 1 => &mut self.estado,
                2 => &mut self.simbolo, _ => &mut self.direccion };
            *seleccion = (*seleccion as i32 + cambio).rem_euclid(cantidades[self.foco]) as usize;
            for (i, tecla) in [KeyboardKey::KEY_ONE, KeyboardKey::KEY_TWO, KeyboardKey::KEY_THREE, KeyboardKey::KEY_FOUR].iter().enumerate() {
                if window.is_key_pressed(*tecla) { self.regla = i; self.foco = 0; }
            }
            for fila in 0..4 {
                for i in 0..cantidades[fila] as usize {
                    if clic && contiene(boton(fila, i), p) {
                        self.foco = fila;
                        match fila { 0 => self.regla = i, 1 => self.estado = i,
                            2 => self.simbolo = i, _ => self.direccion = i }
                    }
                }
            }
            if (clic && contiene(ejecutar, p)) || window.is_key_pressed(KeyboardKey::KEY_ENTER) {
                mision.maquina.ejecutar(self.regla, Accion { estado: self.estado,
                    escribe: ESTADOS[self.simbolo], mueve: if self.direccion == 0 {
                        Direccion::Izquierda } else { Direccion::Derecha } });
            }
        }

        let mut d = window.begin_drawing(thread);
        d.clear_background(Color::BLACK);
        let fondo = if mision.fase != FaseMarte::Terminal {
            self.aterrizaje.as_ref().or(self.fondo.as_ref()).unwrap()
        } else { self.fondo.as_ref().unwrap() };
        let destino = Rectangle::new(ox, oy, 1280.0 * s, 800.0 * s);
        d.draw_texture_pro(fondo, Rectangle::new(0.0, 0.0, fondo.width as f32, fondo.height as f32),
            destino, Vector2::zero(), 0.0, Color::WHITE);
        if mision.fase == FaseMarte::Descenso {
            if let Some(tex) = exterior {
                let alfa = ((1.0 - suave(mision.descenso * 6.0)) * 255.0) as u8;
                d.draw_texture_pro(tex, Rectangle::new(0.0, 0.0, tex.width as f32, tex.height as f32),
                    destino, Vector2::zero(), 0.0, Color::new(255, 255, 255, alfa));
            }
            let mut l = Lienzo { d: &mut d, s, ox, oy };
            l.rect(Rectangle::new(0.0, 0.0, 1280.0, 110.0), Color::new(5, 10, 14, 230));
            l.rect(Rectangle::new(0.0, 650.0, 1280.0, 150.0), Color::new(5, 10, 14, 230));
            l.texto("MARTE / DESCENSO AUTOMÁTICO", 64.0, 45.0, 24.0, TEXTO);
            l.texto(if mision.descenso < 0.4 { "Aproximación a la superficie. Estabilizando la nave..." }
                else if mision.descenso < 0.86 { "Descenso controlado. Preparando contacto con el suelo..." }
                else { "CONTACTO CONFIRMADO. Motores en reposo." }, 64.0, 685.0, 20.0, TEXTO);
            l.texto("X para volver al vuelo", 64.0, 765.0, 15.0, TENUE);
            l.rect(Rectangle::new(64.0, 733.0, 1152.0, 4.0), Color::new(65, 77, 80, 255));
            l.rect(Rectangle::new(64.0, 733.0, 1152.0 * mision.descenso, 4.0), CIAN);
            return false;
        }
        if mision.fase == FaseMarte::Instrucciones {
            let mut l = Lienzo { d: &mut d, s, ox, oy };
            l.rect(Rectangle::new(42.0, 98.0, 1196.0, 650.0), Color::new(8, 16, 23, 238));
            l.borde(Rectangle::new(42.0, 98.0, 1196.0, 650.0), CIAN);
            l.texto("TERMINAL DE NAVEGACIÓN / MARTE", 74.0, 124.0, 26.0, TEXTO);
            l.rect(salir, Color::new(27, 40, 47, 255));
            l.texto("SALIR [X / ESC]", salir.x + 18.0, salir.y + 10.0, 16.0, TEXTO);
            l.texto("PROTOCOLO DE COMPUTACIÓN", 84.0, 195.0, 24.0, CIAN);
            l.texto("El motor interestelar esta bloqueado. Para activarlo, ejecuta una máquina de Turing.",
                84.0, 237.0, 19.0, TEXTO);
            l.texto("COMO SE COMPONE", 84.0, 294.0, 19.0, CIAN);
            let composicion = [
                "Cinta: cinco celdas con 0, 1 o blanco (_).",
                "Cabezal: lee y escribe una celda cada vez.",
                "Estados: Q = {q0, q1, q2, q3, qf}.",
                "Entrada: {0, 1}. Cinta: {0, 1, blanco}.",
                "Inicio: q0, cabezal sobre el primer 0.",
                "Aceptación: qf, el protocolo esta completo.",
            ];
            for (i, texto) in composicion.iter().enumerate() {
                l.texto(texto, 84.0, 337.0 + i as f32 * 34.0, 17.0, TEXTO);
            }
            l.texto("QUE TIENES QUE HACER", 670.0, 294.0, 19.0, CIAN);
            let objetivo = [
                "1. Mira el estado y el simbolo del cabezal.",
                "2. Selecciona la regla que coincide con ellos.",
                "3. Elige estado siguiente, simbolo y dirección.",
                "4. Pulsa Ejecutar para aplicar la transición.",
                "Completa cuatro transiciones hasta llegar a qf.",
                "Un error no borra lo que ya completaste.",
            ];
            for (i, texto) in objetivo.iter().enumerate() {
                l.texto(texto, 670.0, 337.0 + i as f32 * 34.0, 17.0, TEXTO);
            }
            l.texto("COMO LEER UNA REGLA", 84.0, 562.0, 16.0, CIAN);
            l.texto("(q0, 0) -> (q1, 1, R): si lees 0 en q0, escribe 1, mueve a la derecha y pasa a q1.",
                84.0, 590.0, 18.0, TEXTO);
            l.texto("L = izquierda / R = derecha. Solo seleccionas opciones; no necesitas escribir respuestas.",
                84.0, 619.0, 17.0, TENUE);
            l.rect(comenzar, CIAN);
            l.texto("COMENZAR [ENTER]", comenzar.x + 54.0, comenzar.y + 14.0, 20.0, Color::new(5, 24, 29, 255));
            return false;
        }
        let m = &mision.maquina;
        let acento = if m.aceptada() { VERDE } else if m.animando() {
            let pulso = (m.progreso() * std::f32::consts::PI).sin();
            Color::new((95.0 + pulso * 130.0) as u8, 223, (231.0 - pulso * 80.0) as u8, 255)
        } else { CIAN };
        let mut l = Lienzo { d: &mut d, s, ox, oy };
        // Marco de terminal dejando el paisaje visible alrededor.
        l.rect(Rectangle::new(42.0, 98.0, 1196.0, 650.0), Color::new(8, 16, 23, 226));
        l.borde(Rectangle::new(42.0, 98.0, 1196.0, 650.0), Color::new(79, 111, 115, 255));
        l.rect(Rectangle::new(42.0, 98.0, 5.0, 650.0), acento);
        l.texto("MARTE", 48.0, 37.0, 38.0, TEXTO);
        l.texto("SUPERFICIE / ENLACE ESTABLECIDO", 224.0, 52.0, 16.0, Color::new(255, 209, 167, 255));
        l.texto("TERMINAL DE NAVEGACIÓN", 72.0, 119.0, 26.0, TEXTO);
        l.rect(salir, Color::new(27, 40, 47, 255));
        l.borde(salir, if contiene(salir, p) { acento } else { TENUE });
        l.texto("SALIR [X / ESC]", salir.x + 15.0, salir.y + 10.0, 16.0, TEXTO);
        l.texto(if m.aceptada() { "PROTOCOLO ACEPTADO / MOTOR INTERESTELAR ACTIVADO" }
            else { "Motor bloqueado. Completa el protocolo de computación." }, 74.0, 170.0, 19.0, acento);
        l.texto("01 / CINTA DE EJECUCIÓN", 510.0, 207.0, 16.0, TENUE);
        l.texto(&format!("ESTADO {}   /   LEE {}   /   PASO {}/4", estado_texto(m.estado()),
            m.cinta()[m.cabezal()].texto(), m.pasos()), 435.0, 236.0, 16.0, acento);

        let progreso = m.progreso();
        for (i, simbolo) in m.cinta().iter().enumerate() {
            let x = 316.0 + i as f32 * 132.0;
            let actual = i == m.cabezal();
            let c = if actual { acento } else { TEXTO };
            l.rect(Rectangle::new(x, 269.0, 120.0, 64.0), if actual {
                Color::new(17, 48, 59, 255) } else { Color::new(13, 25, 33, 255) });
            l.borde(Rectangle::new(x, 269.0, 120.0, 64.0), if actual { acento } else { Color::new(49, 66, 74, 255) });
            let mostrado = if actual && progreso > 0.28 { m.pendiente().map(|a| a.escribe).unwrap_or(*simbolo) } else { *simbolo };
            l.celda(mostrado, x + 40.0, 286.0, c);
            if actual && m.animando() && progreso < 0.35 {
                l.rect(Rectangle::new(x + 8.0, 273.0 + progreso * 155.0, 104.0, 3.0), acento);
            }
        }
        let movimiento = m.pendiente().map(|a| if a.mueve == Direccion::Derecha { 1.0 } else { -1.0 }).unwrap_or(0.0);
        let cabeza = m.cabezal() as f32 + movimiento * suave((progreso - 0.35) / 0.55);
        let x = 316.0 + cabeza * 132.0;
        l.rect(Rectangle::new(x + 33.0, 340.0, 54.0, 4.0), acento);
        l.texto("CABEZAL", x + 24.0, 351.0, 13.0, acento);
        l.texto("Blanco = cuadrado vacio / _ en las reglas", 470.0, 376.0, 14.0, TENUE);

        let titulos = ["", "¿CUÁL SERA EL SIGUIENTE ESTADO?", "¿QUÉ SIMBOLO ESCRIBIRAS?", "¿HACIA DÓNDE TE MUEVES?"];
        for fila in 1..4 {
            let y = [0.0, 411.0, 479.0, 547.0][fila];
            l.texto(titulos[fila], 620.0, y, 14.0, if self.foco == fila { acento } else { TENUE });
            let cuenta = [0, 5, 3, 2][fila];
            let seleccionado = [0, self.estado, self.simbolo, self.direccion][fila];
            for i in 0..cuenta {
                let r = boton(fila, i);
                let elegido = i == seleccionado;
                l.rect(r, if elegido { Color::new(23, 65, 72, 255) } else { Color::new(20, 32, 40, 255) });
                l.borde(r, if elegido || (!bloqueada && contiene(r, p)) { acento } else { Color::new(52, 66, 74, 255) });
                let texto = match fila { 1 => estado_texto(i), 2 => ["0", "1", "Blanco"][i], _ => ["< IZQUIERDA", "DERECHA >"][i] };
                l.texto(texto, r.x + 18.0, r.y + 13.0, 18.0, if elegido { acento } else { TEXTO });
            }
        }
        l.rect(ejecutar, if bloqueada { Color::new(33, 51, 55, 255) } else { acento });
        l.texto(if m.animando() { "EJECUTANDO..." } else if m.aceptada() { "ACEPTADO" } else { "EJECUTAR [ENTER]" },
            ejecutar.x + 18.0, ejecutar.y + 13.0, 17.0, Color::new(5, 24, 29, 255));

        l.texto("REGLAS / SELECCIONA UNA", 84.0, 411.0, 16.0, if self.foco == 0 { acento } else { TENUE });
        for (i, regla) in REGLAS.iter().enumerate() {
            let r = boton(0, i);
            l.rect(r, if self.regla == i { Color::new(26, 61, 67, 255) } else { Color::new(17, 29, 37, 255) });
            l.borde(r, if self.regla == i { acento } else { Color::new(42, 59, 66, 255) });
            let texto = format!("{}  ({},{}) -> ({},{},{})", i + 1, estado_texto(regla.desde), regla.lee.texto(),
                estado_texto(regla.accion.estado), regla.accion.escribe.texto(),
                if regla.accion.mueve == Direccion::Derecha { "R" } else { "L" });
            l.texto(&texto, r.x + 24.0, r.y + 11.0, 18.0, TEXTO);
        }
        l.texto("R: derecha / L: izquierda", 84.0, 625.0, 14.0, TENUE);
        l.texto("REACTOR / SECUENCIA DE ENCENDIDO", 84.0, 653.0, 14.0, TENUE);
        let carga = m.pasos() as f32 + if m.animando() { progreso } else { 0.0 };
        for i in 0..4 {
            let r = Rectangle::new(84.0 + i as f32 * 200.0, 680.0, 188.0, 17.0);
            l.rect(r, Color::new(29, 44, 50, 255));
            l.rect(Rectangle::new(r.x, r.y, r.width * (carga - i as f32).clamp(0.0, 1.0), r.height), acento);
        }
        let aviso = if m.aceptada() { "qf alcanzado. Motor listo. Puedes despegar." }
            else if m.error { "Transicion inválida. Revisa la regla y vuelve a intentar." }
            else if m.animando() { "Escribiendo simbolo y desplazando el cabezal..." }
            else { "Selecciona la regla y construye su acción" };
        l.texto(aviso, 84.0, 721.0, 17.0, if m.error { Color::new(255, 142, 110, 255) } else { acento });
        l.texto("CLIC seleccionar  /  TAB o ARRIBA-ABAJO campo  /  IZQ-DER opcion  /  ENTER ejecutar  /  X salir", 48.0, 766.0, 15.0, TEXTO);
        // Motas discretas por encima del marco, sin afectar la interacción.
        for i in 0..16 {
            let x = (i as f32 * 89.0 + tiempo * 8.0).rem_euclid(1280.0);
            let y = 20.0 + (i as f32 * 19.0).rem_euclid(66.0);
            l.rect(Rectangle::new(x, y, 2.0, 1.0), Color::new(255, 211, 166, 90));
        }
        false
    }
}
