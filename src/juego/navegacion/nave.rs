// La nave del jugador: modelo (.obj), vuelo, modo local y colisiones.
//
// Convenciones del modelo nave.obj: +Z es el frente (cabina), -Z los motores,
// +Y arriba y las alas se extienden en X.
//
// Lo reutilizado de proyectos anteriores:
//   * obj.rs: el cargador .obj/.mtl (un material por grupo de caras).
//   * El suavizado exponencial `1 - exp(-k*dt)` de CamaraPersona, para que la
//     aceleración y el frenado sean progresivos y no dependan de los FPS.
//   * Los SUBPASOS de Colisiones::mover: el avance de un cuadro se divide en
//     pasos pequeños para no "saltar" una roca a alta velocidad.
use crate::camara::Camara;
use crate::dibujo::*;
use crate::malla::{self, Malla};
use crate::math::*;
use crate::obj::Modelo;
use crate::shader::*;

pub const ESCALA_NAVE: f32 = 0.05; // el modelo mide ~50 unidades: queda de ~2.5 en el mundo
const PIVOTE_Y: f32 = 3.5;
pub const INICIO: V3 = v3(0.0, 6.0, 90.0); // zona despejada entre Marte y el cinturón
const REBOTE: f32 = 0.3;
const SENSIBILIDAD_RATON: f32 = 0.0022;
const GIRO_MAX: f32 = 1.9; // rad/s

// Esferas de colisión (unidades del modelo, relativas al pivote): cuerpo, nariz, cola y puntas de ala.
const ESFERAS: [(V3, f32); 5] = [
    (v3(0.0, 0.0, 0.0), 17.0),
    (v3(0.0, 0.0, 22.0), 7.0),
    (v3(0.0, 0.0, -23.0), 9.0),
    (v3(24.0, 0.0, 0.0), 6.0),
    (v3(-24.0, 0.0, 0.0), 6.0),
];
const RADIO_ENVOLVENTE: f32 = 32.0 * ESCALA_NAVE;

// Pose visual independiente del estado de vuelo (cinemáticas y vistas previas).
#[derive(Clone, Copy)]
pub struct PoseNave {
    pub pos: V3,
    pub fwd: V3,
    pub up: V3,
    pub empuje: f32,
}

// Lo que la nave necesita saber del mundo.
pub struct Mundo<'a> {
    pub cuerpos: &'a [(V3, f32)],
    pub asteroides: &'a [(V3, f32)],
    pub radio_limite: f32,
    pub alto_limite: f32,
    pub cinturon: Option<(f32, f32)>,
}

// Entrada ya interpretada (la llena main.rs con raylib).
#[derive(Default)]
pub struct Control {
    pub empuje: f32,   // W/S
    pub lateral: f32,  // A/D
    pub vertical: f32, // Espacio/Ctrl
    pub cabeceo: f32,  // flechas arriba/abajo
    pub guinada: f32,  // flechas izq/der
    pub alabeo: f32,   // Q/E
    pub raton: (f32, f32),
    pub turbo: bool,
}

pub struct Nave {
    pub pos: V3,
    reaparicion: V3,
    pub fwd: V3,
    pub up: V3,
    pub vel: V3,
    w: V3, // velocidad angular local: (cabeceo, guiñada, alabeo)
    pub empuje_visual: f32,
    pub modo_local: bool,  // el jugador lo pidió
    pub local_activo: bool, // y además hay un cuerpo lo bastante cerca
    pub objetivo: V3,      // centro del cuerpo más cercano
    pub altura: f32,       // distancia a su superficie
    pub velocidad_max: f32,
    pub integridad: f32,
    pub golpe: f32,     // >0 justo después de un impacto
    pub destruida: f32, // >0 mientras se muestra "nave destruida"
    inmune: f32,
    pub pivote: V3,
    mallas: Vec<Malla>,
    shaders: Vec<ShaderNave>,
    partes: Vec<(usize, usize, V3)>, // (malla, shader, color)
    m_llama: Malla,
    s_llama: ShaderLlama,
}

