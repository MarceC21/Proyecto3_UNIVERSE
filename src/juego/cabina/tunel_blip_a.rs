//! Geometría del túnel de Blip-A; la aventura aporta movimiento, colisiones y progreso.
//! Reutiliza Geo y el rasterizador del proyecto. Eje del recorrido: -Z; suelo: Y=0.14.
use crate::{
    camara::Camara,
    construccion::Geo,
    dibujo::{Item, Mezcla},
    malla::Malla,
    math::*,
    raster::{preparar_item, rasterizar_banda, Framebuffer, Vista},
    ruido::{fbm, ruido, Rng},
    shader::{Contexto, Fragmento, Shader},
    textura::nucleos,
};
use std::{f32::consts::TAU, sync::Mutex};

pub const LARGO: f32 = 24.0;
pub const CENTRO_Y: f32 = 2.77;
pub const RADIO_CRISTAL: f32 = 2.05;
pub const Z_CRISTAL: f32 = -24.1;
/// Posición inicial de Rocky detrás del cristal.
pub const POSICION_PERSONAJE: V3 = v3(0.0, 0.15, -28.0);

#[derive(Clone, Copy)]
enum Material {
    Lamina,
    Cobre,
    Bronce,
    Nervio,
    Piso,
    Camara,
    Luz,
    Cristal,
}
struct Superficie {
    malla: Malla,
    shader: Metal,
    mezcla: Mezcla,
}
struct Metal {
    material: Material,
}

// Fuentes tenues de cobre/ámbar, incluyendo una luz suave al otro lado del vidrio.
const LUCES: [(V3, V3, f32); 6] = [
    (v3(-2.55, 2.8, -2.5), v3(1.0, 0.87, 0.66), 0.70),
    (v3(2.55, 2.6, -8.0), v3(1.0, 0.83, 0.61), 0.65),
    (v3(-2.55, 2.8, -14.0), v3(1.0, 0.86, 0.65), 0.75),
    (v3(2.55, 2.6, -20.0), v3(1.0, 0.82, 0.60), 0.65),
    (v3(0.0, 4.8, -23.0), v3(1.0, 0.89, 0.72), 0.70),
    (v3(0.0, 4.2, -28.5), v3(0.95, 0.87, 0.71), 1.05),
];

impl Shader for Metal {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        if matches!(self.material, Material::Luz) {
            return [0.85, 0.49, 0.20, 1.0];
        }
        let v = (c.cam - f.pos).norm();
        let mut n = f.normal.norm();
        if n.dot(v) < 0.0 {
            n = -n;
        }
        if matches!(self.material, Material::Cristal) {
            let borde = (1.0 - n.dot(v).abs()).powi(3);
            // Reflejos estrechos e incompletos: el centro queda despejado.
            let diagonal = f.pos.x * 0.74 + (f.pos.y - CENTRO_Y) * 0.45;
            let linea = (1.0 - smoothstep(0.012, 0.065, (diagonal - 0.85).abs())) * 0.22
                + (1.0 - smoothstep(0.006, 0.024, (diagonal + 0.92).abs())) * 0.08;
            let alfa = (0.075
                + borde * 0.18
                + linea
                + (1.0 - smoothstep(0.12, 0.22, (diagonal - 0.65).abs())) * 0.045)
                .min(0.38);
            return [
                0.62 + linea * 0.4,
                0.58 + linea * 0.4,
                0.46 + linea * 0.3,
                alfa,
            ];
        }
        let (base, brillo) = match self.material {
            Material::Lamina => (rgb(118.0, 87.0, 53.0), 0.65),
            Material::Cobre => (rgb(133.0, 81.0, 43.0), 0.72),
            Material::Bronce => (rgb(144.0, 109.0, 61.0), 0.8),
            Material::Nervio => (rgb(77.0, 58.0, 37.0), 0.9),
            Material::Piso => (rgb(80.0, 65.0, 43.0), 0.35),
            Material::Camara => (rgb(107.0, 88.0, 60.0), 0.5),
            _ => unreachable!(),
        };
        let p = f.pos;
        let desgaste = fbm(p * 2.1, 2, 1101) * 0.5 + 0.5;
        let rayado = ruido(v3(p.x * 24.0, p.y * 1.3, p.z * 3.0), 1102) * 0.045;
        let base = base * (0.77 + desgaste * 0.37 + rayado);
        let mut luz = v3(0.085, 0.073, 0.055);
        let mut reflejo = V3::ZERO;
        for (pos, color, potencia) in LUCES {
            let dv = pos - p;
            let distancia = dv.len().max(0.01);
            let atenuacion = potencia / (1.0 + distancia * distancia * 0.14);
            let l = dv * (1.0 / distancia);
            luz += color * (n.dot(l).max(0.0) * atenuacion * 0.95);
            let h = (l + v).norm();
            let especular = n.dot(h).max(0.0).powf(38.0);
            reflejo += color * (especular * atenuacion * brillo * 1.1);
        }
        let mut col = base * luz + reflejo;
        if matches!(self.material, Material::Piso) {
            let junta_z = (p.z / 1.3).rem_euclid(1.0);
            let junta_x = (p.x / 0.6).rem_euclid(1.0);
            if junta_z < 0.014 || junta_x < 0.012 {
                col = col * 0.32;
            }
        }
        let niebla = smoothstep(16.0, 45.0, (c.cam - p).len()) * 0.15;
        col = mezcla(col, rgb(12.0, 9.0, 5.0), niebla);
        [col.x, col.y, col.z, 1.0]
    }
}

