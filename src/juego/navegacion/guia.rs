use crate::{
    camara::Camara,
    escena::Escena,
    math::*,
    progreso::{Galaxia, Progreso},
};

pub struct Objetivo {
    pub nombre: &'static str,
    pub indicacion: &'static str,
    pub pos: V3,
}

// Punto despejado sobre el vecindario de Tierra, orientado a la estación.
pub fn inicio_solar(escena: &Escena) -> (V3, V3) {
    let tierra = escena
        .cuerpos
        .iter()
        .find(|c| c.nombre == "Tierra")
        .expect("falta Tierra");
    let pos = tierra.pos + v3(0.0, 7.0, 16.0);
    (pos, escena.estacion.as_ref().map_or(tierra.pos, |e| e.pos))
}

pub fn objetivo(escena: &Escena, progreso: &Progreso, nave: V3) -> Option<Objetivo> {
    let cuerpo = |nombre| {
        escena
            .cuerpos
            .iter()
            .find(|c| c.nombre == nombre)
            .map(|c| c.pos)
    };
    if escena.galaxia == Galaxia::Solar {
        if !progreso.estacion_completada() {
            return escena.estacion.as_ref().map(|e| Objetivo {
                nombre: "ESTACION",
                indicacion: "Acercate despacio / E para atracar",
                pos: e.pos,
            });
        }
        if !progreso.marte_completado() {
            return cuerpo("Marte").map(|pos| Objetivo {
                nombre: "MARTE",
                indicacion: "Acercate despacio para aterrizar",
                pos,
            });
        }
    } else if escena.galaxia == Galaxia::Gargantua && !progreso.completada(Galaxia::Gargantua) {
        return cuerpo("Miller").map(|pos| Objetivo {
            nombre: "MILLER",
            indicacion: "Acercate despacio para aterrizar",
            pos,
        });
    }
    let radial = v3(nave.x, 0.0, nave.z);
    let dir = if radial.len() > 0.01 {
        radial.norm()
    } else {
        v3(1.0, 0.0, 0.0)
    };
    Some(Objetivo {
        nombre: "LIMITE / SALTO",
        indicacion: "Vuela hasta el borde / M para consultar la ruta",
        pos: dir * (escena.radio_limite - 3.0)
            + v3(
                0.0,
                nave.y
                    .clamp(-escena.alto_limite + 3.0, escena.alto_limite - 3.0),
                0.0,
            ),
    })
}

// Coordenadas normalizadas: dentro de pantalla el marcador sigue al destino;
// fuera de pantalla se fija al borde. Detrás indica hacia qué lado girar.
pub fn marcador(cam: &Camara, destino: V3) -> (f32, f32, bool) {
    let d = destino - cam.pos;
    let (f, r, u) = cam.base();
    let z = d.dot(f);
    let (fx, fy) = cam.factores();
    let mut x = d.dot(r) * fx;
    let mut y = -d.dot(u) * fy;
    if z > 0.1 {
        x /= z;
        y /= z;
    }
    if z <= 0.1 && x.abs() + y.abs() < 0.001 {
        x = 1.0;
    }
    let borde = z <= 0.1 || x.abs() > 0.78 || y.abs() > 0.48;
    if borde {
        let k = (x.abs() / 0.78).max(y.abs() / 0.48).max(0.001);
        x /= k;
        y /= k;
    }
    ((x + 1.0) * 0.5, (y + 1.0) * 0.5, z <= 0.1)
}

