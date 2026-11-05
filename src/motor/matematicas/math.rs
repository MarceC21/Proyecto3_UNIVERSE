// Matemática mínima: vectores 3D y rotaciones. Sin dependencias (ni raylib),
// así el renderer compila y se puede probar sin abrir una ventana.
use std::ops::{Add, AddAssign, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub const fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

impl Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 { v3(self.x + o.x, self.y + o.y, self.z + o.z) }
}
impl Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 { v3(self.x - o.x, self.y - o.y, self.z - o.z) }
}
impl Mul<f32> for V3 {
    type Output = V3;
    fn mul(self, k: f32) -> V3 { v3(self.x * k, self.y * k, self.z * k) }
}
// Producto componente a componente (para multiplicar colores).
impl Mul<V3> for V3 {
    type Output = V3;
    fn mul(self, o: V3) -> V3 { v3(self.x * o.x, self.y * o.y, self.z * o.z) }
}
impl Neg for V3 {
    type Output = V3;
    fn neg(self) -> V3 { v3(-self.x, -self.y, -self.z) }
}
impl AddAssign for V3 {
    fn add_assign(&mut self, o: V3) { *self = *self + o; }
}

impl V3 {
    pub const ZERO: V3 = v3(0.0, 0.0, 0.0);
    pub fn dot(self, o: V3) -> f32 { self.x * o.x + self.y * o.y + self.z * o.z }
    pub fn cross(self, o: V3) -> V3 {
        v3(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    pub fn len(self) -> f32 { self.dot(self).sqrt() }
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l < 1e-9 { self } else { self * (1.0 / l) }
    }
    pub fn lerp(self, o: V3, t: f32) -> V3 { self + (o - self) * t }
}

pub fn clamp01(x: f32) -> f32 { x.clamp(0.0, 1.0) }
pub fn mezcla(a: V3, b: V3, t: f32) -> V3 { a.lerp(b, t) }
pub fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = clamp01((x - a) / (b - a));
    t * t * (3.0 - 2.0 * t)
}
// Color 0..255 -> 0..1
pub fn rgb(r: f32, g: f32, b: f32) -> V3 { v3(r / 255.0, g / 255.0, b / 255.0) }

// Rotación 3x3 guardada por columnas (la imagen de los ejes X, Y, Z).
#[derive(Clone, Copy)]
pub struct Rot {
    pub c: [V3; 3],
}

impl Rot {
    pub fn identidad() -> Rot { Rot { c: [v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0), v3(0.0, 0.0, 1.0)] } }
    pub fn de_columnas(x: V3, y: V3, z: V3) -> Rot { Rot { c: [x, y, z] } }
    pub fn rot_x(a: f32) -> Rot {
        let (s, c) = a.sin_cos();
        Rot { c: [v3(1.0, 0.0, 0.0), v3(0.0, c, s), v3(0.0, -s, c)] }
    }
    pub fn rot_y(a: f32) -> Rot {
        let (s, c) = a.sin_cos();
        Rot { c: [v3(c, 0.0, -s), v3(0.0, 1.0, 0.0), v3(s, 0.0, c)] }
    }
    pub fn rot_z(a: f32) -> Rot {
        let (s, c) = a.sin_cos();
        Rot { c: [v3(c, s, 0.0), v3(-s, c, 0.0), v3(0.0, 0.0, 1.0)] }
    }
    pub fn aplicar(&self, v: V3) -> V3 { self.c[0] * v.x + self.c[1] * v.y + self.c[2] * v.z }
    // self * o : primero se aplica `o`, luego `self`.
    pub fn mul(&self, o: &Rot) -> Rot {
        Rot { c: [self.aplicar(o.c[0]), self.aplicar(o.c[1]), self.aplicar(o.c[2])] }
    }
}

// Gira el vector `v` un ángulo `a` alrededor del eje unitario `eje` (fórmula de Rodrigues).
pub fn girar(v: V3, eje: V3, a: f32) -> V3 {
    let (s, c) = a.sin_cos();
    v * c + eje.cross(v) * s + eje * (eje.dot(v) * (1.0 - c))
}
