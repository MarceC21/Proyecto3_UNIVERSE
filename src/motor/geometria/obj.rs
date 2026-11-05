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

        let mut materiales = String::new();
        for linea in texto.lines() {
            let mut partes = linea.split_whitespace();
            if partes.next() == Some("mtllib") {
                if let Some(nombre) = partes.next() {
                    materiales = fs::read_to_string(format!("{carpeta}/{nombre}")).unwrap_or_default();
                }
            }
        }
        Self::desde_textos(&texto, &materiales).map_err(|e| format!("{ruta}: {e}"))
    }

    // También permite empaquetar los recursos con include_str!, sin depender
    // del directorio desde el que se ejecuta el juego.
    pub fn desde_textos(texto: &str, mtl: &str) -> Result<Modelo, String> {
        let mut vertices: Vec<Vec3> = Vec::new();
        let mut caras: Vec<Cara> = Vec::new();
        let mut materiales: Vec<Material> = Vec::new();
        let mut indice_mat: HashMap<String, usize> = HashMap::new();
        let colores = leer_mtl(mtl);
        let mut actual = 0usize;

        for linea in texto.lines() {
            let mut partes = linea.split_whitespace();
            match partes.next() {
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
                    let ids: Vec<usize> = partes.take_while(|p| !p.starts_with('#'))
                        .map(|p| {
                            let i = p.split('/').next().unwrap_or("").parse::<isize>()
                                .map_err(|_| format!("índice OBJ inválido: {p}"))?;
                            let id = if i > 0 { i - 1 } else { vertices.len() as isize + i };
                            if i == 0 || id < 0 || id as usize >= vertices.len() {
                                return Err(format!("índice OBJ fuera de rango: {i}"));
                            }
                            Ok(id as usize)
                        }).collect::<Result<_, String>>()?;
                    if ids.len() < 3 { return Err("cara OBJ con menos de tres vértices".into()); }
                    if materiales.is_empty() {
                        materiales.push(Material { nombre: "Default".into(), color: [0.7; 3] });
                    }
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
            return Err("el modelo no tiene vértices o caras".into());
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

fn leer_mtl(texto: &str) -> HashMap<String, [f32; 3]> {
    let mut mapa = HashMap::new();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_invalidos_devuelven_error() {
        let vertices = "v 0 0 0\nv 1 0 0\nv 0 1 0\n";
        for cara in ["f 0 2 3", "f 1 2 4", "f -4 -2 -1", "f x 2 3"] {
            assert!(Modelo::desde_textos(&format!("{vertices}{cara}"), "").is_err());
        }
        let m = Modelo::desde_textos(&format!("{vertices}f -3 -2 -1"), "").unwrap();
        assert_eq!(m.caras[0].v, [0, 1, 2]);
        assert_eq!(m.materiales.len(), 1);
    }
}