impl Nave {
    // Solo física (sin modelo): sirve para las pruebas automáticas.
    pub fn fisica() -> Nave {
        Nave {
            pos: INICIO, reaparicion: INICIO, fwd: v3(0.0, -0.05, -1.0).norm(), up: v3(0.0, 1.0, 0.0), vel: V3::ZERO, w: V3::ZERO,
            empuje_visual: 0.0, modo_local: false, local_activo: false, objetivo: V3::ZERO, altura: 100.0,
            velocidad_max: 0.0, integridad: 100.0, golpe: 0.0, destruida: 0.0, inmune: 0.0,
            pivote: v3(0.0, PIVOTE_Y, 0.0), mallas: vec![], shaders: vec![], partes: vec![],
            m_llama: malla::cuadrado(), s_llama: ShaderLlama,
        }
        .ortonormalizada()
    }

    pub fn cargar(ruta: &str) -> Result<Nave, String> {
        Self::desde_modelo(Modelo::cargar(ruta)?)
    }

    pub fn predeterminada() -> Result<Nave, String> {
        Self::desde_modelo(Modelo::desde_textos(
            include_str!("../../../assets/compartidos/nave/nave.obj"),
            include_str!("../../../assets/compartidos/nave/nave.mtl"),
        )?)
    }

    fn desde_modelo(modelo: Modelo) -> Result<Nave, String> {
        let (centro, radio) = modelo.centro_y_radio();
        let mut n = Nave::fisica();
        n.pivote = v3(centro[0], PIVOTE_Y, centro[2]);
        // Una malla por material: cada una con su shader y su color del .mtl.
        let mut grupos: Vec<Vec<[usize; 3]>> = vec![vec![]; modelo.materiales.len()];
        for c in &modelo.caras { grupos[c.material].push(c.v); }
        for (mi, mat) in modelo.materiales.iter().enumerate() {
            let (mut pos, mut nrm, mut uv, mut idx) = (vec![], vec![], vec![], vec![]);
            for t in &grupos[mi] {
                let p: Vec<V3> = t.iter().map(|&i| { let v = modelo.vertices[i]; v3(v[0], v[1], v[2]) }).collect();
                let normal = (p[1] - p[0]).cross(p[2] - p[0]).norm();
                let base = pos.len() as u32;
                for q in &p { pos.push(*q); nrm.push(normal); uv.push([0.0, 0.0]); }
                idx.push([base, base + 1, base + 2]);
            }
            if idx.is_empty() { continue; }
            n.mallas.push(Malla { pos, nrm, uv, idx, radio });
            let sh = match mat.nombre.as_str() {
                "Luz_cian" | "Fondo_de_toberas" => ShaderNave { espec: 0.0, emisivo: 1.0 },
                "Vidrio" => ShaderNave { espec: 1.0, emisivo: 0.15 },
                "Reflejo" => ShaderNave { espec: 1.0, emisivo: 0.6 },
                "Panel_solar_azul" | "Celda_clara" => ShaderNave { espec: 0.6, emisivo: 0.05 },
                "Marco_claro" | "Placa" => ShaderNave { espec: 0.35, emisivo: 0.0 },
                _ => ShaderNave { espec: 0.15, emisivo: 0.0 },
            };
            n.shaders.push(sh);
            let k = mat.color;
            n.partes.push((n.mallas.len() - 1, n.shaders.len() - 1, v3(k[0], k[1], k[2])));
        }
        Ok(n)
    }

    fn ortonormalizada(mut self) -> Nave {
        self.ortonormalizar();
        self
    }

    fn ortonormalizar(&mut self) {
        self.fwd = self.fwd.norm();
        self.up = (self.up - self.fwd * self.up.dot(self.fwd)).norm();
    }

    pub fn derecha(&self) -> V3 { self.fwd.cross(self.up).norm() }

