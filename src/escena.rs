// El sistema solar: qué existe, dónde está y cómo se anima.
// Solo los planetas ROTAN sobre su eje (no hay órbitas); todo lo demás es estático.
use crate::camara::Camara;
use crate::dibujo::*;
use crate::generadores as gen;
use crate::malla::{self, Malla};
use crate::math::*;
use crate::ruido::*;
use crate::shader::*;

// ---- Dimensiones del mundo (unidades arbitrarias) ----
pub const RADIO_LIMITE: f32 = 350.0; // muro después de Neptuno; cinturón interior intacto
pub const ALTO_LIMITE: f32 = 45.0;
pub const CINTURON_INT: f32 = 104.0;
pub const CINTURON_EXT: f32 = 136.0;
pub const SEMILLA_CINTURON: u64 = 4242; // misma semilla => mismo cinturón
const NUM_ASTEROIDES: usize = 900;
// Multiplicador global de la velocidad de rotación de los planetas.
const VEL_ROT: f32 = 0.5;

pub struct Cuerpo {
    pub nombre: &'static str,
    pub pos: V3,
    pub radio: f32,
    inclinacion: f32,
    vel_rot: f32,
    angulo: f32,
    shader: usize,
    es_sol: bool,
    nubes: Option<usize>, // índice de shader de nubes (Tierra)
    angulo_nubes: f32,
    orbita: Option<(usize, V3)>, // malla del anillo y su color
    anillos: Option<(usize, usize)>, // malla local y shader; independiente de la guía orbital
}

struct Asteroide {
    pos: V3,
    rot: Rot,
    escala: f32,
    tipo: usize,
    tinte: V3,
}

pub struct Escena {
    // Mallas: [0..3] esferas LOD0..2, luego asteroides (TIPOS x 3 LODs), luego decorado.
    mallas: Vec<Malla>,
    shaders: Vec<Box<dyn Shader>>,
    pub cuerpos: Vec<Cuerpo>,
    asteroides: Vec<Asteroide>,
    m_cuadrado: usize,
    m_octaedro: usize,
    m_muro: usize,
    m_anillo_limite: usize,
    m_borde_cinturon: [usize; 2],
    s_asteroide: usize,
    s_orbita: usize,
    s_barrera: usize,
    s_baliza: usize,
    s_brillo: usize,
}

const TIPOS_AST: usize = 4;
const BALIZAS: usize = 48;

