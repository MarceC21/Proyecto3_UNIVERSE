// Ventana (raylib) que muestra el sistema solar dibujado con NUESTRO rasterizador.
// raylib solo abre la ventana, lee teclado/ratón y muestra el framebuffer + HUD.
use nave_raster::{
    camara::{Camara, Entrada},
    cielo::Cielo,
    dibujo::Ajustes,
    escena::{Escena, ALTO_LIMITE, RADIO_LIMITE},
    raster::Framebuffer,
    render::dibujar,
};
use raylib::prelude::*;

fn resolucion(calidad: usize) -> (usize, usize) {
    match calidad { 1 => (960, 600), 2 => (720, 450), _ => (480, 300) }
}

fn main() {
    let (mut window, thread) = raylib::init().size(1280, 800).title("Sistema solar - rasterizador por software").build();
    window.set_target_fps(60);
    window.disable_cursor();

    println!("Generando texturas...");
    let mut escena = Escena::nueva();
    let cielo = Cielo::nuevo();

    let mut calidad = 1usize;
    let (mut rw, mut rh) = resolucion(calidad);
    let mut fb = Framebuffer::new(rw, rh);
    let mut cam = Camara::nueva(rw as f32, rh as f32);
    let vistas = escena.vistas();
    cam.pos = vistas[0].1;
    cam.mirar_a(vistas[0].2);

    let mut textura: Option<Texture2D> = None;
    let mut aj = Ajustes::default();
    let mut tiempo = 0.0f32;
    let mut mouse = true;

    while !window.window_should_close() {
        let dt = window.get_frame_time().min(0.1);
        tiempo += dt;

        // ----- Interruptores -----
        if window.is_key_pressed(KeyboardKey::KEY_F1) { calidad = 1; }
        if window.is_key_pressed(KeyboardKey::KEY_F2) { calidad = 2; }
        if window.is_key_pressed(KeyboardKey::KEY_F3) { calidad = 3; }
        if window.is_key_pressed(KeyboardKey::KEY_C) { aj.culling = !aj.culling; }
        if window.is_key_pressed(KeyboardKey::KEY_L) { aj.lod = !aj.lod; }
        if window.is_key_pressed(KeyboardKey::KEY_O) { aj.orbitas = !aj.orbitas; }
        if window.is_key_pressed(KeyboardKey::KEY_B) { aj.limite = !aj.limite; }
        if window.is_key_pressed(KeyboardKey::KEY_TAB) {
            mouse = !mouse;
            if mouse { window.disable_cursor(); } else { window.enable_cursor(); }
        }
        // Se conservan 6=cinturón y 7=límite para los controles existentes.
        let teclas = [KeyboardKey::KEY_ZERO, KeyboardKey::KEY_ONE, KeyboardKey::KEY_TWO, KeyboardKey::KEY_THREE,
                      KeyboardKey::KEY_FOUR, KeyboardKey::KEY_FIVE, KeyboardKey::KEY_EIGHT, KeyboardKey::KEY_NINE,
                      KeyboardKey::KEY_U, KeyboardKey::KEY_N, KeyboardKey::KEY_SIX, KeyboardKey::KEY_SEVEN,
                      KeyboardKey::KEY_E];
        for (i, k) in teclas.iter().enumerate() {
            if window.is_key_pressed(*k) && i < vistas.len() {
                cam.pos = vistas[i].1;
                cam.vel = nave_raster::math::V3::ZERO;
                cam.mirar_a(vistas[i].2);
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
        let mut giro = Vector2::zero();
        if mouse { giro = window.get_mouse_delta(); }
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

        // ----- Actualizar y dibujar -----
        escena.actualizar(dt);
        cam.actualizar(&entrada, dt, &escena.esferas(), RADIO_LIMITE, ALTO_LIMITE);
        let st = dibujar(&mut fb, &escena, &cielo, &cam, tiempo, &aj);

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
        let (cercano, alt) = escena.cuerpo_cercano(cam.pos);
        let dist_lim = cam.distancia_limite(RADIO_LIMITE);
        let fps = window.get_fps();

        let mut d = window.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture_pro(&*tex, Rectangle::new(0.0, 0.0, rw as f32, rh as f32),
            Rectangle::new(ox, oy, dw, dh), Vector2::zero(), 0.0, Color::WHITE);

        // Etiquetas de los cuerpos proyectadas a pantalla.
        for c in &escena.cuerpos {
            if let Some((sx, sy, w)) = cam.proyectar(c.pos + nave_raster::math::v3(0.0, c.radio * 1.5, 0.0)) {
                if w < 400.0 {
                    let (x, y) = ((ox + sx * escala) as i32, (oy + sy * escala) as i32);
                    let t = format!("{} ({:.0})", c.nombre, w);
                    d.draw_text(&t, x - (t.len() as i32) * 3, y - 14, 16, Color::new(255, 255, 255, 230));
                }
            }
        }

        let estacion = &escena.estacion;
        if let Some((sx, sy, profundidad)) = cam.proyectar(estacion.pos + nave_raster::math::v3(0.0, estacion.radio, 0.0)) {
            if profundidad < 160.0 {
                let distancia = (cam.pos - estacion.pos).len();
                let texto = format!("Estacion ({:.0})", distancia);
                d.draw_text(&texto, (ox + sx * escala) as i32 - 50,
                    (oy + sy * escala) as i32 - 14, 16, Color::new(100, 240, 245, 230));
            }
        }

        d.draw_rectangle(8, 8, 670, 146, Color::new(0, 0, 0, 170));
        d.draw_text(&format!("{} FPS | velocidad {:.1} | cerca de {} ({:.1})", fps, cam.velocidad_actual, cercano, alt), 16, 14, 16, Color::WHITE);
        d.draw_text(&format!("Objetos {}/{}   Triangulos {} de {} (sin culling/LOD)", st.objetos_enviados, st.objetos_total, st.tris_enviados, st.tris_sin_optimizar), 16, 36, 16, Color::WHITE);
        d.draw_text(&format!("Rasterizados {}   [C]ulling {}  [L]OD {}  [O]rbitas {}  [B]limite {}",
            st.tris_raster, if aj.culling { "ON" } else { "off" }, if aj.lod { "ON" } else { "off" },
            if aj.orbitas { "ON" } else { "off" }, if aj.limite { "ON" } else { "off" }), 16, 58, 16, Color::WHITE);
        d.draw_text("WASD mover  Espacio/Ctrl subir/bajar  Shift turbo  Raton/flechas mirar  TAB cursor", 16, 84, 14, Color::LIGHTGRAY);
        d.draw_text("0 general  1 Sol 2 Mercurio 3 Venus 4 Tierra 5 Marte 6 cinturon 7 limite", 16, 104, 14, Color::LIGHTGRAY);
        d.draw_text("8 Jupiter  9 Saturno  U Urano  N Neptuno  E Estacion  F1-F3 calidad", 16, 124, 14, Color::LIGHTGRAY);

        if dist_lim < 25.0 {
            let parpadeo = ((tiempo * 6.0).sin() * 0.5 + 0.5) * 120.0 + 135.0;
            let msg = "LIMITE DEL SISTEMA SOLAR";
            d.draw_text(msg, (sw as i32 - 360) / 2, 150, 28, Color::new(255, 90, 200, parpadeo as u8));
        }
    }
}