    pub fn velocidad(&self) -> f32 { self.vel.len() }

    // Coloca la nave en un punto mirando a un objetivo (para los atajos 1..7).
    pub fn teletransportar(&mut self, pos: V3, objetivo: V3) {
        self.pos = pos;
        self.vel = V3::ZERO;
        self.w = V3::ZERO;
        self.fwd = (objetivo - pos).norm();
        self.up = v3(0.0, 1.0, 0.0);
        if self.fwd.y.abs() > 0.98 { self.up = v3(0.0, 0.0, -1.0); }
        self.ortonormalizar();
    }

    pub fn establecer_reaparicion(&mut self, pos: V3) { self.reaparicion = pos; }

    pub fn reaparecer(&mut self) {
        self.teletransportar(self.reaparicion, self.reaparicion + v3(0.0, -0.05, -1.0));
        self.integridad = 100.0;
        self.destruida = 0.0;
        self.golpe = 0.0;
        self.inmune = 0.0;
        self.local_activo = false;
        self.empuje_visual = 0.0;
    }

    // Velocidad máxima según el entorno: rápida en el espacio abierto, lenta cerca
    // de una superficie, y moderada dentro del cinturón de asteroides.
    fn limite_velocidad(&self, altura: f32, m: &Mundo, turbo: bool, local: bool) -> f32 {
        let mut v = (altura * 0.7).clamp(2.5, 40.0);
        let r = (self.pos.x * self.pos.x + self.pos.z * self.pos.z).sqrt();
        let en_cinturon = m.cinturon.is_some_and(|(a, b)| r > a - 6.0 && r < b + 6.0 && self.pos.y.abs() < 14.0);
        if en_cinturon { v = v.min(12.0); }
        if local { v = v.min(7.0); }
        if turbo && !local { v *= if en_cinturon { 1.6 } else { 3.0 }; }
        v
    }

