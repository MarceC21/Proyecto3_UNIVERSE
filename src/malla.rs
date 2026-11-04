// Mallas de triángulos generadas por código: esferas, asteroides deformados con
// ruido, anillos de órbita, muro del límite, etc.
use crate::math::*;
use crate::ruido::*;
use std::f32::consts::{PI, TAU};

pub struct Malla {
    pub pos: Vec<V3>,
    pub nrm: Vec<V3>,
    pub uv: Vec<[f32; 2]>,
    pub idx: Vec<[u32; 3]>,
    pub radio: f32, // radio de la esfera envolvente (para culling)
}

impl Malla {
    fn calcular_radio(&mut self) {
        self.radio = self.pos.iter().map(|p| p.len()).fold(0.0, f32::max);
    }

    // Convierte a "sopa de triángulos" con normal por cara (aspecto facetado).
    fn facetada(&self) -> Malla {
        let (mut pos, mut nrm, mut uv, mut idx) = (vec![], vec![], vec![], vec![]);
        for t in &self.idx {
            let (a, b, c) = (self.pos[t[0] as usize], self.pos[t[1] as usize], self.pos[t[2] as usize]);
            let n = (b - a).cross(c - a).norm();
            let base = pos.len() as u32;
            for k in 0..3 {
                pos.push(self.pos[t[k] as usize]);
                nrm.push(n);
                uv.push(self.uv[t[k] as usize]);
            }
            idx.push([base, base + 1, base + 2]);
        }
        let mut m = Malla { pos, nrm, uv, idx, radio: 0.0 };
        m.calcular_radio();
        m
    }
}

// Esfera unidad. u = longitud (0..1), v = latitud (0 = polo norte). La misma
// parametrización que usan las texturas horneadas, por eso coinciden.
pub fn esfera(nlon: usize, nlat: usize) -> Malla {
    let (mut pos, mut uv, mut idx) = (vec![], vec![], vec![]);
    for j in 0..=nlat {
        let v = j as f32 / nlat as f32;
        let lat = (0.5 - v) * PI;
        for i in 0..=nlon {
            let u = i as f32 / nlon as f32;
            let lon = u * TAU;
            pos.push(v3(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin()));
            uv.push([u, v]);
        }
    }
    for j in 0..nlat {
        for i in 0..nlon {
            let a = (j * (nlon + 1) + i) as u32;
            let (b, c) = (a + 1, a + (nlon + 1) as u32);
            let d = c + 1;
            idx.push([a, b, c]);
            idx.push([b, d, c]);
        }
    }
    let nrm = pos.clone(); // en la esfera unidad la normal es la posición
    Malla { pos, nrm, uv, idx, radio: 1.0 }
}

// Asteroide: esfera de baja resolución deformada por ruido (misma semilla =>
// misma forma en todos los niveles de detalle) y con normales por cara.
pub fn asteroide(semilla: u32, nlon: usize, nlat: usize) -> Malla {
    let mut rng = Rng::new(semilla as u64 * 7919 + 13);
    let estira = v3(rng.rango(0.75, 1.3), rng.rango(0.7, 1.15), rng.rango(0.75, 1.3));
    let off = v3(rng.rango(0.0, 50.0), rng.rango(0.0, 50.0), rng.rango(0.0, 50.0));
    let mut m = esfera(nlon, nlat);
    for p in m.pos.iter_mut() {
        let d = *p;
        let k = 1.0 + 0.36 * fbm(d * 1.5 + off, 3, semilla) + 0.07 * ruido(d * 4.5 + off, semilla + 1);
        *p = v3(d.x * k * estira.x, d.y * k * estira.y, d.z * k * estira.z);
    }
    m.facetada()
}

// Anillo plano en XZ. uv.x da `repeticiones` vueltas de patrón (para líneas
// punteadas) y uv.y va de 0 a 1 a lo ancho.
pub fn anillo(r_int: f32, r_ext: f32, seg: usize, repeticiones: f32) -> Malla {
    let (mut pos, mut uv, mut idx) = (vec![], vec![], vec![]);
    for i in 0..=seg {
        let t = i as f32 / seg as f32;
        let (s, c) = (t * TAU).sin_cos();
        pos.push(v3(r_int * c, 0.0, r_int * s));
        uv.push([t * repeticiones, 0.0]);
        pos.push(v3(r_ext * c, 0.0, r_ext * s));
        uv.push([t * repeticiones, 1.0]);
    }
    for i in 0..seg {
        let a = (i * 2) as u32;
        idx.push([a, a + 1, a + 2]);
        idx.push([a + 1, a + 3, a + 2]);
    }
    let nrm = vec![v3(0.0, 1.0, 0.0); pos.len()];
    let mut m = Malla { pos, nrm, uv, idx, radio: 0.0 };
    m.calcular_radio();
    m
}

// Muro cilíndrico (el límite del sistema solar): uv.x = ángulo 0..1, uv.y = altura 0..1.
pub fn muro(radio: f32, y0: f32, y1: f32, seg: usize) -> Malla {
    let (mut pos, mut nrm, mut uv, mut idx) = (vec![], vec![], vec![], vec![]);
    for i in 0..=seg {
        let t = i as f32 / seg as f32;
        let (s, c) = (t * TAU).sin_cos();
        pos.push(v3(radio * c, y0, radio * s));
        nrm.push(v3(c, 0.0, s));
        uv.push([t, 0.0]);
        pos.push(v3(radio * c, y1, radio * s));
        nrm.push(v3(c, 0.0, s));
        uv.push([t, 1.0]);
    }
    for i in 0..seg {
        let a = (i * 2) as u32;
        idx.push([a, a + 1, a + 2]);
        idx.push([a + 1, a + 3, a + 2]);
    }
    let mut m = Malla { pos, nrm, uv, idx, radio: 0.0 };
    m.calcular_radio();
    m
}

// Cuadrado de -1..1 en XY mirando a +Z (para sprites que miran a la cámara).
pub fn cuadrado() -> Malla {
    Malla {
        pos: vec![v3(-1.0, -1.0, 0.0), v3(1.0, -1.0, 0.0), v3(1.0, 1.0, 0.0), v3(-1.0, 1.0, 0.0)],
        nrm: vec![v3(0.0, 0.0, 1.0); 4],
        uv: vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        idx: vec![[0, 1, 2], [0, 2, 3]],
        radio: 1.5,
    }
}

// Octaedro unidad (las balizas del límite).
pub fn octaedro() -> Malla {
    let v = [v3(1.0, 0.0, 0.0), v3(-1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0),
             v3(0.0, -1.0, 0.0), v3(0.0, 0.0, 1.0), v3(0.0, 0.0, -1.0)];
    let caras = [[0, 2, 4], [4, 2, 1], [1, 2, 5], [5, 2, 0], [4, 3, 0], [1, 3, 4], [5, 3, 1], [0, 3, 5]];
    let base = Malla {
        pos: v.to_vec(),
        nrm: v.to_vec(),
        uv: vec![[0.0, 0.0]; 6],
        idx: caras.iter().map(|c| [c[0] as u32, c[1] as u32, c[2] as u32]).collect(),
        radio: 1.0,
    };
    base.facetada()
}
