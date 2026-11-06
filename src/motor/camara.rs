// Cámara libre con velocidad suave. Sin raylib: la entrada llega en `Entrada`.
use crate::math::*;

pub struct Entrada {
    pub adelante: f32, // -1..1
    pub lateral: f32,
    pub vertical: f32,
    pub giro_x: f32, // píxeles de ratón (o equivalente)
    pub giro_y: f32,
    pub turbo: bool,
}

pub struct Camara {
    pub pos: V3,
    pub yaw: f32,
    pub pitch: f32,
    pub vel: V3,
    pub fov: f32, // vertical, radianes
    pub ancho: f32,
    pub alto: f32,
    pub velocidad_actual: f32,
    // Si está presente, sustituye a la base calculada con yaw/pitch (cámara de persecución con alabeo).
    pub base_ext: Option<(V3, V3, V3)>,
}

const SENSIBILIDAD: f32 = 0.0022;

impl Camara {
    pub fn nueva(ancho: f32, alto: f32) -> Camara {
        Camara { pos: v3(0.0, 60.0, 150.0), yaw: 0.0, pitch: -0.3, vel: V3::ZERO,
                 fov: 62f32.to_radians(), ancho, alto, velocidad_actual: 0.0, base_ext: None }
    }

    pub fn ajustar_resolucion(&mut self, ancho: f32, alto: f32) {
        self.ancho = ancho;
        self.alto = alto;
    }