    pub fn actualizar(&mut self, c: &Control, dt: f32, m: &Mundo) {
        if !dt.is_finite() || dt <= 0.0 { return; }
        if self.destruida > 0.0 {
            self.destruida -= dt;
            if self.destruida <= 0.0 { self.reaparecer(); }
            return;
        }
        self.golpe = (self.golpe - dt).max(0.0);
        self.inmune = (self.inmune - dt).max(0.0);

        // Cuerpo más cercano.
        let (mut alt, mut centro, mut rad) = (f32::MAX, V3::ZERO, 1.0);
        for (cc, r) in m.cuerpos {
            let a = (self.pos - *cc).len() - r;
            if a < alt { alt = a; centro = *cc; rad = *r; }
        }
        self.altura = alt;
        self.objetivo = centro;
        self.local_activo = self.modo_local && alt < rad * 3.0 + 4.0;

        // ---------------- Orientación ----------------
        if self.local_activo {
            // MODO LOCAL: la nariz se mantiene apuntando al cuerpo y el "arriba" se
            // alinea con el del mundo. Con A/D la nave se desplaza de lado y por
            // tanto RODEA el planeta sin tener que corregir la orientación.
            let t = 1.0 - (-3.0 * dt).exp();
            let objetivo = (centro - self.pos).norm();
            self.fwd = (self.fwd + (objetivo - self.fwd) * t).norm();
            let up_obj = v3(0.0, 1.0, 0.0) - self.fwd * self.fwd.y;
            if up_obj.len() > 0.2 { self.up = (self.up + (up_obj.norm() - self.up) * t).norm(); }
            self.ortonormalizar();
            self.w = V3::ZERO;
        } else {
            let inv = 1.0 / dt.max(1e-4);
            let objetivo = v3(
                (c.cabeceo * 1.3 - c.raton.1 * SENSIBILIDAD_RATON * inv).clamp(-GIRO_MAX, GIRO_MAX),
                (-c.guinada * 1.3 - c.raton.0 * SENSIBILIDAD_RATON * inv).clamp(-GIRO_MAX, GIRO_MAX),
                c.alabeo * 1.8,
            );
            // Giro progresivo: la velocidad angular persigue a la deseada (suavizado exponencial).
            self.w = self.w + (objetivo - self.w) * (1.0 - (-9.0 * dt).exp());
            let der = self.derecha();
            self.fwd = girar(self.fwd, der, self.w.x * dt);
            self.up = girar(self.up, der, self.w.x * dt);
            self.fwd = girar(self.fwd, self.up, self.w.y * dt);
            self.up = girar(self.up, self.fwd, self.w.z * dt);
            self.ortonormalizar();
        }

        // ---------------- Traslación ----------------
        let vmax = self.limite_velocidad(alt, m, c.turbo, self.local_activo);
        self.velocidad_max = vmax;
        let der = self.derecha();
        let adelante = if c.empuje > 0.0 { vmax } else { vmax * 0.5 };
        let deseo = self.fwd * (c.empuje * adelante) + der * (c.lateral * vmax * 0.6) + self.up * (c.vertical * vmax * 0.6);
        // Acelera despacio y frena un poco más rápido; independiente de los FPS.
        let k = if deseo.len() < self.vel.len() { 2.4 } else { 1.4 };
        self.vel = self.vel + (deseo - self.vel) * (1.0 - (-k * dt).exp());
        let llama = if c.empuje > 0.0 { 1.0 + if c.turbo { 0.6 } else { 0.0 } } else { 0.12 };
        self.empuje_visual += (llama - self.empuje_visual) * (1.0 - (-6.0 * dt).exp());

        // Subpasos: el avance se parte en tramos de <= 0.3 unidades antes de colisionar.
        // En modo local, sin W/S la nave conserva su distancia al cuerpo: el
        // desplazamiento lateral/vertical se convierte en una ÓRBITA alrededor de él.
        let mantener = self.local_activo && c.empuje.abs() < 0.05;
        let radio_orbita = (self.pos - self.objetivo).len();
        if mantener {
            let radial = (self.pos - self.objetivo).norm();
            self.vel = self.vel - radial * self.vel.dot(radial);
        }
        let paso = self.vel * dt;
        let n = (paso.len() / 0.3).ceil().max(1.0) as usize;
        let mut impacto = 0.0f32;
        for _ in 0..n {
            self.pos = self.pos + self.vel * (dt / n as f32);
            if mantener {
                self.pos = self.objetivo + (self.pos - self.objetivo).norm() * radio_orbita;
            }
            impacto = impacto.max(self.resolver_colisiones(m));
        }
        if impacto > 2.5 { self.golpear(impacto); }
    }

    // Empuja la nave fuera de lo que toque y rebota. Devuelve la velocidad del impacto.
    fn resolver_colisiones(&mut self, m: &Mundo) -> f32 {
        let mut impacto = 0.0f32;
        let der = self.derecha();
        for (cc, rr) in m.cuerpos.iter().chain(m.asteroides.iter()) {
            // Rechazo rápido con una esfera que envuelve toda la nave.
            let d = self.pos - *cc;
            let lim = RADIO_ENVOLVENTE + rr;
            if d.dot(d) > lim * lim { continue; }
            for (o, r_modelo) in ESFERAS {
                // Recalcular después de cada corrección: las cinco esferas
                // pertenecen a la misma nave y deben trasladarse juntas.
                let c = self.pos + der * (o.x * ESCALA_NAVE)
                    + self.up * (o.y * ESCALA_NAVE) + self.fwd * (o.z * ESCALA_NAVE);
                let r = r_modelo * ESCALA_NAVE;
                let d = c - *cc;
                let min = r + rr;
                let l2 = d.dot(d);
                if l2 >= min * min { continue; }
                let l = l2.sqrt().max(1e-4);
                let n = if l2 < 1e-8 { self.up } else { d * (1.0 / l) };
                self.pos = self.pos + n * (min - l);
                let vn = self.vel.dot(n);
                if vn < 0.0 {
                    self.vel = self.vel - n * (vn * (1.0 + REBOTE));
                    impacto = impacto.max(-vn);
                }
            }
        }
        // Muro del límite del sistema solar: tope suave, sin daño.
        let r = (self.pos.x * self.pos.x + self.pos.z * self.pos.z).sqrt();
        let max = m.radio_limite - RADIO_ENVOLVENTE;
        if r > max {
            let n = v3(self.pos.x / r, 0.0, self.pos.z / r);
            self.pos = self.pos - n * (r - max);
            let vn = self.vel.dot(n);
            if vn > 0.0 { self.vel = self.vel - n * vn; }
        }
        let ymax = m.alto_limite - RADIO_ENVOLVENTE;
        if self.pos.y.abs() > ymax {
            self.pos.y = self.pos.y.clamp(-ymax, ymax);
            self.vel.y = 0.0;
        }
        impacto
    }

