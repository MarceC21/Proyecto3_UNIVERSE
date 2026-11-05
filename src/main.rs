// Ventana (raylib) con el sistema solar y la nave, dibujados con NUESTRO rasterizador.
// raylib solo abre la ventana, lee teclado/ratón y muestra el framebuffer + HUD.
use nave_raster::{
    camara::{Camara, Entrada},
    cielo::Cielo,
    dibujo::Ajustes,
    escena::{Escena, ALTO_LIMITE, CINTURON_EXT, CINTURON_INT, RADIO_LIMITE},
    math::{v3, V3},
    nave::{Control, Mundo, Nave},
    raster::Framebuffer,
    render::dibujar_con_nave as dibujar,
};
use nave_raster::seguimiento::Seguimiento;
use nave_raster::mision_marte::{FaseMarte, MisionMarte};
use nave_raster::terminal_marte::TerminalMarte;
use nave_raster::progreso::Progreso;
use nave_raster::salto::Salto;
use nave_raster::mapa::Mapa;
use raylib::prelude::*;

fn completar_marte(mision: &mut MisionMarte, progreso: &mut Progreso) {
    if mision.maquina.aceptada() {
        mision.marcar_completada();
        progreso.completar_marte();
    }
}

fn resolucion(calidad: usize) -> (usize, usize) {
    match calidad { 1 => (960, 600), 2 => (720, 450), _ => (480, 300) }
}

