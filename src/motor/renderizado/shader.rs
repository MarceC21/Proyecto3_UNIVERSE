// Fragment shaders escritos a mano. El rasterizador llama a `shade` una vez por
// píxel visible y recibe un color RGB (0..1) + alfa.
use crate::math::*;
use crate::textura::Textura;

// Datos globales del cuadro.
pub struct Contexto {
    pub cam: V3,
    pub tiempo: f32,
    pub sol: V3,
}

// Datos interpolados del píxel (con corrección de perspectiva).
pub struct Fragmento {
    pub pos: V3,
    pub normal: V3,
    pub uv: [f32; 2],
    pub tinte: V3,
    pub centro: V3,
    pub mip: f32,
}

pub trait Shader: Sync {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4];
}

// Materiales del OBJ: casco y paneles iluminados; ventanas con reflejo y
// luces cian emisivas. Todo se evalúa en CPU y escribe en el z-buffer común.
pub struct ShaderEstacion {
    color: V3,
    emisivo: bool,
    vidrio: bool,
}

impl ShaderEstacion {
    pub fn nuevo(nombre: &str, color: V3) -> Self {
        Self { color,
            emisivo: nombre == "Luz_cian" || nombre.contains("toberas"),
            vidrio: nombre == "Vidrio" || nombre == "Reflejo",
        }
    }
}

impl Shader for ShaderEstacion {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        if self.emisivo {
            return [self.color.x, self.color.y, self.color.z, 1.0];
        }
        let n = f.normal.norm();
        let l = (c.sol - f.pos).norm();
        let v = (c.cam - f.pos).norm();
        let luz = luz_toon(n.dot(l));
        let mut col = self.color * tono_sombra(luz) * luz;
        if self.vidrio {
            let reflejo = n.dot((l + v).norm()).max(0.0).powi(24);
            let fresnel = (1.0 - n.dot(v).max(0.0)).powi(3);
            col += rgb(155.0, 232.0, 245.0) * (0.20 + 0.35 * fresnel + 0.65 * reflejo);
        }
        [col.x, col.y, col.z, 1.0]
    }
}

// Iluminación "toon": 3 escalones con bordes suaves (sombra / medio / luz).
fn luz_toon(ndl: f32) -> f32 {
    0.34 + 0.36 * smoothstep(0.02, 0.10, ndl) + 0.30 * smoothstep(0.42, 0.50, ndl)
}

// Las sombras se enfrían (azul/violeta) en vez de solo oscurecerse.
fn tono_sombra(luz: f32) -> V3 {
    mezcla(v3(0.62, 0.66, 1.08), v3(1.0, 0.98, 0.93), clamp01((luz - 0.34) / 0.66))
}

// ---------- Planeta rocoso con textura, luz toon y halo de atmósfera ----------
pub struct ShaderPlaneta {
    pub tex: Textura,
    pub atmosfera: V3,
    pub fuerza_atm: f32,
}

impl Shader for ShaderPlaneta {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let n = f.normal.norm();
        let l = (c.sol - f.pos).norm();
        let v = (c.cam - f.pos).norm();
        let ndl = n.dot(l);
        let luz = luz_toon(ndl);
        let alb = self.tex.muestrear(f.uv[0], f.uv[1], f.mip);
        let mut col = alb * tono_sombra(luz) * (luz * 1.12);
        // Halo atmosférico: Fresnel (más fuerte en el borde) solo del lado iluminado.
        let fres = (1.0 - n.dot(v).max(0.0)).powi(3);
        col += self.atmosfera * (fres * smoothstep(-0.25, 0.35, ndl) * self.fuerza_atm);
        [col.x, col.y, col.z, 1.0]
    }
}

// ---------- Anillos de Saturno: bandas radiales y división de Cassini ----------
pub struct ShaderAnillos;

