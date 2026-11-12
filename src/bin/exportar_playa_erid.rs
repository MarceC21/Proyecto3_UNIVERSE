// Imágenes del final usando las mismas mallas, personajes y rasterizador que el juego.
use nave_raster::{
    camara::Camara,
    elenco_erid::{camara_descenso, PlayaErid},
    final_erid::FinalErid,
    math::*,
    nave::Nave,
    raster::Framebuffer,
};
use std::{
    io::{BufWriter, Write},
    path::Path,
};
fn guardar(fb: &Framebuffer, p: &Path) {
    let mut f = BufWriter::new(std::fs::File::create(p).unwrap());
    write!(f, "P6\n{} {}\n255\n", fb.width, fb.height).unwrap();
    for p in fb.pixels.chunks_exact(4) {
        f.write_all(&p[..3]).unwrap();
    }
}
fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/playa-erid".into());
    std::fs::create_dir_all(&dir).unwrap();
    let escena = PlayaErid::nueva();
    let nave = Nave::predeterminada().unwrap();
    let mut fb = Framebuffer::new(960, 600);
    for (nombre, t) in [
        ("descenso_inicio", 0.0),
        ("descenso_mitad", 0.5),
        ("descenso_fin", 1.0),
    ] {
        let cam = camara_descenso(960, 600, t);
        escena.dibujar(&mut fb, &cam, &nave, t * 5.0, Some(t));
        guardar(&fb, &Path::new(&dir).join(format!("{nombre}.ppm")));
    }
    let m = FinalErid::nuevo(false);
    let mut cam = Camara::nueva(960.0, 600.0);
    m.jugador.camara(&mut cam);
    let t = std::time::Instant::now();
    escena.dibujar(&mut fb, &cam, &nave, 6.0, None);
    println!(
        "Playa y personajes: {:.1} ms",
        t.elapsed().as_secs_f32() * 1000.0
    );
    guardar(&fb, &Path::new(&dir).join("rocky_tars.ppm"));
    cam.pos = v3(-38.0, 8.0, 28.0);
    cam.mirar_a(v3(-55.0, 1.0, -14.0));
    escena.dibujar(&mut fb, &cam, &nave, 9.0, None);
    guardar(&fb, &Path::new(&dir).join("arco.ppm"));
    let mut paseo = FinalErid::nuevo(true);
    paseo.actualizar(5.1, &Default::default());
    for (nombre, segundos) in [("paseo_amigos", 12.0), ("paseo_mas_tarde", 50.0)] {
        for _ in 0..(segundos * 60.0) as usize {
            paseo.actualizar(1.0 / 60.0, &Default::default());
        }
        let centro = (paseo.paseo.rocky.pos + paseo.paseo.tars.pos) * 0.5;
        cam.pos = centro + v3(10.0, 4.0, 10.0);
        cam.mirar_a(centro + v3(0.0, 0.8, 0.0));
        escena.dibujar_vivo(&mut fb, &cam, &nave, &paseo);
        guardar(&fb, &Path::new(&dir).join(format!("{nombre}.ppm")));
    }
    paseo.jugador.colocar(25.0, -12.0, std::f32::consts::PI);
    paseo.actualizar(0.02, &Default::default());
    paseo.jugador.camara(&mut cam);
    escena.dibujar_vivo(&mut fb, &cam, &nave, &paseo);
    guardar(&fb, &Path::new(&dir).join("nadando.ppm"));
    println!("Vistas del final: {dir}");
}
