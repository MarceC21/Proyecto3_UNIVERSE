// Una sola traducción de teclado a Control, compartida por vuelo y minijuegos.
use crate::nave::Control;
use raylib::prelude::*;
pub fn leer(window: &RaylibHandle, raton: (f32, f32)) -> Control {
    let eje = |a, b| (window.is_key_down(a) as i32 - window.is_key_down(b) as i32) as f32;
    Control {
        empuje: eje(KeyboardKey::KEY_W, KeyboardKey::KEY_S),
        lateral: eje(KeyboardKey::KEY_D, KeyboardKey::KEY_A),
        vertical: eje(KeyboardKey::KEY_SPACE, KeyboardKey::KEY_LEFT_CONTROL),
        cabeceo: eje(KeyboardKey::KEY_UP, KeyboardKey::KEY_DOWN),
        guinada: eje(KeyboardKey::KEY_RIGHT, KeyboardKey::KEY_LEFT),
        alabeo: eje(KeyboardKey::KEY_E, KeyboardKey::KEY_Q),
        raton,
        turbo: window.is_key_down(KeyboardKey::KEY_LEFT_SHIFT),
    }
}