impl Shader for ShaderAnillos {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let r = f.uv[1];
        let division = smoothstep(0.58, 0.60, r) * (1.0 - smoothstep(0.65, 0.67, r));
        let bandas = (r * 160.0).sin() * 0.5 + 0.5;
        let col = mezcla(rgb(139.0, 112.0, 73.0), rgb(238.0, 218.0, 171.0), bandas * 0.45 + 0.4);
        let hacia_sol = c.sol - f.pos;
        let l = hacia_sol.norm();
        // Sombra del planeta sobre el anillo (radio interior = 1.3 radios).
        let radio_planeta = (f.pos - f.centro).len() / (1.3 + 0.95 * r);
        let al_centro = f.centro - f.pos;
        let t = al_centro.dot(l);
        let separacion = (al_centro - l * t).len();
        let sombra = if t > 0.0 && t < hacia_sol.len() {
            1.0 - smoothstep(radio_planeta * 0.96, radio_planeta * 1.04, separacion)
        } else { 0.0 };
        let luz = (0.45 + 0.55 * f.normal.norm().dot(l).abs()) * (1.0 - sombra * 0.7);
        let borde = smoothstep(0.0, 0.025, r) * (1.0 - smoothstep(0.97, 1.0, r));
        let alfa = borde * (1.0 - division * 0.97) * (0.72 + bandas * 0.20);
        let col = col * luz;
        [col.x, col.y, col.z, alfa]
    }
}

// ---------- Sol: emisivo y animado ----------
pub struct ShaderSol {
    pub tex: Textura,
}

impl Shader for ShaderSol {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let t = c.tiempo;
        // Dos capas de gránulos que se deslizan en sentidos distintos.
        let a = self.tex.muestrear(f.uv[0] + t * 0.010, f.uv[1], f.mip).x;
        let b = self.tex.muestrear(1.0 - f.uv[0] + t * 0.006 + 0.3, f.uv[1], f.mip).x;
        let v = a * 0.6 + b * 0.4;
        // Rampa de color con escalones suaves: naranja -> amarillo -> crema.
        let q = ((v * 4.0).floor() + smoothstep(0.35, 0.65, v * 4.0 - (v * 4.0).floor())) / 4.0;
        let col0 = mezcla(rgb(255.0, 96.0, 20.0), rgb(255.0, 190.0, 44.0), smoothstep(0.15, 0.55, q));
        let mut col = mezcla(col0, rgb(255.0, 246.0, 176.0), smoothstep(0.55, 0.95, q));
        let n = f.normal.norm();
        let ndv = n.dot((c.cam - f.pos).norm()).max(0.0);
        col = mezcla(col, rgb(255.0, 120.0, 30.0), (1.0 - ndv).powi(2) * 0.55);
        col = col * (1.0 + 0.05 * (t * 1.7).sin());
        [col.x, col.y, col.z, 1.0]
    }
}

// ---------- Capa de nubes: el canal R de la textura es la opacidad ----------
pub struct ShaderNubes {
    pub tex: Textura,
}

impl Shader for ShaderNubes {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let a = self.tex.muestrear(f.uv[0], f.uv[1], f.mip).x;
        if a < 0.02 { return [0.0; 4]; }
        let n = f.normal.norm();
        let luz = luz_toon(n.dot((c.sol - f.pos).norm()));
        let col = v3(1.0, 1.0, 1.0) * tono_sombra(luz) * (luz * 1.1);
        [col.x, col.y, col.z, a * 0.9]
    }
}

// ---------- Asteroide: textura + toon + contorno oscuro ----------
pub struct ShaderAsteroide {
    pub tex: Textura,
}

impl Shader for ShaderAsteroide {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let n = f.normal.norm();
        let v = (c.cam - f.pos).norm();
        let luz = luz_toon(n.dot((c.sol - f.pos).norm()));
        let alb = self.tex.muestrear(f.uv[0], f.uv[1], f.mip) * f.tinte;
        let mut col = alb * tono_sombra(luz) * (luz * 1.15);
        if n.dot(v) < 0.2 { col = col * 0.6; } // contorno estilo caricatura
        [col.x, col.y, col.z, 1.0]
    }
}

// ---------- Órbita punteada ----------
pub struct ShaderOrbita;

impl Shader for ShaderOrbita {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let trazo = smoothstep(0.0, 0.08, 0.62 - (f.uv[0] - f.uv[0].floor())) ;
        let borde = 1.0 - (f.uv[1] - 0.5).abs() * 2.0;
        let a = trazo * smoothstep(0.0, 0.5, borde) * (0.55 + 0.1 * (c.tiempo * 1.5).sin());
        [f.tinte.x, f.tinte.y, f.tinte.z, a]
    }
}

// ---------- Campo de fuerza del límite del sistema solar ----------
// Retícula de rombos que viaja hacia arriba y se enciende al acercarse.
pub struct ShaderBarrera;

