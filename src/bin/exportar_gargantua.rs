// Vistas de revisión y tiempos del nuevo sistema, sin ventana ni GPU 3D.
use nave_raster::{camara::Camara, cielo::Cielo, dibujo::Ajustes, escena::Escena,
    math::*, nave::Nave, raster::Framebuffer, render::dibujar_con_nave, seguimiento::Seguimiento};
use std::{io::Write, time::Instant};

fn guardar(fb: &Framebuffer, ruta: &std::path::Path) {
    let mut f = std::io::BufWriter::new(std::fs::File::create(ruta).unwrap());
    write!(f, "P6\n{} {}\n255\n", fb.width, fb.height).unwrap();
    for p in fb.pixels.chunks_exact(4) { f.write_all(&p[..3]).unwrap(); }
}
fn main() {
    let carpeta = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "/tmp/gargantua".into()));
    std::fs::create_dir_all(&carpeta).unwrap();
    let t = Instant::now();
    let escena = Escena::gargantua();
    let cielo = Cielo::gargantua();
    println!("Gargantua lista en {:.2}s; {} cuerpos; {} asteroides", t.elapsed().as_secs_f32(), escena.cuerpos.len(), escena.colisionadores_asteroides().len());
    let mut fb = Framebuffer::new(960, 600);
    let mut nave = Nave::predeterminada().unwrap();
    let mut vistas = escena.vistas();
    let r = nave_raster::agujero_negro::RADIO_HORIZONTE;
    vistas.push(("agujero_cerca".into(), v3(0.0, r * 0.5, r * 2.5), V3::ZERO));
    vistas.push(("agujero_lateral".into(), v3(r * 3.3, r * 0.7, r * 2.0), V3::ZERO));
    let thalos = escena.cuerpos.iter().find(|c| c.nombre == "Thalos").unwrap();
    vistas.push(("doble_anillo".into(), thalos.pos + (escena.pos_sol() - thalos.pos).norm() * 57.0 + v3(0.0, 27.0, 0.0), thalos.pos));
    for (nombre, pos, objetivo) in vistas {
        let mut cam = Camara::nueva(960.0, 600.0);
        cam.pos = pos;
        cam.mirar_a(objetivo);
        let t = Instant::now();
        let st = dibujar_con_nave(&mut fb, &escena, &cielo, &cam, 2.0, &Ajustes::default(), None);
        println!("{nombre:<14} {:>6.1}ms | tris {} / {} | objetos {}", t.elapsed().as_secs_f32() * 1000.0, st.tris_enviados, st.tris_sin_optimizar, st.objetos_enviados);
        guardar(&fb, &carpeta.join(format!("{nombre}.ppm")));
    }
    let miller = &escena.cuerpos[1];
    nave.teletransportar(miller.pos + (escena.pos_sol() - miller.pos).norm() * 30.0 + v3(3.0, 3.0, 0.0), miller.pos);
    let mut cam = Camara::nueva(960.0, 600.0);
    Seguimiento::nuevo(nave.fwd, nave.up).actualizar(&mut cam, nave.pos, nave.fwd, nave.up, 0.0, 1.0 / 60.0, None, &escena.esferas());
    dibujar_con_nave(&mut fb, &escena, &cielo, &cam, 2.0, &Ajustes::default(), Some(&nave));
    guardar(&fb, &carpeta.join("nave_miller.ppm"));
}
