// El sistema solar: qué existe, dónde está y cómo se anima.
// Solo los planetas ROTAN sobre su eje 
use crate::camara::Camara;
use crate::dibujo::*;
use crate::estacion::Estacion;
use crate::generadores as gen;
use crate::malla::{self, Malla};
use crate::math::*;
use crate::ruido::*;
use crate::shader::*;
use crate::progreso::Galaxia;

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
    pub(crate) inclinacion: f32,
    pub(crate) vel_rot: f32,
    pub(crate) angulo: f32,
    pub(crate) shader: usize,
    pub(crate) es_sol: bool,
    pub(crate) nubes: Option<usize>, // índice de shader de nubes (Tierra)
    pub(crate) angulo_nubes: f32,
    pub(crate) orbita: Option<(usize, V3)>, // malla del anillo y su color
    pub(crate) anillos: Vec<(usize, usize)>, // malla local y shader; independiente de la guía orbital
}

pub(crate) struct Asteroide {
    pub(crate) pos: V3,
    pub(crate) rot: Rot,
    pub(crate) escala: f32,
    pub(crate) tipo: usize,
    pub(crate) tinte: V3,
}

pub struct Escena {
    // Mallas: [0..3] esferas LOD0..2, luego asteroides (TIPOS x 3 LODs), luego decorado.
    pub(crate) mallas: Vec<Malla>,
    pub(crate) shaders: Vec<Box<dyn Shader>>,
    pub cuerpos: Vec<Cuerpo>,
    pub estacion: Option<Estacion>,
    pub nave_cafe: Option<Estacion>,
    pub galaxia: Galaxia,
    pub radio_limite: f32,
    pub alto_limite: f32,
    pub cinturon: Option<(f32, f32)>,
    pub agujero_negro: Option<crate::agujero_negro::AgujeroNegro>,
    pub(crate) asteroides: Vec<Asteroide>,
    pub(crate) cols_ast: Vec<(V3, f32)>,
    pub(crate) m_cuadrado: usize,
    pub(crate) m_octaedro: usize,
    pub(crate) m_muro: usize,
    pub(crate) m_anillo_limite: usize,
    pub(crate) m_borde_cinturon: Vec<usize>,
    pub(crate) s_asteroide: usize,
    pub(crate) s_orbita: usize,
    pub(crate) s_barrera: usize,
    pub(crate) s_baliza: usize,
    pub(crate) s_brillo: usize,
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
        let m_borde_cinturon = vec![mallas.len(), mallas.len() + 1];
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
        let s_anillos = agregar(Box::new(ShaderAnillos { interior: 1.3, exterior: 2.25 }));
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
                vec![(i, s_anillos)]
            } else { vec![] };
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

        let cols_ast = asteroides.iter().map(|a| {
            let primero = 3 + a.tipo * 3;
            let radio = mallas[primero..primero + 3].iter().map(|m| m.radio).fold(0.0, f32::max);
            (a.pos, a.escala * radio)
        }).collect();
        let tierra = cuerpos.iter().find(|c| c.nombre == "Tierra").expect("falta Tierra");
        let estacion = Some(Estacion::nueva(tierra.pos, cuerpos[0].pos));
        Escena {
            galaxia: Galaxia::Solar, radio_limite: RADIO_LIMITE, alto_limite: ALTO_LIMITE,
            cinturon: Some((CINTURON_INT, CINTURON_EXT)), agujero_negro: None,
            mallas, shaders, cuerpos, estacion, nave_cafe: None, asteroides, cols_ast, m_cuadrado, m_octaedro, m_muro, m_anillo_limite,
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

    pub fn colisionadores_asteroides(&self) -> &[(V3, f32)] { &self.cols_ast }

    pub fn esferas(&self) -> Vec<(V3, f32)> {
        self.cuerpos.iter().map(|c| (c.pos, c.radio))
            .chain(self.nave_cafe.iter().map(|n| (n.pos,n.radio))).collect()
    }

    pub fn cuerpo_cercano(&self, p: V3) -> (&'static str, f32) {
        self.cuerpos.iter().map(|c| (c.nombre, (p - c.pos).len() - c.radio))
            .chain(self.nave_cafe.iter().map(|n| ("Blip-A",(p-n.pos).len()-n.radio)))
            .fold(self.estacion.as_ref().map(|e| ("Estacion", (p - e.pos).len() - e.radio))
                .unwrap_or(("", f32::MAX)), |a, b| if b.1 < a.1 { b } else { a })
    }

    // Vistas de demostración: (nombre, posición, objetivo)
    pub fn vistas(&self) -> Vec<(String, V3, V3)> {
        let mut v = vec![("general".to_string(), v3(0.0, self.alto_limite - 5.0, self.radio_limite * 0.9), V3::ZERO)];
        for c in &self.cuerpos {
            let mut cam = Camara::nueva(100.0, 100.0);
            let encuadre = if self.galaxia != Galaxia::Solar {
                c.anillos.iter().map(|(m, _)| self.mallas[*m].radio * 0.6).fold(1.0, f32::max)
            } else { 1.0 };
            cam.vista_esfera(c.pos, c.radio * encuadre, self.pos_sol());
            if self.agujero_negro.is_some() && c.nombre == "Gargantua" {
                cam.pos = v3(0.0, 23.0, 110.0);
                cam.mirar_a(c.pos);
            }
            if self.galaxia == Galaxia::Eridani && c.nombre == "Erid" {
                cam.pos.y += c.radio * 2.0;
                cam.mirar_a(c.pos);
            }
            let (f, _, _) = cam.base();
            v.push((c.nombre.to_lowercase(), cam.pos, cam.pos + f));
        }
        if let Some((interior, exterior)) = self.cinturon {
            let radio = (interior+exterior)*0.5;
            v.push(("cinturon".to_string(), v3(radio-25.0, 6.0, 40.0), v3(radio-2.0, 0.0, -30.0)));
        }
        v.push(("limite".to_string(), v3(self.radio_limite - 20.0, 4.0, 0.0), v3(self.radio_limite, 0.0, 0.0)));
        if let Some(estacion) = &self.estacion {
            let mut cam = Camara::nueva(100.0, 100.0);
            cam.vista_esfera(estacion.pos, estacion.radio, self.pos_sol());
            // Más altura para ver el anillo, los brazos y los paneles solares.
            cam.pos.y += estacion.radio * 1.6;
            v.push(("estacion".to_string(), cam.pos, estacion.pos));
        }
        if let Some(nave) = &self.nave_cafe {
            let mut cam = Camara::nueva(100.0,100.0);
            cam.vista_esfera(nave.pos,nave.radio,self.pos_sol());
            cam.pos.y += nave.radio*1.2;
            v.push(("nave_cafe".to_string(),cam.pos,nave.pos));
        }
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
            for &(m, _) in &c.anillos {
                st.objetos_total += 1;
                st.tris_sin_optimizar += self.mallas[m].idx.len();
            }
            let radio_visible = c.radio * c.anillos.iter().map(|(m, _)| self.mallas[*m].radio).fold(1.1, f32::max);
            if aj.culling && !cam.esfera_visible(c.pos, radio_visible) { continue; }
            let d = (c.pos - cam.pos).len().max(0.01);
            let px = c.radio / d * k;
            let lod = if !aj.lod || px > 110.0 { 0 } else if px > 22.0 { 1 } else { 2 };
            let mip = if c.es_sol { 0.0 } else { (1024.0 / (4.0 * px.max(1.0))).log2().max(0.0) };
            let rot = Rot::rot_z(c.inclinacion).mul(&Rot::rot_y(c.angulo));
            puesto(Item { malla: &self.mallas[lod], shader: self.shaders[c.shader].as_ref(), pos: c.pos, rot,
                escala: unif(c.radio), mezcla: Mezcla::Opaco, doble_cara: false, tinte: v3(1.0, 1.0, 1.0), mip, dist: d }, st, out);
            for &(m, s) in &c.anillos {
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

        if let Some(agujero) = &self.agujero_negro { agujero.emitir(cam, aj, out, st); }

        // Estación OBJ: cada material conserva su malla y shader. La esfera
        // envolvente descarta el conjunto antes de transformar sus vértices.
        for e in self.estacion.iter().chain(self.nave_cafe.iter()) {
            let visible = !aj.culling || cam.esfera_visible(e.pos, e.radio);
            let distancia = (e.pos - cam.pos).len();
            for (malla, shader) in &e.partes {
                st.objetos_total += 1;
                st.tris_sin_optimizar += malla.idx.len();
                if !visible { continue; }
                puesto(Item { malla, shader, pos: e.pos, rot: e.rot, escala: unif(1.0),
                    mezcla: Mezcla::Opaco, doble_cara: false, tinte: v3(1.0, 1.0, 1.0),
                    mip: 0.0, dist: distancia }, st, out);
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
                let pos = v3(self.radio_limite * a.cos(), 0.0, self.radio_limite * a.sin());
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
