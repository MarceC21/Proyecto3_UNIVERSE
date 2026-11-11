// Vistas de revisión con el rasterizador de producción, sin ventana.
use nave_raster::{
    camara::Camara, cielo::Cielo, dibujo::Ajustes, escena::Escena, nave::Nave, raster::Framebuffer,
    render::dibujar_con_nave, seguimiento::Seguimiento,
};
use std::{io::Write, time::Instant};
fn guardar(fb: &Framebuffer, ruta: &std::path::Path) {
    let mut f = std::io::BufWriter::new(std::fs::File::create(ruta).unwrap());
    write!(f, "P6\n{} {}\n255\n", fb.width, fb.height).unwrap();
    for p in fb.pixels.chunks_exact(4) {
        f.write_all(&p[..3]).unwrap();
    }
}
fn main() {
    let carpeta = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "/tmp/eridani".into()),
    );
    std::fs::create_dir_all(&carpeta).unwrap();
    let t = Instant::now();
    let escena = Escena::eridani();
    let cielo = Cielo::eridani();
    println!(
        "40 Eridani: {:.2}s, {} cuerpos, {} asteroides",
        t.elapsed().as_secs_f32(),
        escena.cuerpos.len(),
        escena.colisionadores_asteroides().len()
    );
    let mut fb = Framebuffer::new(960, 600);
    for (nombre, pos, obj) in escena.vistas() {
        let mut cam = Camara::nueva(960.0, 600.0);
        cam.pos = pos;
        cam.mirar_a(obj);
        let t = Instant::now();
        let st = dibujar_con_nave(
            &mut fb,
            &escena,
            &cielo,
            &cam,
            2.0,
            &Ajustes::default(),
            None,
        );
        println!(
            "{nombre}: {:.1}ms, {} / {} tris",
            t.elapsed().as_secs_f32() * 1000.0,
            st.tris_enviados,
            st.tris_sin_optimizar
        );
        guardar(
            &fb,
            &carpeta.join(format!("{}.ppm", nombre.replace(' ', "_"))),
        );
    }
    let mut nave = Nave::predeterminada().unwrap();
    nave.teletransportar(nave_raster::eridani::INICIO, nave_raster::math::V3::ZERO);
    let mut cam = Camara::nueva(960.0, 600.0);
    Seguimiento::nuevo(nave.fwd, nave.up).actualizar(
        &mut cam,
        nave.pos,
        nave.fwd,
        nave.up,
        0.0,
        1.0 / 60.0,
        None,
        &escena.esferas(),
    );
    dibujar_con_nave(
        &mut fb,
        &escena,
        &cielo,
        &cam,
        2.0,
        &Ajustes::default(),
        Some(&nave),
    );
    guardar(&fb, &carpeta.join("llegada.ppm"));
}