impl Shader for ShaderBarrera {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let fino = |x: f32| {
            let k = x - x.floor();
            1.0 - smoothstep(0.0, 0.07, k.min(1.0 - k))
        };
        let p = f.uv[0] * 120.0;
        let q = f.pos.y / 5.0 - c.tiempo * 0.15;
        let linea = fino(p + q).max(fino(p - q));
        let cerca = 1.0 - smoothstep(8.0, 70.0, (c.cam - f.pos).len());
        let energia = 0.5 + 0.5 * (f.pos.y * 0.22 - c.tiempo * 1.6 + f.uv[0] * 40.0).sin();
        let borde = smoothstep(0.0, 0.18, f.uv[1]) * smoothstep(1.0, 0.82, f.uv[1]);
        let a = (0.035 + linea * (0.16 + 0.55 * cerca) + cerca * energia * 0.09) * borde;
        let col = mezcla(rgb(40.0, 220.0, 255.0), rgb(255.0, 80.0, 200.0), f.uv[1]);
        [col.x, col.y, col.z, a]
    }
}

// ---------- Baliza / franja emisiva con pulso viajero ----------
pub struct ShaderBaliza;

impl Shader for ShaderBaliza {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let ang = f.centro.z.atan2(f.centro.x);
        let pulso = 0.65 + 0.35 * (c.tiempo * 3.0 - ang * 6.0).sin();
        let col = f.tinte * pulso;
        [col.x, col.y, col.z, 0.9]
    }
}

// ---------- Resplandor del Sol (sprite aditivo con rayos) ----------
pub struct ShaderBrillo;

impl Shader for ShaderBrillo {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let (x, y) = (f.uv[0] - 0.5, f.uv[1] - 0.5);
        let r = (x * x + y * y).sqrt() * 2.0;
        if r >= 1.0 { return [0.0; 4]; }
        let ang = y.atan2(x);
        let aureola = (1.0 - r).powi(3);
        let rayos = (0.5 + 0.5 * (ang * 9.0 + c.tiempo * 0.25).cos()).powi(6) * (1.0 - r).powi(2) * 0.5;
        let col = mezcla(rgb(255.0, 150.0, 40.0), rgb(255.0, 225.0, 140.0), aureola);
        [col.x, col.y, col.z, (aureola * 0.85 + rayos) * f.tinte.x]
    }
}

// ---------- Casco de la nave: color del .mtl + luz toon + brillo especular ----------
// `espec` = intensidad del destello; `emisivo` >= 1 => la pieza brilla por sí sola (toberas).
pub struct ShaderNave {
    pub espec: f32,
    pub emisivo: f32,
}

impl Shader for ShaderNave {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        if self.emisivo >= 1.0 {
            let col = f.tinte * (1.0 + 0.12 * (c.tiempo * 6.0).sin());
            return [col.x, col.y, col.z, 1.0];
        }
        let v = (c.cam - f.pos).norm();
        let mut n = f.normal.norm();
        // El modelo se dibuja a doble cara: la normal siempre se orienta hacia el ojo.
        if n.dot(v) < 0.0 { n = -n; }
        let l = (c.sol - f.pos).norm();
        let luz = luz_toon(n.dot(l));
        let mut col = f.tinte * 1.25 * tono_sombra(luz) * (luz * 1.1) + f.tinte * self.emisivo;
        let h = (l + v).norm();
        let s = n.dot(h).max(0.0).powf(48.0);
        col += v3(1.0, 1.0, 1.0) * (smoothstep(0.55, 0.65, s) * self.espec);
        let ndv = n.dot(v).max(0.0);
        col += rgb(120.0, 190.0, 255.0) * ((1.0 - ndv).powi(3) * 0.18); // luz de borde fría
        [col.x, col.y, col.z, 1.0]
    }
}

// ---------- Llama del propulsor (sprite aditivo cian -> blanco) ----------
pub struct ShaderLlama;

impl Shader for ShaderLlama {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let (x, y) = (f.uv[0] - 0.5, f.uv[1] - 0.5);
        let r = (x * x + y * y).sqrt() * 2.0;
        if r >= 1.0 { return [0.0; 4]; }
        let nucleo = (1.0 - r).powf(2.2);
        let col = mezcla(rgb(40.0, 150.0, 255.0), rgb(235.0, 250.0, 255.0), nucleo * nucleo);
        let parpadeo = 0.9 + 0.1 * (c.tiempo * 38.0 + f.centro.x * 7.0).sin();
        [col.x, col.y, col.z, nucleo * f.tinte.x * parpadeo]
    }
}
