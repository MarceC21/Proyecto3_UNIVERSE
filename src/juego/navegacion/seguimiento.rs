use crate::camara::Camara;
use crate::math::*;

// ---------------------------------------------------------------------------
// Cámara de persecución: va detrás de la nave. La POSICIÓN es rígida (sin
// retraso al avanzar), pero la ORIENTACIÓN se suaviza con el mismo filtro
// exponencial `1 - exp(-k*dt)` de CamaraPersona: en los giros la cámara
// "se balancea" y se siente la inercia. Con el botón derecho se orbita.
// ---------------------------------------------------------------------------
pub struct Seguimiento {
    sf: V3,
    su: V3,
    orb_yaw: f32,
    orb_pitch: f32,
    fov_extra: f32,
}

impl Seguimiento {
    pub fn nuevo(fwd: V3, up: V3) -> Seguimiento {
        Seguimiento { sf: fwd, su: up, orb_yaw: 0.0, orb_pitch: 0.0, fov_extra: 0.0 }
    }

    pub fn reiniciar(&mut self, fwd: V3, up: V3) {
        self.sf = fwd;
        self.su = up;
        self.orb_yaw = 0.0;
        self.orb_pitch = 0.0;
        self.fov_extra = 0.0;
    }

    pub fn actualizar(&mut self, cam: &mut Camara, pos: V3, fwd: V3, up: V3, velocidad: f32, dt: f32,
                      orbitar: Option<(f32, f32)>, esferas: &[(V3, f32)]) {
        let t = 1.0 - (-8.0 * dt).exp();
        self.sf = (self.sf + (fwd - self.sf) * t).norm();
        let su = self.su + (up - self.su) * t;
        self.su = (su - self.sf * su.dot(self.sf)).norm();
        let sr = self.sf.cross(self.su).norm();

        match orbitar {
            Some((dx, dy)) => {
                self.orb_yaw += dx * 0.004;
                self.orb_pitch = (self.orb_pitch - dy * 0.004).clamp(-1.2, 1.2);
            }
            None => {
                let k = (-3.0 * dt).exp();
                self.orb_yaw *= k;
                self.orb_pitch *= k;
            }
        }
        let d0 = -self.sf * 0.96 + self.su * 0.28;
        let d = girar(girar(d0, self.su, self.orb_yaw), sr, self.orb_pitch).norm();
        let dist = 6.5 + velocidad * 0.04;
        cam.pos = pos + d * dist;
        // La cámara tampoco atraviesa cuerpos sólidos.
        for (c, r) in esferas {
            let v = cam.pos - *c;
            let l = v.len().max(1e-4);
            if l < r + 0.6 { cam.pos = *c + v * ((r + 0.6) / l); }
        }
        let f = (pos + self.sf * 4.0 - cam.pos).norm();
        let r = f.cross(self.su).norm();
        cam.base_ext = Some((f, r, r.cross(f)));
        // El campo de visión se abre un poco con la velocidad.
        let objetivo = (velocidad / 45.0).min(1.0) * 12f32.to_radians();
        self.fov_extra += (objetivo - self.fov_extra) * (1.0 - (-3.0 * dt).exp());
        cam.fov = 62f32.to_radians() + self.fov_extra;
    }
}