impl Escena {
    pub fn nueva() -> Escena {
        let mut mallas = vec![malla::esfera(96, 64), malla::esfera(48, 32), malla::esfera(20, 12)];
        for t in 0..TIPOS_AST {
            let s = 100 + t as u32 * 17;
            mallas.push(malla::asteroide(s, 14, 9)); // LOD0
            mallas.push(malla::asteroide(s, 8, 6)); // LOD1
            mallas.push(malla::asteroide(s, 5, 4)); // LOD2
        }
        let m_cuadrado = mallas.len();
        mallas.push(malla::cuadrado());
        let m_octaedro = mallas.len();
        mallas.push(malla::octaedro());
        let m_muro = mallas.len();
        mallas.push(malla::muro(RADIO_LIMITE, -ALTO_LIMITE, ALTO_LIMITE, 192));
        let m_anillo_limite = mallas.len();
        mallas.push(malla::anillo(RADIO_LIMITE - 1.0, RADIO_LIMITE + 1.0, 384, 150.0));
        let m_borde_cinturon = [mallas.len(), mallas.len() + 1];
        for r in [CINTURON_INT, CINTURON_EXT] {
            mallas.push(malla::anillo(r - 0.35, r + 0.35, 320, (r * std::f32::consts::TAU / 2.6).round()));
        }

        // ---- Shaders (los de planetas llevan su textura horneada) ----
        let mut shaders: Vec<Box<dyn Shader>> = Vec::new();
        let mut agregar = |s: Box<dyn Shader>| { shaders.push(s); shaders.len() - 1 };
        let s_sol = agregar(Box::new(ShaderSol { tex: gen::sol() }));
        let s_mercurio = agregar(Box::new(ShaderPlaneta { tex: gen::mercurio(), atmosfera: rgb(190.0, 170.0, 160.0), fuerza_atm: 0.25 }));
        let s_venus = agregar(Box::new(ShaderPlaneta { tex: gen::venus(), atmosfera: rgb(255.0, 200.0, 110.0), fuerza_atm: 0.9 }));
        let s_tierra = agregar(Box::new(ShaderPlaneta { tex: gen::tierra(), atmosfera: rgb(90.0, 170.0, 255.0), fuerza_atm: 1.1 }));
        let s_nubes = agregar(Box::new(ShaderNubes { tex: gen::nubes() }));
        let s_marte = agregar(Box::new(ShaderPlaneta { tex: gen::marte(), atmosfera: rgb(255.0, 140.0, 90.0), fuerza_atm: 0.5 }));
        let s_jupiter = agregar(Box::new(ShaderPlaneta { tex: gen::jupiter(), atmosfera: rgb(245.0, 206.0, 154.0), fuerza_atm: 0.35 }));
        let s_saturno = agregar(Box::new(ShaderPlaneta { tex: gen::saturno(), atmosfera: rgb(248.0, 225.0, 165.0), fuerza_atm: 0.4 }));
        let s_urano = agregar(Box::new(ShaderPlaneta { tex: gen::urano(), atmosfera: rgb(143.0, 235.0, 240.0), fuerza_atm: 0.65 }));
        let s_neptuno = agregar(Box::new(ShaderPlaneta { tex: gen::neptuno(), atmosfera: rgb(72.0, 141.0, 255.0), fuerza_atm: 0.65 }));
        let s_anillos = agregar(Box::new(ShaderAnillos));
        let s_asteroide = agregar(Box::new(ShaderAsteroide { tex: gen::asteroide() }));
        let s_orbita = agregar(Box::new(ShaderOrbita));
        let s_barrera = agregar(Box::new(ShaderBarrera));
        let s_baliza = agregar(Box::new(ShaderBaliza));
        let s_brillo = agregar(Box::new(ShaderBrillo));

        // ---- Cuerpos: (nombre, distancia al Sol, radio, ángulo en el plano, rotación, inclinación) ----
        // Escala artística para recorrer el sistema; distancias y radios no astronómicos.
        let datos: [(&str, f32, f32, f32, f32, f32, usize, V3); 9] = [
            ("Sol", 0.0, 10.0, 0.0, 0.10, 0.0, s_sol, V3::ZERO),
            ("Mercurio", 26.0, 1.7, 200.0, 0.6, 0.03, s_mercurio, rgb(190.0, 175.0, 160.0)),
            ("Venus", 44.0, 3.8, 300.0, -0.25, 0.05, s_venus, rgb(255.0, 200.0, 110.0)),
            ("Tierra", 64.0, 4.2, 245.0, 1.4, 0.41, s_tierra, rgb(110.0, 190.0, 255.0)),
            ("Marte", 86.0, 2.3, 330.0, 1.3, 0.44, s_marte, rgb(255.0, 130.0, 90.0)),
            ("Jupiter", 160.0, 9.0, 215.0, 2.0, 0.05, s_jupiter, rgb(235.0, 191.0, 145.0)),
            ("Saturno", 215.0, 7.8, 310.0, 1.8, 0.47, s_saturno, rgb(243.0, 218.0, 155.0)),
            ("Urano", 270.0, 5.6, 150.0, -1.2, 1.71, s_urano, rgb(142.0, 224.0, 229.0)),
            ("Neptuno", 315.0, 5.4, 35.0, 1.25, 0.49, s_neptuno, rgb(70.0, 132.0, 242.0)),
        ];
        let mut cuerpos = Vec::new();
        for (nombre, dist, radio, ang, rot, incl, shader, col) in datos {
            let a = ang.to_radians();
            let es_sol = dist == 0.0;
            let orbita = if es_sol { None } else {
                let i = mallas.len();
                mallas.push(malla::anillo(dist - 0.3, dist + 0.3, 320, (dist * std::f32::consts::TAU / 2.6).round()));
                Some((i, col))
            };
            let anillos = if nombre == "Saturno" {
                let i = mallas.len();
                mallas.push(malla::anillo(1.3, 2.25, 256, 1.0));
                Some((i, s_anillos))
            } else { None };
            cuerpos.push(Cuerpo {
                nombre, pos: v3(dist * a.cos(), 0.0, dist * a.sin()), radio, inclinacion: incl,
                vel_rot: rot * VEL_ROT, angulo: 0.0, shader, es_sol,
                nubes: if nombre == "Tierra" { Some(s_nubes) } else { None },
                angulo_nubes: 0.0, orbita, anillos,
            });
        }

        // ---- Cinturón de asteroides: distribución procedural reproducible ----
        // Se proponen posiciones al azar (semilla fija) y se aceptan con una
        // probabilidad dada por ruido 3D: así salen cúmulos y huecos naturales.
        let mut rng = Rng::new(SEMILLA_CINTURON);
        let mut asteroides = Vec::new();
        let paleta = [rgb(255.0, 255.0, 255.0), rgb(255.0, 215.0, 175.0), rgb(190.0, 210.0, 255.0), rgb(235.0, 190.0, 160.0)];
        while asteroides.len() < NUM_ASTEROIDES {
            let ang = rng.rango(0.0, std::f32::consts::TAU);
            // Radio con forma de campana: más denso en el centro de la banda.
            let r = CINTURON_INT + (CINTURON_EXT - CINTURON_INT) * (rng.f() + rng.f()) * 0.5;
            let y = (rng.f() + rng.f() + rng.f() - 1.5) * 6.0;
            let pos = v3(r * ang.cos(), y, r * ang.sin());
            let densidad = clamp01(fbm(pos * 0.045, 3, 7) * 0.9 + 0.55);
            if rng.f() > densidad * densidad { continue; }
            let u = rng.f();
            let escala = 0.45 + 4.2 * u * u * u * u; // pocos grandes, muchos pequeños
            let rot = Rot::rot_y(rng.rango(0.0, 6.28)).mul(&Rot::rot_x(rng.rango(0.0, 6.28))).mul(&Rot::rot_z(rng.rango(0.0, 6.28)));
            asteroides.push(Asteroide {
                pos, rot, escala, tipo: (rng.siguiente() % TIPOS_AST as u64) as usize,
                tinte: paleta[(rng.siguiente() % 4) as usize] * rng.rango(0.8, 1.05),
            });
        }

        Escena {
            mallas, shaders, cuerpos, asteroides, m_cuadrado, m_octaedro, m_muro, m_anillo_limite,
            m_borde_cinturon, s_asteroide, s_orbita, s_barrera, s_baliza, s_brillo,
        }
    }

