// Renderiza la escena de Miller a .ppm sin ventana.
// Uso: cargo run --release --no-default-features --bin exportar_miller [carpeta]
use nave_raster::{
    mision_miller::{Configuracion, MisionMiller},
    nave::Nave,
    paisaje_miller::EscenaMiller,
    raster::Framebuffer,
};
use std::io::Write;

const ANCHO: usize = 960;
const ALTO: usize = 600;

fn guardar(fb: &Framebuffer, ruta: &str) {
    let mut f = std::fs::File::create(ruta).unwrap();
    write!(f, "P6\n{} {}\n255\n", fb.width, fb.height).unwrap();
    for p in fb.pixels.chunks_exact(4) {
        f.write_all(&p[..3]).unwrap();
    }
}

fn main() {
    let carpeta = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    std::fs::create_dir_all(&carpeta).unwrap();
    let mut escena = EscenaMiller::nueva();
    let nave = Nave::predeterminada().unwrap();
    let cam = EscenaMiller::camara(ANCHO, ALTO);
    let mut fb = Framebuffer::new(ANCHO, ALTO);
    for (nombre, t) in [
        ("miller_aproximacion", 0.1),
        ("miller_descenso", 0.55),
        ("miller_contacto", 1.0),
    ] {
        escena.dibujar_descenso(&mut fb, &nave, t);
        guardar(&fb, &format!("{carpeta}/{nombre}.ppm"));
    }
    for (nombre, t) in [
        ("miller_t0", 0.0f32),
        ("miller_t60", 60.0),
        ("miller_t100", 100.0),
    ] {
        let mut mision = MisionMiller::nueva(0, Configuracion::default());
        mision.actualizar(t as f64, cam.pos);
        escena.radio_ola = mision.radio_ola();
        let inicio = std::time::Instant::now();
        escena.dibujar_con_nave(&mut fb, &cam, t, Some(&nave));
        println!(
            "{nombre}: {:.0} ms",
            inicio.elapsed().as_secs_f32() * 1000.0
        );
        guardar(&fb, &format!("{carpeta}/{nombre}.ppm"));
    }
}