    fn golpear(&mut self, v: f32) {
        if self.inmune > 0.0 { return; }
        self.integridad -= (v - 2.0) * 4.0;
        self.golpe = 0.5;
        self.inmune = 0.4;
        if self.integridad <= 0.0 {
            self.integridad = 0.0;
            self.destruida = 2.5;
        }
    }

    // ---------------- Dibujo ----------------
    pub fn emitir<'a>(&'a self, cam: &Camara, out: &mut Vec<Item<'a>>, st: &mut Stats) {
        if self.destruida > 0.0 { return; }
        self.emitir_pose(cam, PoseNave { pos: self.pos, fwd: self.fwd,
            up: self.up, empuje: self.empuje_visual }, out, st);
    }

    pub fn emitir_pose<'a>(&'a self, cam: &Camara, pose: PoseNave,
        out: &mut Vec<Item<'a>>, st: &mut Stats) {
        let rot = Rot::de_columnas(-pose.fwd.cross(pose.up).norm(), pose.up, pose.fwd);
        let escala = v3(ESCALA_NAVE, ESCALA_NAVE, ESCALA_NAVE);
        let origen = pose.pos - rot.aplicar(self.pivote * ESCALA_NAVE);
        let dist = (pose.pos - cam.pos).len();
        for (mi, si, col) in &self.partes {
            st.objetos_total += 1;
            st.objetos_enviados += 1;
            st.tris_sin_optimizar += self.mallas[*mi].idx.len();
            st.tris_enviados += self.mallas[*mi].idx.len();
            out.push(Item { malla: &self.mallas[*mi], shader: &self.shaders[*si], pos: origen, rot, escala,
                mezcla: Mezcla::Opaco, doble_cara: true, tinte: *col, mip: 0.0, dist });
        }
        // Llamas de los tres motores (en el modelo: x = -5.5, 0, 5.5; y = 4.8; z = -29.2).
        let (f, r, u) = cam.base();
        let rot_b = Rot::de_columnas(r, u, -f);
        let tam = 0.28 + 0.55 * pose.empuje.min(1.6);
        for x in [-5.5f32, 0.0, 5.5] {
            st.objetos_total += 1;
            st.objetos_enviados += 1;
            st.tris_sin_optimizar += self.m_llama.idx.len();
            st.tris_enviados += self.m_llama.idx.len();
            let p = origen + rot.aplicar(v3(x, 4.8, -29.6) * ESCALA_NAVE);
            out.push(Item { malla: &self.m_llama, shader: &self.s_llama, pos: p, rot: rot_b,
                escala: v3(tam, tam, tam), mezcla: Mezcla::Aditivo, doble_cara: true,
                tinte: v3(0.3 + 0.7 * pose.empuje.min(1.0), 0.0, 0.0), mip: 0.0, dist: dist - 0.3 });
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::escena::*;

    fn mundo<'a>(cuerpos: &'a [(V3, f32)], ast: &'a [(V3, f32)]) -> Mundo<'a> {
        Mundo { cuerpos, asteroides: ast, radio_limite: RADIO_LIMITE, alto_limite: ALTO_LIMITE, cinturon: Some((CINTURON_INT, CINTURON_EXT)) }
    }

    #[test]
    fn impacto_en_un_cuadro_largo_no_atraviesa_roca() {
        let ast = [(V3::ZERO, 1.0)];
        let m = Mundo { cuerpos: &[], asteroides: &ast, radio_limite: 1000.0,
            alto_limite: 1000.0, cinturon: Some((104.0, 136.0)) };
        let mut n = Nave::fisica();
        n.teletransportar(v3(0.0, 0.0, -6.0), V3::ZERO);
        n.vel = v3(0.0, 0.0, 400.0);
        n.actualizar(&Control { empuje: 1.0, turbo: true, ..Default::default() }, 0.1, &m);
        // La nave se mantiene del lado de llegada incluso si cruza el diámetro
        // entero de la roca en un solo cuadro. No se permite reaparecer para pasar.
        assert!(n.pos.z < -1.0, "atravesó la roca: {:?}", n.pos);
        assert!(n.vel.z <= 0.0, "no rebotó: {:?}", n.vel);
        assert!(n.integridad < 100.0);
        assert!(n.destruida > 0.0);
    }

    #[test]
    fn recurso_nave_y_seguimiento_se_renderizan_en_el_sistema_actual() {
        use crate::cielo::Cielo;
        use crate::raster::Framebuffer;
        use crate::render::{dibujar, dibujar_con_nave};
        use crate::seguimiento::Seguimiento;
        let e = Escena::nueva();
        assert_eq!(e.cuerpos.len(), 9);
        let mut n = Nave::predeterminada().unwrap();
        assert!(!n.partes.is_empty());
        let vista = e.vistas().into_iter().find(|v| v.0 == "estacion").unwrap();
        n.teletransportar(vista.1, vista.2);
        let mut cam = Camara::nueva(320.0, 200.0);
        let mut seg = Seguimiento::nuevo(n.fwd, n.up);
        seg.actualizar(&mut cam, n.pos, n.fwd, n.up, 0.0, 1.0 / 60.0, None, &e.esferas());
        let cielo = Cielo::nuevo();
        let aj = Ajustes::default();
        let mut sin = Framebuffer::new(320, 200);
        let mut con = Framebuffer::new(320, 200);
        let a = dibujar(&mut sin, &e, &cielo, &cam, 0.0, &aj);
        let b = dibujar_con_nave(&mut con, &e, &cielo, &cam, 0.0, &aj, Some(&n));
        assert!(b.tris_enviados > a.tris_enviados);
        assert!(con.pixels.chunks_exact(4).zip(sin.pixels.chunks_exact(4))
            .filter(|(a, b)| a != b).count() > 100, "la nave no aparece en el framebuffer");
        assert!(con.zbuf.iter().zip(&sin.zbuf).any(|(a, b)| a > b), "la nave no escribe profundidad");
    }

    // Vuela contra un objetivo a toda velocidad y devuelve la mínima distancia
    // (centro de la nave al centro del objetivo) observada.
    fn embestir(escena: &Escena, desde: V3, hacia: V3, turbo: bool) -> f32 {
        let (cu, ast) = (escena.esferas(), escena.colisionadores_asteroides().to_vec());
        let m = mundo(&cu, &ast);
        let mut n = Nave::fisica();
        n.teletransportar(desde, hacia);
        let c = Control { empuje: 1.0, turbo, ..Default::default() };
        let mut minimo = f32::MAX;
        for _ in 0..900 {
            n.actualizar(&c, 1.0 / 60.0, &m);
            minimo = minimo.min((n.pos - hacia).len());
        }
        minimo
    }

    #[test]
    fn no_atraviesa_planetas_ni_el_sol() {
        let e = Escena::nueva();
        for c in &e.cuerpos {
            let dir = (v3(0.3, 0.1, 1.0)).norm();
            let desde = c.pos + dir * (c.radio + 40.0);
            for dt_turbo in [false, true] {
                let d = embestir(&e, desde, c.pos, dt_turbo);
                assert!(d > c.radio + 0.2, "{} atravesado: distancia {}", c.nombre, d);
            }
        }
    }

    #[test]
    fn no_atraviesa_asteroides() {
        let e = Escena::nueva();
        let (mut probados, ast) = (0, e.colisionadores_asteroides().to_vec());
        for (p, r) in ast.iter().step_by(37).take(20) {
            let desde = *p + v3(0.0, 0.0, 30.0 + r);
            let d = embestir(&e, desde, *p, true);
            assert!(d > r * 0.9, "asteroide atravesado: {} < {}", d, r);
            probados += 1;
        }
        assert!(probados > 5);
    }

    #[test]
    fn respeta_el_limite_del_sistema() {
        let e = Escena::nueva();
        let (cu, ast) = (e.esferas(), vec![]);
        let m = mundo(&cu, &ast);
        let mut n = Nave::fisica();
        n.teletransportar(v3(100.0, 0.0, 100.0), v3(500.0, 0.0, 500.0));
        let c = Control { empuje: 1.0, turbo: true, ..Default::default() };
        for _ in 0..1200 {
            n.actualizar(&c, 1.0 / 60.0, &m);
            assert!((n.pos.x * n.pos.x + n.pos.z * n.pos.z).sqrt() <= RADIO_LIMITE);
        }
    }

    #[test]
    fn aceleracion_progresiva_y_consistente_entre_fps() {
        let e = Escena::nueva();
        let (cu, ast) = (e.esferas(), vec![]);
        let m = mundo(&cu, &ast);
        let mut resultados = vec![];
        for fps in [30.0f32, 144.0] {
            let mut n = Nave::fisica();
            n.teletransportar(v3(0.0, 0.0, 160.0), v3(0.0, 0.0, 0.0));
            let c = Control { empuje: 1.0, ..Default::default() };
            let dt = 1.0 / fps;
            let primer = { n.actualizar(&c, dt, &m); n.velocidad() };
            for _ in 0..(fps as usize * 2 - 1) { n.actualizar(&c, dt, &m); }
            assert!(primer < n.velocidad() * 0.2, "no es progresiva");
            resultados.push(n.velocidad());
        }
        assert!((resultados[0] - resultados[1]).abs() < 0.5, "{:?}", resultados);
    }

    #[test]
    fn modo_local_permite_rodear_un_planeta() {
        let e = Escena::nueva();
        let (cu, ast) = (e.esferas(), vec![]);
        let m = mundo(&cu, &ast);
        let tierra = e.cuerpos.iter().find(|c| c.nombre == "Tierra").unwrap();
        let mut n = Nave::fisica();
        n.teletransportar(tierra.pos + v3(0.0, 0.0, tierra.radio + 8.0), tierra.pos);
        n.modo_local = true;
        let c = Control { lateral: 1.0, ..Default::default() };
        let a0 = (n.pos.x - tierra.pos.x).atan2(n.pos.z - tierra.pos.z);
        let mut dist_min = f32::MAX;
        for _ in 0..1800 {
            n.actualizar(&c, 1.0 / 60.0, &m);
            dist_min = dist_min.min((n.pos - tierra.pos).len());
        }
        let a1 = (n.pos.x - tierra.pos.x).atan2(n.pos.z - tierra.pos.z);
        assert!((a1 - a0).abs() > 0.8, "no rodeó: {} -> {}", a0, a1);
        assert!(dist_min > tierra.radius_ok());
        // Y la nariz sigue mirando al planeta.
        assert!(n.fwd.dot((tierra.pos - n.pos).norm()) > 0.9);
    }

    trait RadioOk { fn radius_ok(&self) -> f32; }
    impl RadioOk for Cuerpo { fn radius_ok(&self) -> f32 { self.radio } }
}