fn hex(k: usize, radio: f32, z: f32) -> V3 {
    let a = (k % 6) as f32 * TAU / 6.0;
    v3(a.cos() * radio, CENTRO_Y + a.sin() * radio, z)
}

fn triangulo(g: &mut Geo, mut p: [V3; 3], hacia: V3) {
    if (p[1] - p[0]).cross(p[2] - p[0]).dot(hacia) < 0.0 {
        p.swap(1, 2);
    }
    let n = (p[1] - p[0]).cross(p[2] - p[0]).norm();
    let b = g.pos.len() as u32;
    for v in p {
        g.pos.push(v);
        g.nrm.push(n);
        g.uv.push([v.x, v.z]);
    }
    g.idx.push([b, b + 1, b + 2]);
}

// Viga rectangular orientada entre dos puntos; sirve para nervios, marcos y costuras.
fn viga(g: &mut Geo, a: V3, b: V3, grosor: f32) {
    let f = (b - a).norm();
    let aux = if f.y.abs() > 0.95 {
        v3(1.0, 0.0, 0.0)
    } else {
        v3(0.0, 1.0, 0.0)
    };
    let r = f.cross(aux).norm() * (grosor * 0.5);
    let u = r.norm().cross(f) * (grosor * 0.5);
    let puntos = [
        a - r - u,
        a + r - u,
        a + r + u,
        a - r + u,
        b - r - u,
        b + r - u,
        b + r + u,
        b - r + u,
    ];
    let centro = (a + b) * 0.5;
    for ids in [
        [0, 1, 2, 3],
        [7, 6, 5, 4],
        [0, 4, 5, 1],
        [1, 5, 6, 2],
        [2, 6, 7, 3],
        [3, 7, 4, 0],
    ] {
        let p = ids.map(|i| puntos[i]);
        g.quad(
            p,
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            (p[0] + p[1] + p[2] + p[3]) * 0.25 - centro,
        );
    }
}

fn marco(g: &mut Geo, radio: f32, z: f32, grosor: f32) {
    for k in 0..6 {
        viga(g, hex(k, radio, z), hex(k + 1, radio, z), grosor);
    }
}

fn anillo(g: &mut Geo, interior: f32, exterior: f32, z: f32) {
    for k in 0..6 {
        g.quad(
            [
                hex(k, interior, z),
                hex(k + 1, interior, z),
                hex(k + 1, exterior, z),
                hex(k, exterior, z),
            ],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            v3(0.0, 0.0, 1.0),
        );
    }
}

pub struct TunelBlipA {
    superficies: Vec<Superficie>,
    mamparo: Vec<Superficie>,
}

