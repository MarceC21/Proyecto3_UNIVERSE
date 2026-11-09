// Renderiza vistas del interior (sin ventana) a .ppm para revisar el aspecto.
// Uso: cargo run --release --no-default-features --bin exportar_interior [carpeta]
use nave_raster::{
    camara::Camara, cielo::Cielo, dibujo::Ajustes, dibujo_interior::dibujar_items, escena::Escena,
    estacion_interior, jugador::Jugador, nave_interior, raster::Framebuffer, render::dibujar,
};
use std::io::Write;

const ANCHO: usize = 960;
const ALTO: usize = 600;

fn guardar(fb: &Framebuffer, ruta: &str) {
    let mut f = std::fs::File::create(ruta).unwrap();
    write!(f, "P6\n{} {}\n255\n", ANCHO, ALTO).unwrap();
    for p in fb.pixels.chunks_exact(4) {
        f.write_all(&p[..3]).unwrap();
    }
}

fn main() {
    let carpeta = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    std::fs::create_dir_all(&carpeta).expect("carpeta de vistas interiores");
    let (esc, _pts) = nave_interior::construir();
    let (est, _pe) = estacion_interior::construir();
    let escena = Escena::nueva();
    let cielo = Cielo::nuevo();
    let aj = Ajustes::default();
    let mut fb = Framebuffer::new(ANCHO, ALTO);
    let vistas = [
        ("c_frente", 0.0, 5.0, 0.0, -0.05),
        ("c_izq", -3.0, 3.0, -0.9, -0.1),
        ("c_der", 3.0, -2.0, 1.0, -0.05),
        ("c_mesa", 0.0, 4.0, 0.0, -0.45),
        ("c_puerta", 0.0, -5.0, 3.14159, 0.0),
        ("c_esclusa", 0.0, 11.0, 3.14159, 0.0),
    ];
    let vistas_e = [
        ("e_hub", 0.0, 3.0, 0.5, -0.1),
        ("e_brazo", 0.0, 4.0, 1.5708, 0.0),
        ("e_anillo", 17.0, 3.0, 3.6, 0.0),
        ("e_tubo", 21.0, 0.0, -1.5708, 0.0),
        ("e_ventana", 12.0, 12.0, -2.2, 0.0),
        ("e_brazo2", 3.0, 0.0, 3.1416, -0.1),
    ];
    let todas: Vec<(&str, f32, f32, f32, f32, bool)> = vistas
        .iter()
        .map(|v| (v.0, v.1, v.2, v.3, v.4, false))
        .chain(vistas_e.iter().map(|v| (v.0, v.1, v.2, v.3, v.4, true)))
        .collect();
    for (nombre, x, z, yaw, pitch, es_est) in todas {
        let esc = if es_est { &est } else { &esc };
        let mut j = Jugador::nuevo(x, z, yaw);
        j.pitch = pitch;
        let mut cam = Camara::nueva(ANCHO as f32, ALTO as f32);
        j.camara(&mut cam);
        let mut ext = Camara::nueva(ANCHO as f32, ALTO as f32);
        let tierra = escena
            .cuerpos
            .iter()
            .find(|c| c.nombre == "Tierra")
            .unwrap();
        ext.pos = tierra.pos + nave_raster::math::v3(14.0, 3.0, 8.0);
        ext.fov = cam.fov;
        ext.base_ext = Some(cam.base());
        dibujar(&mut fb, &escena, &cielo, &ext, 2.0, &aj);
        let items: Vec<_> = esc.partes.iter().map(|p| p.estatico()).collect();
        let t = std::time::Instant::now();
        let tris = dibujar_items(&mut fb, &items, &cam, 2.0);
        println!("{nombre}: {} ms, {} tris", t.elapsed().as_millis(), tris);
        guardar(&fb, &format!("{carpeta}/{nombre}.ppm"));
    }
    // La aventura real añade puertas móviles, holograma y el OBJ de TARS.
    let mut a = nave_raster::aventura::Aventura::nueva();
    let est = escena.estacion.as_ref().unwrap();
    a.iniciar_atraque(
        est.pos + nave_raster::math::v3(0.0, 3.0, 11.0),
        nave_raster::math::v3(0.0, 0.0, -1.0),
        est.pos,
        est.radio,
        est.rot,
    );
    for _ in 0..360 {
        a.actualizar(&Default::default(), 1.0 / 60.0);
    }
    for (nombre, x, z, yaw) in [
        ("mision_cabina", 0.0, 4.8, 0.0),
        ("mision_tars", 5.0, -3.0, 0.6),
    ] {
        a.jugador.colocar(x, z, yaw);
        a.dibujar(&mut fb, &escena, &cielo, &aj);
        guardar(&fb, &format!("{carpeta}/{nombre}.ppm"));
    }
}