    pub fn pos_sol(&self) -> V3 { self.cuerpos[0].pos }

    // Solo rotación sobre el eje (dt = tiempo real transcurrido).
    pub fn actualizar(&mut self, dt: f32) {
        for c in self.cuerpos.iter_mut() {
            c.angulo = (c.angulo + c.vel_rot * dt) % std::f32::consts::TAU;
            c.angulo_nubes = (c.angulo_nubes + c.vel_rot * 1.25 * dt) % std::f32::consts::TAU;
        }
    }

    pub fn esferas(&self) -> Vec<(V3, f32)> { self.cuerpos.iter().map(|c| (c.pos, c.radio)).collect() }

    pub fn cuerpo_cercano(&self, p: V3) -> (&'static str, f32) {
        self.cuerpos.iter().map(|c| (c.nombre, (p - c.pos).len() - c.radio))
            .fold(("", f32::MAX), |a, b| if b.1 < a.1 { b } else { a })
    }

    // Vistas de demostración: (nombre, posición, objetivo)
    pub fn vistas(&self) -> Vec<(String, V3, V3)> {
        let mut v = vec![("general".to_string(), v3(0.0, ALTO_LIMITE - 5.0, RADIO_LIMITE * 0.9), V3::ZERO)];
        for c in &self.cuerpos {
            let mut cam = Camara::nueva(100.0, 100.0);
            cam.vista_esfera(c.pos, c.radio, self.pos_sol());
            let (f, _, _) = cam.base();
            v.push((c.nombre.to_lowercase(), cam.pos, cam.pos + f));
        }
        v.push(("cinturon".to_string(), v3(95.0, 6.0, 40.0), v3(118.0, 0.0, -30.0)));
        v.push(("limite".to_string(), v3(RADIO_LIMITE - 20.0, 4.0, 0.0), v3(RADIO_LIMITE, 0.0, 0.0)));
        v
    }

