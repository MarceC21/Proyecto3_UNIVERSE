// Renderiza varias vistas del sistema solar a archivos .ppm (sin ventana).
// Uso: cargo run --release --no-default-features --bin exportar [carpeta]
use nave_raster::{camara::Camara, cielo::Cielo, dibujo::Ajustes, escena::Escena, raster::Framebuffer, render::dibujar};
use std::io::Write;
use std::time::Instant;

const ANCHO: usize = 960;
const ALTO: usize = 600;

fn main() {
    let carpeta = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let t0 = Instant::now();
    let escena = Escena::nueva();
    let cielo = Cielo::nuevo();
    println!("Texturas y escena listas en {:.2}s", t0.elapsed().as_secs_f32());
    let mut fb = Framebuffer::new(ANCHO, ALTO);
    let aj = Ajustes::default();
    let vistas = escena.vistas();
    for (nombre, pos, objetivo) in vistas {
        let mut cam = Camara::nueva(ANCHO as f32, ALTO as f32);
        cam.pos = pos;
        cam.mirar_a(objetivo);
        let t = Instant::now();
        let st = dibujar(&mut fb, &escena, &cielo, &cam, 3.0, &aj);
        println!("{:<9} {:>6.1} ms | objetos {}/{} | tris enviados {} / {} sin optimizar | raster {}",
            nombre, t.elapsed().as_secs_f32() * 1000.0, st.objetos_enviados, st.objetos_total,
            st.tris_enviados, st.tris_sin_optimizar, st.tris_raster);
        let mut f = std::fs::File::create(format!("{}/vista_{}.ppm", carpeta, nombre)).unwrap();
        write!(f, "P6\n{} {}\n255\n", ANCHO, ALTO).unwrap();
        for p in fb.pixels.chunks_exact(4) { f.write_all(&p[..3]).unwrap(); }
    }
}
