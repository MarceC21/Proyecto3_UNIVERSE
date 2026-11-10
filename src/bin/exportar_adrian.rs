// Revisión de la misión con el rasterizador de producción, sin ventana.
use nave_raster::{
    escena_adrian::EscenaAdrian,
    math::*,
    mision_adrian::{Configuracion, Fase, MisionAdrian, Objeto, Tipo},
    nave::{Control, Nave},
    raster::Framebuffer,
};
use std::{
    io::{BufWriter, Write},
    path::Path,
};
fn guardar(fb: &Framebuffer, ruta: &Path) {
    let mut f = BufWriter::new(std::fs::File::create(ruta).unwrap());
    write!(f, "P6\n{} {}\n255\n", fb.width, fb.height).unwrap();
    for p in fb.pixels.chunks_exact(4) {
        f.write_all(&p[..3]).unwrap();
    }
}
fn main() {
    let carpeta = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/adrian".into());
    std::fs::create_dir_all(&carpeta).unwrap();
    let escena = EscenaAdrian::nueva();
    let nave = Nave::predeterminada().unwrap();
    let mut fb = Framebuffer::new(960, 600);
    let mut m = MisionAdrian::nueva(
        42,
        v3(0.0, 0.0, -1.0),
        v3(0.0, 1.0, 0.0),
        Configuracion::default(),
    );
    for (nombre, t) in [
        ("petrova_inicio", 0.6),
        ("petrova_intensa", 2.5),
        ("petrova_salida", 4.8),
    ] {
        m.intro = t;
        m.visual = t;
        let inicio = std::time::Instant::now();
        escena.dibujar(&mut fb, &m, &nave);
        println!("Render: {:.1} ms", inicio.elapsed().as_secs_f32() * 1000.0);
        guardar(&fb, &Path::new(&carpeta).join(format!("{nombre}.ppm")));
    }
    m.actualizar(5.0, &Control::default());
    m.comenzar();
    m.objetos = vec![
        Objeto {
            id: 0,
            tipo: Tipo::Muestra,
            pos: v3(-2.5, 0.8, 8.0),
            radio: 0.9,
            variante: 0,
            fase: 0.0,
        },
        Objeto {
            id: 1,
            tipo: Tipo::Obstaculo,
            pos: v3(3.2, -0.4, 13.0),
            radio: 1.3,
            variante: 1,
            fase: 1.0,
        },
        Objeto {
            id: 2,
            tipo: Tipo::Muestra,
            pos: v3(1.0, 2.0, 22.0),
            radio: 0.9,
            variante: 0,
            fase: 2.0,
        },
        Objeto {
            id: 3,
            tipo: Tipo::Obstaculo,
            pos: v3(-4.0, 2.2, 29.0),
            radio: 1.2,
            variante: 3,
            fase: 0.5,
        },
    ];
    let inicio = std::time::Instant::now();
    escena.dibujar(&mut fb, &m, &nave);
    println!("Render: {:.1} ms", inicio.elapsed().as_secs_f32() * 1000.0);
    guardar(&fb, &Path::new(&carpeta).join("recoleccion.ppm"));
    // Detalle del aspecto biológico, sin añadir modelos externos.
    m.objetos = vec![Objeto {
        id: 4,
        tipo: Tipo::Muestra,
        pos: v3(0.0, 1.3, 1.0),
        radio: 1.8,
        variante: 0,
        fase: 0.0,
    }];
    let inicio = std::time::Instant::now();
    escena.dibujar(&mut fb, &m, &nave);
    println!("Render: {:.1} ms", inicio.elapsed().as_secs_f32() * 1000.0);
    guardar(&fb, &Path::new(&carpeta).join("taumoeba.ppm"));
    m.objetos.clear();
    m.muestras = 8;
    m.fase = Fase::Extraccion;
    let inicio = std::time::Instant::now();
    escena.dibujar(&mut fb, &m, &nave);
    println!("Render: {:.1} ms", inicio.elapsed().as_secs_f32() * 1000.0);
    guardar(&fb, &Path::new(&carpeta).join("extraccion.ppm"));
    println!("Vistas de Petrova, recoleccion, taumoeba y extraccion: {carpeta}");
}