    // Decide qué se dibuja este cuadro: descarte fuera de vista (culling) y
    // nivel de detalle (LOD) según el tamaño en pantalla.
    pub fn emitir<'a>(&'a self, cam: &Camara, aj: &Ajustes, out: &mut Vec<Item<'a>>, st: &mut Stats) {
        let k = cam.escala_px();
        let (_, cr, cu) = cam.base();
        let unif = |s: f32| v3(s, s, s);
        let mut puesto = |it: Item<'a>, st: &mut Stats, out: &mut Vec<Item<'a>>| {
            st.objetos_enviados += 1;
            st.tris_enviados += it.malla.idx.len();
            out.push(it);
        };

        for c in &self.cuerpos {
            let max = self.mallas[0].idx.len();
            st.objetos_total += 1;
            st.tris_sin_optimizar += max * if c.nubes.is_some() { 2 } else { 1 } + if c.es_sol { 2 } else { 0 };
            if let Some((m, _)) = c.anillos {
                st.objetos_total += 1;
                st.tris_sin_optimizar += self.mallas[m].idx.len();
            }
            let radio_visible = c.radio * if c.anillos.is_some() { 2.25 } else { 1.1 };
            if aj.culling && !cam.esfera_visible(c.pos, radio_visible) { continue; }
            let d = (c.pos - cam.pos).len().max(0.01);
            let px = c.radio / d * k;
            let lod = if !aj.lod || px > 110.0 { 0 } else if px > 22.0 { 1 } else { 2 };
            let mip = if c.es_sol { 0.0 } else { (1024.0 / (4.0 * px.max(1.0))).log2().max(0.0) };
            let rot = Rot::rot_z(c.inclinacion).mul(&Rot::rot_y(c.angulo));
            puesto(Item { malla: &self.mallas[lod], shader: self.shaders[c.shader].as_ref(), pos: c.pos, rot,
                escala: unif(c.radio), mezcla: Mezcla::Opaco, doble_cara: false, tinte: v3(1.0, 1.0, 1.0), mip, dist: d }, st, out);
            if let Some((m, s)) = c.anillos {
                puesto(Item { malla: &self.mallas[m], shader: self.shaders[s].as_ref(), pos: c.pos,
                    rot: Rot::rot_z(c.inclinacion), escala: unif(c.radio), mezcla: Mezcla::Alfa,
                    doble_cara: true, tinte: v3(1.0, 1.0, 1.0), mip: 0.0, dist: d }, st, out);
            }
            if let Some(sn) = c.nubes {
                let rot_n = Rot::rot_z(c.inclinacion).mul(&Rot::rot_y(c.angulo_nubes));
                puesto(Item { malla: &self.mallas[lod], shader: self.shaders[sn].as_ref(), pos: c.pos, rot: rot_n,
                    escala: unif(c.radio * 1.025), mezcla: Mezcla::Alfa, doble_cara: false, tinte: v3(1.0, 1.0, 1.0), mip: mip.min(2.0), dist: d }, st, out);
            }
            if c.es_sol {
                // Resplandor: sprite que siempre mira a la cámara.
                let rot_b = Rot::de_columnas(cr, cu, -cam.base().0);
                puesto(Item { malla: &self.mallas[self.m_cuadrado], shader: self.shaders[self.s_brillo].as_ref(), pos: c.pos,
                    rot: rot_b, escala: unif(c.radio * 3.4), mezcla: Mezcla::Aditivo, doble_cara: true,
                    tinte: v3(1.0, 1.0, 1.0), mip: 0.0, dist: d }, st, out);
            }
            if aj.orbitas {
                if let Some((m, col)) = c.orbita {
                    st.objetos_total += 1;
                    st.tris_sin_optimizar += self.mallas[m].idx.len();
                    puesto(Item { malla: &self.mallas[m], shader: self.shaders[self.s_orbita].as_ref(), pos: V3::ZERO,
                        rot: Rot::identidad(), escala: unif(1.0), mezcla: Mezcla::Alfa, doble_cara: true, tinte: col, mip: 0.0, dist: 1e6 }, st, out);
                }
            }
        }

