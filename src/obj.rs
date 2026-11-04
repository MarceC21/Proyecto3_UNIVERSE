// Cargador de .obj / .mtl escrito a mano (sin librerías).
//
// Lo que nos interesa del archivo:
//   v  x y z          -> un vértice (posición)
//   usemtl nombre     -> las caras siguientes usan ese material
//   f  a/b/c d/e/f .. -> una cara: lista de vértices como "v/vt/vn"
//                        OJO: los índices del OBJ empiezan en 1, no en 0.
use std::collections::HashMap;
use std::fs;

pub type Vec3 = [f32; 3];

// Un triángulo del modelo: tres ÍNDICES al arreglo de vértices.
#[derive(Clone, Copy)]
pub struct Cara {
    pub v: [usize; 3],
    pub material: usize,
}

pub struct Material {
    pub nombre: String,
    pub color: [f32; 3], // Kd, de 0 a 1
}

pub struct Modelo {
    pub vertices: Vec<Vec3>,
    pub caras: Vec<Cara>,
    pub materiales: Vec<Material>,
}

impl Modelo {
    pub fn cargar(ruta: &str) -> Result<Modelo, String> {
        let texto = fs::read_to_string(ruta).map_err(|e| format!("{ruta}: {e}"))?;
        let carpeta = ruta.rsplit_once('/').map(|(c, _)| c).unwrap_or(".");

        let mut vertices: Vec<Vec3> = Vec::new();
        let mut caras: Vec<Cara> = Vec::new();
        let mut materiales: Vec<Material> = Vec::new();
        let mut indice_mat: HashMap<String, usize> = HashMap::new();
        let mut colores: HashMap<String, [f32; 3]> = HashMap::new();
        let mut actual = 0usize;

        for linea in texto.lines() {
            let mut partes = linea.split_whitespace();
            match partes.next() {
                Some("mtllib") => {
                    if let Some(nombre) = partes.next() {
                        colores = leer_mtl(&format!("{carpeta}/{nombre}"));
                    }
                }
                Some("v") => {
                    let n: Vec<f32> = partes.filter_map(|s| s.parse().ok()).collect();
                    if n.len() >= 3 {
                        vertices.push([n[0], n[1], n[2]]);
                    }
                }
                Some("usemtl") => {
                    let nombre = partes.next().unwrap_or("").to_string();
                    actual = *indice_mat.entry(nombre.clone()).or_insert_with(|| {
                        let color = colores.get(&nombre).copied().unwrap_or([0.7, 0.7, 0.7]);
                        materiales.push(Material { nombre: nombre.clone(), color });
                        materiales.len() - 1
                    });
                }
                Some("f") => {
                    // Solo nos importa el primer número de cada "v/vt/vn".
                    let ids: Vec<usize> = partes
                        .filter_map(|p| p.split('/').next()?.parse::<usize>().ok())
                        .map(|i| i - 1) // el OBJ cuenta desde 1
                        .collect();
                    // Triangulación en abanico: un cuadrado (4) -> 2 triángulos.
                    // (Tu OBJ ya viene triangulado, pero así acepta cualquier cara.)
                    for k in 1..ids.len().saturating_sub(1) {
                        caras.push(Cara { v: [ids[0], ids[k], ids[k + 1]], material: actual });
                    }
                }
                _ => {}
            }
        }
        if vertices.is_empty() || caras.is_empty() {
            return Err(format!("{ruta}: el modelo no tiene vértices o caras"));
        }
        Ok(Modelo { vertices, caras, materiales })
    }

    // Centro y radio de la caja que envuelve al modelo (para encuadrarlo).
    pub fn centro_y_radio(&self) -> (Vec3, f32) {
        let mut mn = [f32::MAX; 3];
        let mut mx = [f32::MIN; 3];
        for v in &self.vertices {
            for i in 0..3 {
                mn[i] = mn[i].min(v[i]);
                mx[i] = mx[i].max(v[i]);
            }
        }
        let c = [(mn[0] + mx[0]) * 0.5, (mn[1] + mx[1]) * 0.5, (mn[2] + mx[2]) * 0.5];
        let d = [mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]];
        (c, (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() * 0.5)
    }
}

fn leer_mtl(ruta: &str) -> HashMap<String, [f32; 3]> {
    let mut mapa = HashMap::new();
    let Ok(texto) = fs::read_to_string(ruta) else { return mapa };
    let mut nombre = String::new();
    for linea in texto.lines() {
        let mut p = linea.split_whitespace();
        match p.next() {
            Some("newmtl") => nombre = p.next().unwrap_or("").to_string(),
            Some("Kd") => {
                let n: Vec<f32> = p.filter_map(|s| s.parse().ok()).collect();
                if n.len() >= 3 {
                    mapa.insert(nombre.clone(), [n[0], n[1], n[2]]);
                }
            }
            _ => {}
        }
    }
    mapa
}