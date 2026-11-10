// Renderiza vistas del sistema solar (y de la nave) a archivos .ppm, sin ventana.
// Uso: cargo run --release --no-default-features --bin exportar [carpeta] [ruta_nave.obj]
use nave_raster::{camara::Camara, cielo::Cielo, dibujo::Ajustes, escena::*, math::*,
                  nave::{Control, Mundo, Nave}, raster::Framebuffer, render::dibujar_con_nave as dibujar};
use nave_raster::seguimiento::Seguimiento;
use std::io::Write;
use std::time::Instant;

const ANCHO: usize = 960;
const ALTO: usize = 600;

fn guardar(fb: &Framebuffer, ruta: &str) {
    let mut f = std::fs::File::create(ruta).unwrap();
    write!(f, "P6\n{} {}\n255\n", ANCHO, ALTO).unwrap();
    for p in fb.pixels.chunks_exact(4) { f.write_all(&p[..3]).unwrap(); }
}

fn main() {
    let carpeta = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let ruta = std::env::args().nth(2);
    std::fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta de vistas");
    let t0 = Instant::now();
    let escena = Escena::nueva();
    let cielo = Cielo::nuevo();
    println!("Escena lista en {:.2}s", t0.elapsed().as_secs_f32());
    let mut fb = Framebuffer::new(ANCHO, ALTO);
    let aj = Ajustes::default();

    // Superficie del minijuego de Marte, con el mismo rasterizador en CPU.
    let paisaje = nave_raster::paisaje_marte::dibujar_paisaje(ANCHO, ALTO);
    guardar(&paisaje, &format!("{}/marte_superficie.ppm", carpeta));

    for (nombre, pos, objetivo) in escena.vistas() {
        let mut cam = Camara::nueva(ANCHO as f32, ALTO as f32);
        cam.pos = pos;
        cam.mirar_a(objetivo);
        let st = dibujar(&mut fb, &escena, &cielo, &cam, 3.0, &aj, None);
        println!("{:<9} objetos {}/{} | tris {} / {} | raster {}", nombre, st.objetos_enviados, st.objetos_total,
            st.tris_enviados, st.tris_sin_optimizar, st.tris_raster);
        guardar(&fb, &format!("{}/vista_{}.ppm", carpeta, nombre));
    }

    // ---- Vistas con la nave y cámara de persecución ----
    let mut nave = match ruta.as_deref().map(Nave::cargar).unwrap_or_else(Nave::predeterminada) {
        Ok(n) => n,
        Err(e) => { eprintln!("No se pudo cargar la nave: {e}"); std::process::exit(1); }
    };
    for (nombre, t) in [("marte_aproximacion", 0.25), ("marte_descenso", 0.65), ("marte_contacto", 1.0)] {
        let cuadro = nave_raster::paisaje_marte::dibujar_aterrizaje(&paisaje, &nave, t);
        guardar(&cuadro, &format!("{}/{}.ppm", carpeta, nombre));
    }
    let cuerpos = escena.esferas();
    let ast = escena.colisionadores_asteroides().to_vec();
    let mundo = Mundo { cuerpos: &cuerpos, asteroides: &ast, radio_limite: RADIO_LIMITE, alto_limite: ALTO_LIMITE, gravedad: None, cinturon: Some((CINTURON_INT, CINTURON_EXT)) };
    let tierra = escena.cuerpos.iter().find(|c| c.nombre == "Tierra").unwrap();
    let (inicio, objetivo_inicial) = nave_raster::guia::inicio_solar(&escena);
    let escenarios: Vec<(&str, V3, V3, f32)> = vec![
        ("nave_inicio", inicio, objetivo_inicial, 0.0),
        ("nave_tierra", tierra.pos + (escena.pos_sol() - tierra.pos).norm() * 16.0 + v3(6.0, 2.0, 0.0), tierra.pos, 0.0),
        ("nave_estacion", escena.estacion.as_ref().unwrap().pos + v3(0.0, 4.0, 12.0), escena.estacion.as_ref().unwrap().pos, 0.0),
        ("nave_neptuno", escena.cuerpos[8].pos + (escena.pos_sol() - escena.cuerpos[8].pos).norm() * 24.0, escena.cuerpos[8].pos, 0.0),
        ("nave_cinturon", v3(112.0, 3.0, 30.0), v3(100.0, 0.0, -40.0), 10.0),
        ("nave_turbo", v3(0.0, 8.0, 125.0), v3(0.0, 0.0, 0.0), 40.0),
    ];
    for (nombre, pos, objetivo, vel) in escenarios {
        nave.teletransportar(pos, objetivo);
        nave.vel = nave.fwd * vel;
        let mut cam = Camara::nueva(ANCHO as f32, ALTO as f32);
        let mut seg = Seguimiento::nuevo(nave.fwd, nave.up);
        for _ in 0..30 {
            nave.actualizar(&Control { empuje: if vel > 0.0 { 1.0 } else { 0.0 }, turbo: vel > 30.0, ..Default::default() }, 1.0 / 60.0, &mundo);
            seg.actualizar(&mut cam, nave.pos, nave.fwd, nave.up, nave.velocidad(), 1.0 / 60.0, None, &cuerpos);
        }
        let t = Instant::now();
        let st = dibujar(&mut fb, &escena, &cielo, &cam, 3.0, &aj, Some(&nave));
        println!("{:<13} {:>5.0} ms | tris {} | vel {:.1}", nombre, t.elapsed().as_secs_f32() * 1000.0, st.tris_enviados, nave.velocidad());
        guardar(&fb, &format!("{}/{}.ppm", carpeta, nombre));
    }
}