    // (adelante, derecha, arriba)
    pub fn base(&self) -> (V3, V3, V3) {
        if let Some(b) = self.base_ext { return b; }
        let f = v3(self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), -self.yaw.cos() * self.pitch.cos());
        let r = v3(self.yaw.cos(), 0.0, self.yaw.sin());
        (f, r, r.cross(f).norm())
    }

    // Factores de proyección: x_clip = x_vista * fx, y_clip = y_vista * fy, w = profundidad.
    pub fn factores(&self) -> (f32, f32) {
        let t = (self.fov * 0.5).tan();
        (1.0 / (t * self.ancho / self.alto), 1.0 / t)
    }

    // Píxeles por radian aproximados (para estimar el tamaño en pantalla).
    pub fn escala_px(&self) -> f32 { (self.alto * 0.5) / (self.fov * 0.5).tan() }

    pub fn mirar_a(&mut self, objetivo: V3) {
        self.base_ext = None;
        let d = (objetivo - self.pos).norm();
        self.pitch = d.y.clamp(-1.0, 1.0).asin();
        self.yaw = d.x.atan2(-d.z);
    }

    // Vista "bonita" de una esfera: del lado iluminado, tres cuartos.
    pub fn vista_esfera(&mut self, centro: V3, radio: f32, sol: V3) {
        let hacia_sol = (sol - centro).norm();
        let tang = v3(0.0, 1.0, 0.0).cross(hacia_sol).norm();
        let dir = (hacia_sol * 0.8 + tang * 0.6 + v3(0.0, 0.18, 0.0)).norm();
        self.pos = centro + dir * (radio * 3.6);
        self.vel = V3::ZERO;
        self.mirar_a(centro);
    }

    // Proyecta un punto a píxeles. None si está detrás de la cámara.
    pub fn proyectar(&self, p: V3) -> Option<(f32, f32, f32)> {
        let (f, r, u) = self.base();
        let d = p - self.pos;
        let w = d.dot(f);
        if w < 0.1 { return None; }
        let (fx, fy) = self.factores();
        Some(((d.dot(r) * fx / w * 0.5 + 0.5) * self.ancho, (0.5 - d.dot(u) * fy / w * 0.5) * self.alto, w))
    }

    // ¿La esfera toca el frustum? (planos izq/der/arriba/abajo + plano cercano)
    pub fn esfera_visible(&self, c: V3, radio: f32) -> bool {
        let (f, r, u) = self.base();
        let d = c - self.pos;
        let (w, x, y) = (d.dot(f), d.dot(r), d.dot(u));
        if w < -radio { return false; }
        let (fx, fy) = self.factores();
        let (tx, ty) = (1.0 / fx, 1.0 / fy);
        x.abs() <= tx * w + radio * (1.0 + tx * tx).sqrt() && y.abs() <= ty * w + radio * (1.0 + ty * ty).sqrt()
    }

    // `esferas`: (centro, radio) de los cuerpos sólidos, para colisión y velocidad.
    pub fn actualizar(&mut self, e: &Entrada, dt: f32, esferas: &[(V3, f32)], radio_limite: f32, alto_limite: f32) {
        self.actualizar_modo(e, dt, esferas, radio_limite, alto_limite, None);
    }

    // Mismo control, suavizado y resolución de colisiones que en vuelo libre.
    // La superficie fija la altura y permite una velocidad propia de la misión.
    pub fn actualizar_superficie(&mut self, e: &Entrada, dt: f32, esferas: &[(V3, f32)], velocidad: f32) {
        self.actualizar_modo(e, dt, esferas, 180.0, 100.0, Some(velocidad));
    }

    fn actualizar_modo(&mut self, e: &Entrada, dt: f32, esferas: &[(V3, f32)], radio_limite: f32, alto_limite: f32, superficie: Option<f32>) {
        self.yaw += e.giro_x * SENSIBILIDAD;
        self.pitch = (self.pitch - e.giro_y * SENSIBILIDAD).clamp(-1.5, 1.5);
        self.yaw %= std::f32::consts::TAU;

        // La velocidad escala con la altura sobre el cuerpo más cercano:
        // rápido en el espacio abierto, lento y preciso cerca de una superficie.
        let mut alt = f32::MAX;
        for (c, r) in esferas { alt = alt.min((self.pos - *c).len() - r); }
        let base = superficie.unwrap_or_else(|| (alt * 0.8).clamp(1.5, 45.0) * if e.turbo { 3.5 } else { 1.0 });

        let (f, r, _) = self.base();
        let u = v3(0.0, 1.0, 0.0);
        let f = if superficie.is_some() { v3(f.x, 0.0, f.z).norm() } else { f };
        let deseo = f * e.adelante + r * e.lateral + u * if superficie.is_some() { 0.0 } else { e.vertical };
        let objetivo = if deseo.len() > 1.0 { deseo.norm() * base } else { deseo * base };
        // Aceleración/frenado progresivos con suavizado exponencial (independiente de los FPS).
        let t = 1.0 - (-5.0 * dt).exp();
        self.vel = self.vel + (objetivo - self.vel) * t;
        self.pos = self.pos + self.vel * dt;
        self.velocidad_actual = self.vel.len();

        // Colisión: no se puede atravesar ningún cuerpo.
        for (c, rad) in esferas {
            let d = self.pos - *c;
            let l = d.len().max(1e-4);
            let minimo = rad + 0.5 + rad * 0.04;
            if l < minimo {
                let n = d * (1.0 / l);
                self.pos = *c + n * minimo;
                self.vel = self.vel - n * self.vel.dot(n).min(0.0);
            }
        }
        // Límites del sistema solar.
        let (x, z) = (self.pos.x, self.pos.z);
        let rr = (x * x + z * z).sqrt();
        let max = radio_limite - 2.0;
        if rr > max {
            let k = max / rr;
            self.pos.x *= k;
            self.pos.z *= k;
        }
        self.pos.y = self.pos.y.clamp(-(alto_limite - 2.0), alto_limite - 2.0);
        if superficie.is_some() { self.pos.y = 1.7; self.vel.y = 0.0; }
    }

    // Distancia al muro del límite (positiva dentro).
    pub fn distancia_limite(&self, radio_limite: f32) -> f32 {
        radio_limite - (self.pos.x * self.pos.x + self.pos.z * self.pos.z).sqrt()
    }
}