        // Cinturón de asteroides: culling por esfera envolvente + 3 LODs + descarte por tamaño.
        let tris0 = self.mallas[3].idx.len();
        for a in &self.asteroides {
            st.objetos_total += 1;
            st.tris_sin_optimizar += tris0;
            let radio = a.escala * 1.6;
            if aj.culling && !cam.esfera_visible(a.pos, radio) { continue; }
            let d = (a.pos - cam.pos).len().max(0.01);
            let px = radio / d * k;
            if aj.lod && px < 0.35 { continue; } // invisible a esta distancia
            let lod = if !aj.lod || px > 45.0 { 0 } else if px > 9.0 { 1 } else { 2 };
            puesto(Item { malla: &self.mallas[3 + a.tipo * 3 + lod], shader: self.shaders[self.s_asteroide].as_ref(), pos: a.pos,
                rot: a.rot, escala: unif(a.escala), mezcla: Mezcla::Opaco, doble_cara: false, tinte: a.tinte,
                mip: if px > 20.0 { 0.0 } else if px > 6.0 { 1.0 } else { 2.0 }, dist: d }, st, out);
        }

        if aj.orbitas {
            for (i, m) in self.m_borde_cinturon.iter().enumerate() {
                st.objetos_total += 1;
                st.tris_sin_optimizar += self.mallas[*m].idx.len();
                let _ = i;
                puesto(Item { malla: &self.mallas[*m], shader: self.shaders[self.s_orbita].as_ref(), pos: V3::ZERO,
                    rot: Rot::identidad(), escala: unif(1.0), mezcla: Mezcla::Alfa, doble_cara: true,
                    tinte: rgb(255.0, 190.0, 80.0), mip: 0.0, dist: 1e6 }, st, out);
            }
        }

        // ---- Límite del sistema solar: muro de energía, franja luminosa y balizas ----
        if aj.limite {
            st.objetos_total += 2;
            st.tris_sin_optimizar += self.mallas[self.m_muro].idx.len() + self.mallas[self.m_anillo_limite].idx.len();
            puesto(Item { malla: &self.mallas[self.m_muro], shader: self.shaders[self.s_barrera].as_ref(), pos: V3::ZERO,
                rot: Rot::identidad(), escala: unif(1.0), mezcla: Mezcla::Alfa, doble_cara: true, tinte: V3::ZERO, mip: 0.0, dist: 1e7 }, st, out);
            puesto(Item { malla: &self.mallas[self.m_anillo_limite], shader: self.shaders[self.s_baliza].as_ref(), pos: V3::ZERO,
                rot: Rot::identidad(), escala: unif(1.0), mezcla: Mezcla::Alfa, doble_cara: true, tinte: rgb(255.0, 70.0, 190.0), mip: 0.0, dist: 1e7 }, st, out);
            for i in 0..BALIZAS {
                let a = i as f32 / BALIZAS as f32 * std::f32::consts::TAU;
                let pos = v3(RADIO_LIMITE * a.cos(), 0.0, RADIO_LIMITE * a.sin());
                st.objetos_total += 1;
                st.tris_sin_optimizar += 8;
                if aj.culling && !cam.esfera_visible(pos, 9.0) { continue; }
                let col = if i % 2 == 0 { rgb(60.0, 230.0, 255.0) } else { rgb(255.0, 90.0, 200.0) };
                puesto(Item { malla: &self.mallas[self.m_octaedro], shader: self.shaders[self.s_baliza].as_ref(), pos,
                    rot: Rot::rot_y(-a), escala: v3(1.4, 9.0, 1.4), mezcla: Mezcla::Opaco, doble_cara: false, tinte: col,
                    mip: 0.0, dist: (pos - cam.pos).len() }, st, out);
            }
        }
    }
}