#[cfg(feature = "ventana")]
pub fn dibujar<D: raylib::prelude::RaylibDraw>(
    d: &mut D,
    cam: &Camara,
    objetivo: &Objetivo,
    nave: V3,
    rect: (f32, f32, f32, f32),
) {
    use raylib::prelude::*;
    let (ox, oy, w, h) = rect;
    let (x, y, detras) = marcador(cam, objetivo.pos);
    let borde = detras || x < 0.112 || x > 0.888 || y < 0.262 || y > 0.738;
    let (x, y) = (ox + x * w, oy + y * h);
    let c = Color::new(255, 219, 110, 255);
    // Flecha hacia el punto objetivo y un anillo pequeño para distinguirlo de etiquetas.
    let (dx, dy) = if borde {
        let (dx, dy) = (x - ox - w * 0.5, y - oy - h * 0.5);
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        (dx / len, dy / len)
    } else {
        (0.0, 1.0)
    };
    d.draw_triangle(
        Vector2::new(x, y),
        Vector2::new(x - dx * 24.0 + dy * 13.0, y - dy * 24.0 - dx * 13.0),
        Vector2::new(x - dx * 24.0 - dy * 13.0, y - dy * 24.0 + dx * 13.0),
        c,
    );
    d.draw_circle_lines(x as i32, y as i32, 5.0, c);
    let etiqueta = format!(
        "{}  {:.0}{}",
        objetivo.nombre,
        (objetivo.pos - nave).len(),
        if detras { " / DETRAS" } else { "" }
    );
    let ancho = (etiqueta.len() as f32 * 8.0 + 18.0).min(w - 20.0);
    let tx = (x - ancho * 0.5).clamp(ox + 10.0, ox + w - ancho - 10.0);
    d.draw_rectangle(
        tx as i32,
        (y + 12.0) as i32,
        ancho as i32,
        28,
        Color::new(4, 15, 24, 205),
    );
    d.draw_text(&etiqueta, (tx + 8.0) as i32, (y + 17.0) as i32, 16, c);
    d.draw_rectangle(
        (ox + w - 500.0).max(ox + 8.0) as i32,
        (oy + 12.0) as i32,
        488.min(w as i32 - 16),
        62,
        Color::new(4, 15, 24, 205),
    );
    let tx = (ox + w - 488.0).max(ox + 16.0) as i32;
    d.draw_text(
        &format!("OBJETIVO: {}", objetivo.nombre),
        tx,
        (oy + 21.0) as i32,
        19,
        c,
    );
    d.draw_text(
        objetivo.indicacion,
        tx,
        (oy + 49.0) as i32,
        15,
        Color::WHITE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inicio_despejado_cerca_de_tierra_y_guia_sigue_la_historia() {
        let e = Escena::nueva();
        let (pos, destino) = inicio_solar(&e);
        let tierra = e.cuerpos.iter().find(|c| c.nombre == "Tierra").unwrap();
        assert!((pos - tierra.pos).len() < 25.0);
        for (centro, radio) in e.esferas() {
            assert!((pos - centro).len() > radio + 2.0);
        }
        assert_eq!(destino, e.estacion.as_ref().unwrap().pos);
        let mut p = Progreso::default();
        assert_eq!(objetivo(&e, &p, pos).unwrap().nombre, "ESTACION");
        p.completar_estacion();
        assert_eq!(objetivo(&e, &p, pos).unwrap().nombre, "MARTE");
        p.completar_marte();
        let o = objetivo(&e, &p, pos).unwrap();
        assert_eq!(o.nombre, "LIMITE / SALTO");
        assert!(
            (e.radio_limite - (o.pos.x * o.pos.x + o.pos.z * o.pos.z).sqrt() - 3.0).abs() < 0.001
        );
        let g = Escena::gargantua();
        assert_eq!(objetivo(&g, &p, pos).unwrap().nombre, "MILLER");
        p.completar(Galaxia::Gargantua);
        assert_eq!(objetivo(&g, &p, pos).unwrap().nombre, "LIMITE / SALTO");
    }
    #[test]
    fn flecha_visible_delante_detras_y_fuera_del_encuadre() {
        let mut c = Camara::nueva(960.0, 600.0);
        c.pos = V3::ZERO;
        c.mirar_a(v3(0.0, 0.0, -1.0));
        for pos in [
            v3(0.0, 0.0, -10.0),
            v3(0.0, 0.0, 10.0),
            v3(-100.0, 30.0, -1.0),
        ] {
            let (x, y, detras) = marcador(&c, pos);
            assert!((0.1..=0.9).contains(&x) && (0.25..=0.75).contains(&y));
            assert_eq!(detras, pos.z > 0.0);
        }
    }
}
