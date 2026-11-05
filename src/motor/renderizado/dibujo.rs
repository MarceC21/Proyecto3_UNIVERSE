// Tipos compartidos entre la escena (qué dibujar) y el renderer (cómo).
use crate::malla::Malla;
use crate::math::*;
use crate::shader::Shader;

#[derive(Clone, Copy, PartialEq)]
pub enum Mezcla {
    Opaco,
    Alfa,    // dst = dst*(1-a) + src*a
    Aditivo, // dst = dst + src*a  (brillos)
}

// Un objeto listo para dibujar en este cuadro.
pub struct Item<'a> {
    pub malla: &'a Malla,
    pub shader: &'a dyn Shader,
    pub pos: V3,
    pub rot: Rot,
    pub escala: V3,
    pub mezcla: Mezcla,
    pub doble_cara: bool,
    pub tinte: V3,
    pub mip: f32,
    pub dist: f32, // distancia a la cámara (para ordenar transparentes)
}

// Interruptores para comparar el efecto de las optimizaciones en vivo.
#[derive(Clone, Copy)]
pub struct Ajustes {
    pub culling: bool,
    pub lod: bool,
    pub orbitas: bool,
    pub limite: bool,
}

impl Default for Ajustes {
    fn default() -> Self { Ajustes { culling: true, lod: true, orbitas: true, limite: true } }
}

// Contadores simples de trabajo (lo pide la rúbrica para culling y LOD).
#[derive(Clone, Copy, Default)]
pub struct Stats {
    pub objetos_total: usize,
    pub objetos_enviados: usize,
    pub tris_sin_optimizar: usize, // si todo se dibujara a máximo detalle
    pub tris_enviados: usize,      // después de culling + LOD
    pub tris_raster: usize,        // después de recorte y descarte de caras traseras
}
