// Vistas del túnel integrado, usando el rasterizador de producción.
use nave_raster::{
    encuentro_rocky::EncuentroRocky,
    materiales::Luz,
    math::*,
    raster::Framebuffer,
    rocky::Rocky,
    tunel_blip_a::{TunelBlipA, Z_CRISTAL},
};
use std::{io::Write, path::Path};

fn guardar(fb: &Framebuffer, ruta: &Path) {
    let mut f = std::io::BufWriter::new(std::fs::File::create(ruta).unwrap());
    write!(f, "P6\n{} {}\n255\n", fb.width, fb.height).unwrap();
    for p in fb.pixels.chunks_exact(4) {
        f.write_all(&p[..3]).unwrap();
    }
}
fn main() {
    let carpeta = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/tunel-blip-a".into());
    std::fs::create_dir_all(&carpeta).unwrap();
    let escena = TunelBlipA::nuevo();
    let mut fb = Framebuffer::new(1200, 760);
    let rocky = Rocky::nuevo(
        std::sync::Arc::new(vec![Luz {
            pos: v3(0.0, 3.8, -26.0),
            color: v3(1.0, 0.83, 0.63),
            inten: 1.3,
            radio: 9.0,
        }]),
        v3(0.27, 0.29, 0.34),
    );
    let estado = EncuentroRocky::default();
    for (nombre, pos, obj) in [
        (
            "tunel_entrada",
            v3(0.0, 1.72, -1.0),
            v3(0.0, 2.45, Z_CRISTAL),
        ),
        (
            "tunel_cristal",
            v3(0.7, 1.8, -17.0),
            v3(0.0, 2.60, Z_CRISTAL),
        ),
        ("rocky_cristal", v3(0.0, 1.79, -22.5), v3(0.0, 1.25, -28.0)),
        ("tunel_pared", v3(-0.6, 1.8, -6.0), v3(2.0, 2.5, -13.0)),
    ] {
        let mut cam = TunelBlipA::camara(fb.width, fb.height);
        cam.pos = pos;
        cam.mirar_a(obj);
        let inicio = std::time::Instant::now();
        escena.dibujar_con_items(&mut fb, &cam, 0.0, rocky.items(&estado, 0.0));
        println!(
            "{nombre}: {:.1} ms",
            inicio.elapsed().as_secs_f32() * 1000.0
        );
        guardar(&fb, &Path::new(&carpeta).join(format!("{nombre}.ppm")));
    }
}
