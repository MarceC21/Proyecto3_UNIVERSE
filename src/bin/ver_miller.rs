// Vista previa de la escena de Miller en una ventana (agua y cajas animadas).
// Flechas: mirar alrededor. R: reiniciar cámara. Esc: salir.
use nave_raster::{paisaje_miller::EscenaMiller, raster::Framebuffer};
use raylib::prelude::*;

const ANCHO: usize = 960;
const ALTO: usize = 600;

fn main() {
    let (mut window, thread) = raylib::init()
        .size(1280, 800)
        .title("Miller - vista previa")
        .build();
    window.set_target_fps(30);

    let escena = EscenaMiller::nueva();
    let mut cam = EscenaMiller::camara(ANCHO, ALTO);
    let mut fb = Framebuffer::new(ANCHO, ALTO);
    let imagen = Image::gen_image_color(ANCHO as i32, ALTO as i32, Color::BLACK);
    let mut tex = window
        .load_texture_from_image(&thread, &imagen)
        .expect("textura");
    tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
    let mut tiempo = 0.0f32;

    while !window.window_should_close() {
        let dt = window.get_frame_time().clamp(0.0005, 0.1);
        tiempo += dt;
        if window.is_key_down(KeyboardKey::KEY_LEFT) {
            cam.yaw -= 1.2 * dt;
        }
        if window.is_key_down(KeyboardKey::KEY_RIGHT) {
            cam.yaw += 1.2 * dt;
        }
        if window.is_key_down(KeyboardKey::KEY_UP) {
            cam.pitch = (cam.pitch + 1.0 * dt).min(1.3);
        }
        if window.is_key_down(KeyboardKey::KEY_DOWN) {
            cam.pitch = (cam.pitch - 1.0 * dt).max(-0.6);
        }
        if window.is_key_pressed(KeyboardKey::KEY_R) {
            cam = EscenaMiller::camara(ANCHO, ALTO);
        }

        escena.dibujar(&mut fb, &cam, tiempo);
        tex.update_texture(&fb.pixels)
            .expect("buffer RGBA incompatible");

        let (sw, sh) = (
            window.get_screen_width() as f32,
            window.get_screen_height() as f32,
        );
        let k = (sw / ANCHO as f32).min(sh / ALTO as f32);
        let (dw, dh) = (ANCHO as f32 * k, ALTO as f32 * k);
        let mut d = window.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture_pro(
            &tex,
            Rectangle::new(0.0, 0.0, ANCHO as f32, ALTO as f32),
            Rectangle::new((sw - dw) * 0.5, (sh - dh) * 0.5, dw, dh),
            Vector2::zero(),
            0.0,
            Color::WHITE,
        );
        d.draw_text(
            &format!("{} FPS | Flechas: mirar  R: reiniciar", d.get_fps()),
            12,
            10,
            16,
            Color::DARKGRAY,
        );
    }
}