fn main() {
    let ruta = std::env::args().nth(1);
    let mut nave = match ruta.as_deref() {
        Some(ruta) => Nave::cargar(ruta),
        None => Nave::predeterminada(),
    }.unwrap_or_else(|error| { eprintln!("No se pudo cargar la nave: {error}"); std::process::exit(1) });
    let (mut window, thread) = raylib::init().size(1280, 800).title("Sistema solar - rasterizador por software").build();
    window.set_target_fps(60);
    // Escape vuelve al vuelo desde la terminal; en vuelo cierra el programa.
    window.set_exit_key(None);
    window.disable_cursor();

    println!("Generando texturas y sistema solar...");
    let mut escena = Escena::nueva();
    let cielo = Cielo::nuevo();

    let mut calidad = 1usize;
    let (mut rw, mut rh) = resolucion(calidad);
    let mut fb = Framebuffer::new(rw, rh);
    let mut cam = Camara::nueva(rw as f32, rh as f32);
    let mut seguimiento = Seguimiento::nuevo(nave.fwd, nave.up);
    let vistas = escena.vistas();

    let mut textura: Option<Texture2D> = None;
    let mut aj = Ajustes::default();
    let mut tiempo = 0.0f32;
    let mut mouse = true;
    let mut camara_libre = false;
    let mut mision_marte = MisionMarte::default();
    let mut terminal_marte = TerminalMarte::default();
    // Cada ejecución es una partida nueva; el avance vive solo en memoria.
    let mut progreso = Progreso::default();
    let mut salto = Salto::default();
    let mut mapa = Mapa::default();
    let mut mapa_abierto = false;

    while !window.window_should_close() {
        let dt = window.get_frame_time().clamp(0.0005, 0.1);
        tiempo += dt;

        // La misión conserva su progreso entre visitas y consume su propia
        // entrada: al seleccionar botones nunca se mueve ni daña la nave.
        if mision_marte.fase != FaseMarte::Espacio {
            mision_marte.actualizar(dt);
            completar_marte(&mut mision_marte, &mut progreso);
            if terminal_marte.cuadro(&mut window, &thread, &mut mision_marte, &nave, tiempo, textura.as_ref()) {
                mision_marte.despegar();
                completar_marte(&mut mision_marte, &mut progreso);
                let marte = escena.cuerpos.iter().find(|c| c.nombre == "Marte").expect("falta Marte");
                let radial = (escena.pos_sol() - marte.pos).norm();
                let pos = marte.pos + radial * (marte.radio + 12.0);
                nave.teletransportar(pos, pos + radial);
                nave.modo_local = false;
                seguimiento.reiniciar(nave.fwd, nave.up);
                if mouse { window.disable_cursor(); } else { window.enable_cursor(); }
            }
            continue;
        }
        // Abrir y dibujar en el mismo cuadro: EndDrawing actualiza el teclado.
        let mapa_recien_abierto = !mapa_abierto && window.is_key_pressed(KeyboardKey::KEY_M);
        if mapa_recien_abierto {
            mapa_abierto = true;
            window.enable_cursor();
        }
        if mapa_abierto {
            salto.actualizar(dt);
            let cerrar = mapa.cuadro(&mut window, &thread, &mut progreso, &salto, tiempo, textura.as_ref(), mapa_recien_abierto);
            if cerrar {
                mapa_abierto = false;
                if salto.solicitud().is_some() {
                    // Mientras no exista la próxima escena, conservar el evento
                    // durante el aviso y regresar a una posición segura al cerrar.
                    salto.volver();
                    let radial = v3(nave.pos.x, 0.0, nave.pos.z).norm();
                    let pos = radial * (RADIO_LIMITE - 22.0) + v3(0.0, nave.pos.y, 0.0);
                    nave.teletransportar(pos, pos - radial);
                    seguimiento.reiniciar(nave.fwd, nave.up);
                }
                if mouse { window.disable_cursor(); } else { window.enable_cursor(); }
            }
            continue;
        }
        if window.is_key_pressed(KeyboardKey::KEY_ESCAPE) { break; }

        // ----- Interruptores -----
        if window.is_key_pressed(KeyboardKey::KEY_F1) { calidad = 1; }
        if window.is_key_pressed(KeyboardKey::KEY_F2) { calidad = 2; }
        if window.is_key_pressed(KeyboardKey::KEY_F3) { calidad = 3; }
        if window.is_key_pressed(KeyboardKey::KEY_C) { aj.culling = !aj.culling; }
        if window.is_key_pressed(KeyboardKey::KEY_L) { aj.lod = !aj.lod; }
        if window.is_key_pressed(KeyboardKey::KEY_O) { aj.orbitas = !aj.orbitas; }
        if window.is_key_pressed(KeyboardKey::KEY_B) { aj.limite = !aj.limite; }
        if window.is_key_pressed(KeyboardKey::KEY_F) { nave.modo_local = !nave.modo_local; }
        if window.is_key_pressed(KeyboardKey::KEY_R) { nave.reaparecer(); seguimiento.reiniciar(nave.fwd, nave.up); }
        if window.is_key_pressed(KeyboardKey::KEY_TAB) {
            mouse = !mouse;
            if mouse { window.disable_cursor(); } else { window.enable_cursor(); }
        }
        if window.is_key_pressed(KeyboardKey::KEY_V) {
            camara_libre = !camara_libre;
            cam.vel = V3::ZERO;
            if camara_libre {
                // La cámara libre arranca donde estaba la de persecución.
                let (f, _, _) = cam.base();
                cam.pitch = f.y.clamp(-1.0, 1.0).asin();
                cam.yaw = f.x.atan2(-f.z);
                cam.base_ext = None;
                cam.fov = 62f32.to_radians();
            }
        }
        // Se conservan todos los destinos del sistema actual. T visita la estación; E es alabeo.
        let teclas = [KeyboardKey::KEY_ZERO, KeyboardKey::KEY_ONE, KeyboardKey::KEY_TWO, KeyboardKey::KEY_THREE,
                      KeyboardKey::KEY_FOUR, KeyboardKey::KEY_FIVE, KeyboardKey::KEY_EIGHT, KeyboardKey::KEY_NINE,
                      KeyboardKey::KEY_U, KeyboardKey::KEY_N, KeyboardKey::KEY_SIX, KeyboardKey::KEY_SEVEN,
                      KeyboardKey::KEY_T];
        for (i, k) in teclas.iter().enumerate() {
            if window.is_key_pressed(*k) && i < vistas.len() {
                if camara_libre {
                    cam.pos = vistas[i].1;
                    cam.vel = V3::ZERO;
                    cam.mirar_a(vistas[i].2);
                } else {
                    nave.teletransportar(vistas[i].1, vistas[i].2);
                    seguimiento.reiniciar(nave.fwd, nave.up);
                }
            }
        }

        let (nw, nh) = resolucion(calidad);
        if (nw, nh) != (rw, rh) {
            rw = nw;
            rh = nh;
            fb.redimensionar(rw, rh);
            cam.ajustar_resolucion(rw as f32, rh as f32);
            textura = None;
        }

        // ----- Entrada -----
        let eje = |mas: KeyboardKey, menos: KeyboardKey, w: &RaylibHandle| {
            (w.is_key_down(mas) as i32 - w.is_key_down(menos) as i32) as f32
        };
        let raton = if mouse { window.get_mouse_delta() } else { Vector2::zero() };
        let orbitando = !camara_libre && window.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT);

        escena.actualizar(dt);
        let cuerpos = escena.esferas();
        if camara_libre {
            let mut giro = raton;
            giro.x += 420.0 * dt * eje(KeyboardKey::KEY_RIGHT, KeyboardKey::KEY_LEFT, &window);
            giro.y += 420.0 * dt * eje(KeyboardKey::KEY_UP, KeyboardKey::KEY_DOWN, &window);
            let entrada = Entrada {
                adelante: eje(KeyboardKey::KEY_W, KeyboardKey::KEY_S, &window),
                lateral: eje(KeyboardKey::KEY_D, KeyboardKey::KEY_A, &window),
                vertical: eje(KeyboardKey::KEY_SPACE, KeyboardKey::KEY_LEFT_CONTROL, &window),
                giro_x: giro.x,
                giro_y: giro.y,
                turbo: window.is_key_down(KeyboardKey::KEY_LEFT_SHIFT),
            };
            cam.actualizar(&entrada, dt, &cuerpos, RADIO_LIMITE, ALTO_LIMITE);
        } else {
            let control = Control {
                empuje: eje(KeyboardKey::KEY_W, KeyboardKey::KEY_S, &window),
                lateral: eje(KeyboardKey::KEY_D, KeyboardKey::KEY_A, &window),
                vertical: eje(KeyboardKey::KEY_SPACE, KeyboardKey::KEY_LEFT_CONTROL, &window),
                cabeceo: eje(KeyboardKey::KEY_UP, KeyboardKey::KEY_DOWN, &window),
                guinada: eje(KeyboardKey::KEY_RIGHT, KeyboardKey::KEY_LEFT, &window),
                alabeo: eje(KeyboardKey::KEY_E, KeyboardKey::KEY_Q, &window),
                raton: if orbitando { (0.0, 0.0) } else { (raton.x, raton.y) },
                turbo: window.is_key_down(KeyboardKey::KEY_LEFT_SHIFT),
            };
            let mundo = Mundo {
                cuerpos: &cuerpos,
                asteroides: escena.colisionadores_asteroides(),
                radio_limite: RADIO_LIMITE,
                alto_limite: ALTO_LIMITE,
                cinturon: (CINTURON_INT, CINTURON_EXT),
            };
            nave.actualizar(&control, dt, &mundo);
            seguimiento.actualizar(&mut cam, nave.pos, nave.fwd, nave.up, nave.velocidad(), dt,
                if orbitando { Some((raton.x, raton.y)) } else { None }, &cuerpos);
        }

        let marte = escena.cuerpos.iter().find(|c| c.nombre == "Marte").expect("falta Marte");
        let altura_marte = (nave.pos - marte.pos).len() - marte.radio;
        if mision_marte.observar(altura_marte, nave.velocidad(), !camara_libre && nave.destruida <= 0.0) {
            nave.vel = V3::ZERO;
            window.enable_cursor();
        }
        let distancia_borde_nave = RADIO_LIMITE - (nave.pos.x * nave.pos.x + nave.pos.z * nave.pos.z).sqrt();
        if salto.observar(&progreso, distancia_borde_nave,
            !camara_libre && nave.destruida <= 0.0 && mision_marte.fase == FaseMarte::Espacio) {
            nave.vel = V3::ZERO;
            nave.modo_local = false;
            mapa_abierto = true;
            window.enable_cursor();
        }

        let st = dibujar(&mut fb, &escena, &cielo, &cam, tiempo, &aj, Some(&nave));

        if textura.is_none() {
            let img = Image::gen_image_color(rw as i32, rh as i32, Color::BLACK);
            let t = window.load_texture_from_image(&thread, &img).expect("no se pudo crear la textura");
            t.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
            textura = Some(t);
        }
        let tex = textura.as_mut().unwrap();
        tex.update_texture(&fb.pixels).expect("buffer RGBA incompatible");

        let (sw, sh) = (window.get_screen_width() as f32, window.get_screen_height() as f32);
        let escala = (sw / rw as f32).min(sh / rh as f32);
        let (dw, dh) = (rw as f32 * escala, rh as f32 * escala);
        let (ox, oy) = ((sw - dw) * 0.5, (sh - dh) * 0.5);
        let referencia = if camara_libre { cam.pos } else { nave.pos };
        let (cercano, alt) = escena.cuerpo_cercano(referencia);
        let dist_lim = RADIO_LIMITE - (referencia.x * referencia.x + referencia.z * referencia.z).sqrt();
        let fps = window.get_fps();
        let punto_mira = cam.proyectar(nave.pos + nave.fwd * 40.0);

        let mut d = window.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture_pro(&*tex, Rectangle::new(0.0, 0.0, rw as f32, rh as f32),
            Rectangle::new(ox, oy, dw, dh), Vector2::zero(), 0.0, Color::WHITE);

        // Etiquetas de los cuerpos proyectadas a pantalla.
        for c in &escena.cuerpos {
            if let Some((sx, sy, w)) = cam.proyectar(c.pos + v3(0.0, c.radio * 1.5, 0.0)) {
                if w < 400.0 {
                    let (x, y) = ((ox + sx * escala) as i32, (oy + sy * escala) as i32);
                    let t = format!("{} ({:.0})", c.nombre, (c.pos - referencia).len());
                    d.draw_text(&t, x - (t.len() as i32) * 3, y - 14, 16, Color::new(255, 255, 255, 230));
                }
            }
        }
        let estacion = &escena.estacion;
        if let Some((sx, sy, profundidad)) = cam.proyectar(estacion.pos + v3(0.0, estacion.radio, 0.0)) {
            if profundidad < 160.0 {
                let texto = format!("Estacion ({:.0})", (referencia - estacion.pos).len());
                d.draw_text(&texto, (ox + sx * escala) as i32 - 50,
                    (oy + sy * escala) as i32 - 14, 16, Color::new(100, 240, 245, 230));
            }
        }
        // Mira: hacia dónde apunta la nariz.
        if let Some((sx, sy, _)) = punto_mira {
            let (x, y) = ((ox + sx * escala) as i32, (oy + sy * escala) as i32);
            let c = Color::new(120, 255, 240, 200);
            d.draw_rectangle(x - 10, y - 1, 7, 2, c);
            d.draw_rectangle(x + 4, y - 1, 7, 2, c);
            d.draw_rectangle(x - 1, y - 10, 2, 7, c);
            d.draw_rectangle(x - 1, y + 4, 2, 7, c);
        }

        d.draw_rectangle(8, 8, 760, 172, Color::new(0, 0, 0, 170));
        d.draw_text(&format!("{} FPS | velocidad {:.1} / {:.0} | cerca de {} ({:.1})",
            fps, nave.velocidad(), nave.velocidad_max, cercano, alt), 16, 14, 16, Color::WHITE);
        d.draw_text(&format!("Objetos {}/{}   Triangulos {} de {} (sin culling/LOD)",
            st.objetos_enviados, st.objetos_total, st.tris_enviados, st.tris_sin_optimizar), 16, 36, 16, Color::WHITE);
        d.draw_text(&format!("[C]ulling {}  [L]OD {}  [O]rbitas {}  [B]limite {}  {}",
            if aj.culling { "ON" } else { "off" }, if aj.lod { "ON" } else { "off" },
            if aj.orbitas { "ON" } else { "off" }, if aj.limite { "ON" } else { "off" },
            if camara_libre { "[V] CAMARA LIBRE" } else { "[V] camara libre" }), 16, 58, 16, Color::WHITE);
        d.draw_text("WASD mover  Espacio/Ctrl subir/bajar  Q/E alabeo  Raton/flechas girar  Shift turbo", 16, 84, 14, Color::LIGHTGRAY);
        d.draw_text("Clic derecho orbitar camara  F modo local  R reaparecer  V camara libre  TAB cursor", 16, 104, 14, Color::LIGHTGRAY);

        d.draw_text("0 general  1 Sol 2 Mercurio 3 Venus 4 Tierra 5 Marte 6 cinturon 7 limite", 16, 124, 14, Color::LIGHTGRAY);
        d.draw_text("8 Jupiter  9 Saturno  U Urano  N Neptuno  T Estacion  M Mapa  F1-F3 calidad", 16, 144, 14, Color::LIGHTGRAY);

        // Modo local (rodear un cuerpo).
        let objetivo_local = escena.cuerpos.iter().min_by(|a, b|
            (a.pos - nave.objetivo).len().total_cmp(&(b.pos - nave.objetivo).len()))
            .map(|c| c.nombre).unwrap_or(cercano);
        let local_txt = if camara_libre { None } else if nave.local_activo { Some(format!("MODO LOCAL: orbitando {objetivo_local}  (A/D/Espacio rodean, W/S acercan)")) }
            else if nave.modo_local { Some("MODO LOCAL activado: acércate a un cuerpo".to_string()) }
            else if alt < 25.0 { Some(format!("[F] modo local para rodear {cercano}")) } else { None };
        if let Some(t) = local_txt {
            d.draw_text(&t, 16, (sh as i32) - 60, 18, Color::new(120, 255, 240, 255));
        }
        if !camara_libre && altura_marte < 18.0 && !mision_marte.completada() {
            d.draw_text("MARTE: acercate a menos de 6.5 y reduce la velocidad a 6 para aterrizar",
                16, sh as i32 - 88, 17, Color::new(255, 190, 132, 255));
        }
        if progreso.motor_activo() {
            d.draw_text("MOTOR LISTO / [M] MAPA INTERESTELAR", sw as i32 - 400, sh as i32 - 30,
                17, Color::new(131, 239, 163, 255));
        }

        // Integridad de la nave.
        let barra = (nave.integridad / 100.0).clamp(0.0, 1.0);
        d.draw_rectangle(16, sh as i32 - 34, 204, 18, Color::new(0, 0, 0, 170));
        let col = if barra > 0.5 { Color::new(90, 230, 140, 230) } else if barra > 0.25 { Color::new(250, 200, 70, 230) } else { Color::new(255, 70, 70, 230) };
        d.draw_rectangle(18, sh as i32 - 32, (200.0 * barra) as i32, 14, col);
        d.draw_text("INTEGRIDAD", 24, sh as i32 - 32, 12, Color::BLACK);

        if nave.golpe > 0.0 {
            d.draw_rectangle(0, 0, sw as i32, sh as i32, Color::new(255, 40, 40, (nave.golpe * 160.0) as u8));
        }
        if nave.destruida > 0.0 {
            d.draw_text("NAVE DESTRUIDA", (sw as i32 - 300) / 2, sh as i32 / 2 - 20, 40, Color::new(255, 80, 80, 255));
        }
        if dist_lim < 25.0 {
            let parpadeo = ((tiempo * 6.0).sin() * 0.5 + 0.5) * 120.0 + 135.0;
            let texto = if progreso.motor_activo() { "LIMITE: RUTA DE SALTO PREPARADA [M]" }
                else { "LIMITE: ACTIVA EL MOTOR EN MARTE" };
            d.draw_text(texto, (sw as i32 - 560) / 2, 192, 24, Color::new(255, 90, 200, parpadeo as u8));
        }
    }
}