impl TunelBlipA {
    pub fn nuevo() -> Self {
        let mut g: Vec<Geo> = (0..8).map(|_| Geo::vacio()).collect();
        let mut rng = Rng::new(817051);
        // Una piel de metal plegado: nodos compartidos, sin grietas entre triángulos.
        const ANG: usize = 36;
        const FILAS: usize = 28;
        let mut nodos = Vec::new();
        for j in 0..=FILAS {
            let z = -LARGO * j as f32 / FILAS as f32;
            for i in 0..ANG {
                let lado = i / 6;
                let u = (i % 6) as f32 / 6.0;
                let p = hex(lado, 3.2, z).lerp(hex(lado + 1, 3.2, z), u);
                let radial = v3(p.x, p.y - CENTRO_Y, 0.0).norm();
                let pliegue = if p.y < 0.12 {
                    rng.rango(0.0, 0.12)
                } else if i % 6 == 0 {
                    rng.rango(-0.04, 0.05)
                } else {
                    rng.rango(-0.24, 0.20)
                };
                nodos.push(p + radial * pliegue);
            }
        }
        for j in 0..FILAS {
            for i in 0..ANG {
                let ni = (i + 1) % ANG;
                let p = [
                    nodos[j * ANG + i],
                    nodos[j * ANG + ni],
                    nodos[(j + 1) * ANG + ni],
                    nodos[(j + 1) * ANG + i],
                ];
                let centro = (p[0] + p[1] + p[2] + p[3]) * 0.25;
                let adentro = v3(-centro.x, CENTRO_Y - centro.y, 0.0);
                let material = (rng.siguiente() % 3) as usize;
                triangulo(&mut g[material], [p[0], p[1], p[2]], adentro);
                triangulo(&mut g[material], [p[0], p[2], p[3]], adentro);
                // Aristas aisladas: relieves irregulares, sin convertirlo en rejilla.
                if i % 6 != 0 && rng.f() < 0.22 {
                    viga(
                        &mut g[3],
                        p[0] - adentro.norm() * 0.01,
                        p[2] + adentro.norm() * 0.015,
                        0.022,
                    );
                }
            }
        }
        // Pasarela central oscura: no ocupa todo el ancho del túnel.
        g[4].caja(v3(0.0, 0.07, -12.0), v3(2.5, 0.14, 24.0));
        for x in [-1.28, 1.28] {
            viga(&mut g[2], v3(x, 0.16, 0.0), v3(x, 0.16, -24.0), 0.035);
        }
        for j in 0..8 {
            let z = -(j as f32) * 3.2;
            marco(&mut g[3], 3.05, z, 0.13);
            marco(&mut g[2], 3.015, z - 0.09, 0.028);
        }
        for k in 0..6 {
            viga(&mut g[3], hex(k, 3.04, -0.1), hex(k, 3.04, -23.9), 0.075);
        }
        // Líneas guía y pequeños faroles cálidos, sin iluminación de neón.
        for j in 0..12 {
            for x in [-1.24, 1.24] {
                g[6].caja(v3(x, 0.155, -1.0 - j as f32 * 2.0), v3(0.028, 0.012, 0.16));
            }
        }
        for (p, _, _) in &LUCES[..4] {
            g[3].caja(*p, v3(0.18, 0.34, 0.52));
            g[6].caja(
                *p + v3(-p.x.signum() * 0.10, 0.0, 0.0),
                v3(0.02, 0.17, 0.30),
            );
        }
        let mut puerta: Vec<Geo> = (0..8).map(|_| Geo::vacio()).collect();
        // Mamparo final, bisel y marco hexagonal. El centro contiene vidrio real con alfa.
        anillo(&mut puerta[0], 2.25, 3.25, -24.02);
        anillo(&mut puerta[3], 2.08, 2.27, -24.04);
        marco(&mut puerta[2], 2.22, -23.97, 0.16);
        marco(&mut puerta[1], 2.09, -24.07, 0.045);
        for k in 0..6 {
            let p = hex(k, 2.30, -23.91);
            g[3].caja(p, v3(0.16, 0.19, 0.12));
            g[6].caja(p + v3(0.0, 0.0, 0.07), v3(0.035, 0.055, 0.008));
            triangulo(
                &mut puerta[7],
                [
                    v3(0.0, CENTRO_Y, Z_CRISTAL),
                    hex(k, RADIO_CRISTAL, Z_CRISTAL),
                    hex(k + 1, RADIO_CRISTAL, Z_CRISTAL),
                ],
                v3(0.0, 0.0, 1.0),
            );
        }
        // Cámara de Rocky al otro lado del mamparo móvil.
        for k in 0..6 {
            g[5].quad(
                [
                    hex(k, 2.95, -24.2),
                    hex(k + 1, 2.95, -24.2),
                    hex(k + 1, 2.95, -31.5),
                    hex(k, 2.95, -31.5),
                ],
                [[0.0, 0.0], [2.0, 0.0], [2.0, 7.0], [0.0, 7.0]],
                v3(-hex(k, 1.0, 0.0).x, CENTRO_Y - hex(k, 1.0, 0.0).y, 0.0),
            );
            triangulo(
                &mut g[5],
                [
                    v3(0.0, CENTRO_Y, -31.5),
                    hex(k, 2.95, -31.5),
                    hex(k + 1, 2.95, -31.5),
                ],
                v3(0.0, 0.0, 1.0),
            );
        }
        g[4].caja(v3(0.0, 0.07, -27.7), v3(2.5, 0.14, 7.4));
        for z in [-26.4, -29.0, -31.3] {
            marco(&mut g[3], 2.85, z, 0.11);
        }
        marco(&mut g[2], 1.75, -31.44, 0.06);
        for x in [-1.7, 1.7] {
            g[6].caja(v3(x, 1.6, -31.42), v3(0.035, 1.0, 0.012));
        }
        let materiales = [
            Material::Lamina,
            Material::Cobre,
            Material::Bronce,
            Material::Nervio,
            Material::Piso,
            Material::Camara,
            Material::Luz,
            Material::Cristal,
        ];
        let construir = |g: Vec<Geo>| {
            g.into_iter()
                .zip(materiales)
                .map(|(geo, material)| Superficie {
                    malla: geo.a_malla(),
                    shader: Metal { material },
                    mezcla: if matches!(material, Material::Cristal) {
                        Mezcla::Alfa
                    } else {
                        Mezcla::Opaco
                    },
                })
                .collect()
        };
        Self {
            superficies: construir(g),
            mamparo: construir(puerta),
        }
    }

