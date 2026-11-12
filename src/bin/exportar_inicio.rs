// Fondos del menú e intro con el rasterizador de producción, sin ventana.
use nave_raster::{
    cielo::Cielo,
    escena::Escena,
    escena_inicio::EscenaInicio,
    inicio::{Inicio, Orden},
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
        .unwrap_or_else(|| "/tmp/inicio".into());
    std::fs::create_dir_all(&dir).unwrap();
    let escena = Escena::nueva();
    let cielo = Cielo::nuevo();
    let mut nave = Nave::predeterminada().unwrap();
    let (pos, obj) = nave_raster::guia::inicio_solar(&escena);
    nave.teletransportar(pos, obj);
    let fondos = EscenaInicio::nueva();
    let mut fb = Framebuffer::new(960, 600);
    let mut inicio = Inicio::nuevo();
    inicio.actualizar(2.0);
    fondos.dibujar(&mut fb, &inicio, &escena, &cielo, &nave);
    guardar(&fb, &Path::new(&dir).join("menu_tierra.ppm"));
    for (nombre, t) in [
        ("tierra_inicio", 1.0),
        ("tierra_apagada", 5.4),
        ("cabina_personaje", 8.0),
        ("cabina_giro", 13.0),
    ] {
        let mut inicio = Inicio::nuevo();
        inicio.orden(Orden::Aceptar);
        inicio.actualizar(t);
        let ahora = std::time::Instant::now();
        fondos.dibujar(&mut fb, &inicio, &escena, &cielo, &nave);
        println!("{nombre}: {:.1} ms", ahora.elapsed().as_secs_f32() * 1000.0);
        guardar(&fb, &Path::new(&dir).join(format!("{nombre}.ppm")));
    }
    println!("Fondos y personaje de la intro: {dir}");
}