    /// Posición de observación; se puede sustituir por la cámara del futuro jugador.
    pub fn camara(ancho: usize, alto: usize) -> Camara {
        let mut cam = Camara::nueva(ancho as f32, alto as f32);
        cam.pos = v3(0.0, 1.72, -1.0);
        cam.fov = 65f32.to_radians();
        cam.mirar_a(v3(0.0, 2.45, Z_CRISTAL));
        cam
    }

    /// Se pueden incorporar estos items al renderer de otra escena.
    pub fn items(&self) -> Vec<Item<'_>> {
        self.items_con_apertura(0.0)
    }

    fn items_con_apertura(&self, apertura: f32) -> Vec<Item<'_>> {
        self.superficies
            .iter()
            .map(|s| (s, V3::ZERO))
            .chain(
                self.mamparo
                    .iter()
                    .map(|s| (s, v3(0.0, apertura * 6.5, 0.0))),
            )
            .map(|(s, pos)| Item {
                malla: &s.malla,
                shader: &s.shader,
                pos,
                rot: Rot::identidad(),
                escala: v3(1.0, 1.0, 1.0),
                mezcla: s.mezcla,
                doble_cara: true,
                tinte: v3(1.0, 1.0, 1.0),
                mip: 0.0,
                dist: 0.0,
            })
            .collect()
    }

    pub fn dibujar(&self, fb: &mut Framebuffer, cam: &Camara, tiempo: f32) {
        self.dibujar_con_items(fb, cam, tiempo, vec![]);
    }

    /// Los extras (por ejemplo un OBJ detrás del cristal) se dibujan antes del vidrio.
    pub fn dibujar_con_items<'a>(
        &'a self,
        fb: &mut Framebuffer,
        cam: &Camara,
        tiempo: f32,
        extras: Vec<Item<'a>>,
    ) {
        self.dibujar_encuentro(fb, cam, tiempo, extras, 0.0);
    }

    pub fn dibujar_encuentro<'a>(
        &'a self,
        fb: &mut Framebuffer,
        cam: &Camara,
        tiempo: f32,
        extras: Vec<Item<'a>>,
        apertura: f32,
    ) {
        let mut items = self.items_con_apertura(apertura);
        items.extend(extras);
        let (mut opacos, mut transparentes): (Vec<_>, Vec<_>) =
            items.into_iter().partition(|i| i.mezcla == Mezcla::Opaco);
        for i in &mut transparentes {
            i.dist = (cam.pos - i.pos).len();
        }
        transparentes.sort_by(|a, b| b.dist.total_cmp(&a.dist));
        opacos.extend(transparentes);
        let items = opacos;
        for p in fb.pixels.chunks_exact_mut(4) {
            p.copy_from_slice(&[5, 4, 3, 255]);
        }
        fb.zbuf.fill(0.0);
        let vista = Vista::de(cam, fb.width, fb.height);
        let mut tris = vec![];
        for (id, item) in items.iter().enumerate() {
            preparar_item(item, id as u32, &vista, &mut tris);
        }
        let ctx = Contexto {
            cam: cam.pos,
            tiempo,
            sol: v3(0.0, 4.0, -23.0),
        };
        let w = fb.width;
        const BANDA: usize = 20;
        let trabajos = Mutex::new(
            fb.pixels
                .chunks_mut(w * 4 * BANDA)
                .zip(fb.zbuf.chunks_mut(w * BANDA))
                .enumerate(),
        );
        std::thread::scope(|s| {
            for _ in 0..nucleos() {
                s.spawn(|| loop {
                    let sig = trabajos.lock().unwrap().next();
                    let Some((b, (color, z))) = sig else { break };
                    rasterizar_banda(b * BANDA, color, z, w, &tris, &items, &ctx);
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn objetos_detras_del_cristal_siguen_visibles_con_el_zbuffer() {
        struct Rojo;
        impl Shader for Rojo {
            fn shade(&self, _: &Fragmento, _: &Contexto) -> [f32; 4] {
                [1.0, 0.0, 0.0, 1.0]
            }
        }
        let tunel = TunelBlipA::nuevo();
        let mut cam = TunelBlipA::camara(160, 100);
        cam.pos = v3(0.0, CENTRO_Y, -19.0);
        cam.mirar_a(v3(0.0, CENTRO_Y, -28.0));
        let mut geo = Geo::vacio();
        geo.caja(v3(0.0, CENTRO_Y, -27.0), v3(0.8, 0.8, 0.4));
        let malla = geo.a_malla();
        let shader = Rojo;
        let item = Item {
            malla: &malla,
            shader: &shader,
            pos: V3::ZERO,
            rot: Rot::identidad(),
            escala: v3(1.0, 1.0, 1.0),
            mezcla: Mezcla::Opaco,
            doble_cara: true,
            tinte: v3(1.0, 1.0, 1.0),
            mip: 0.0,
            dist: 0.0,
        };
        let mut fb = Framebuffer::new(160, 100);
        tunel.dibujar_con_items(&mut fb, &cam, 0.0, vec![item]);
        let i = (50 * 160 + 80) * 4;
        assert!(
            fb.pixels[i] > 210 && fb.pixels[i + 1] < 65,
            "el vidrio oculta el objeto: {:?}",
            &fb.pixels[i..i + 3]
        );
    }
    #[test]
    fn cristal_hexagonal_semitransparente_y_camara_vacia_detras() {
        let t = TunelBlipA::nuevo();
        let cristal = t.mamparo.iter().find(|s| s.mezcla == Mezcla::Alfa).unwrap();
        assert_eq!(cristal.malla.idx.len(), 6);
        assert!(POSICION_PERSONAJE.z < Z_CRISTAL);
        let f = Fragmento {
            pos: v3(0.0, CENTRO_Y, Z_CRISTAL),
            normal: v3(0.0, 0.0, 1.0),
            uv: [0.5, 0.5],
            tinte: v3(1.0, 1.0, 1.0),
            centro: V3::ZERO,
            mip: 0.0,
        };
        let ctx = Contexto {
            cam: v3(0.0, CENTRO_Y, -18.0),
            tiempo: 0.0,
            sol: V3::ZERO,
        };
        let alfa = cristal.shader.shade(&f, &ctx)[3];
        assert!(alfa > 0.0 && alfa < 0.15);
        assert!(
            t.superficies
                .iter()
                .map(|s| s.malla.idx.len())
                .sum::<usize>()
                < 15000
        );
    }
}
