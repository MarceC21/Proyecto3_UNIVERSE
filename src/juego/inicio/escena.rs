//! Fondos del arranque usando las mismas mallas, shaders y z-buffer del juego.
use crate::{
    camara::Camara,
    cielo::Cielo,
    dibujo::{Ajustes, Mezcla, Stats},
    dibujo_interior::{cielo_fondo, dibujar_items, dibujar_items_con_sol, Escenario},
    escena::Escena,
    inicio::{Inicio, Pantalla},
    intro::Plano,
    math::*,
    nave::Nave,
    nave_interior,
    personaje::Personaje,
    raster::Framebuffer,
    shader::{Contexto, Fragmento, Shader},
    tars::Tars,
};
pub struct EscenaInicio {
    cabina: Escenario,
    personaje: Personaje,
    tars: Tars,
}
struct Atenuado<'a> {
    original: &'a dyn Shader,
    luz: f32,
}
impl Shader for Atenuado<'_> {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let p = self.original.shade(f, c);
        [p[0] * self.luz, p[1] * self.luz, p[2] * self.luz, p[3]]
    }
}
impl EscenaInicio {
    pub fn nueva() -> Self {
        let (cabina, _) = nave_interior::construir();
        let personaje = Personaje::nuevo(cabina.luces.clone(), cabina.ambiente);
        let mut tars = Tars::nuevo(cabina.luces.clone(), cabina.ambiente);
        tars.origen = v3(3.2, 0.0, -12.8);
        tars.angulo = -0.5;
        Self {
            cabina,
            personaje,
            tars,
        }
    }
    pub fn dibujar(
        &self,
        fb: &mut Framebuffer,
        inicio: &Inicio,
        escena: &Escena,
        cielo: &Cielo,
        nave: &Nave,
    ) {
        let tierra = escena
            .cuerpos
            .iter()
            .find(|c| c.nombre == "Tierra")
            .expect("inicio en el sistema solar");
        let mut cam = Camara::nueva(fb.width as f32, fb.height as f32);
        let aj = Ajustes {
            orbitas: false,
            limite: false,
            ..Default::default()
        };
        match inicio.pantalla() {
            Pantalla::Menu => {
                let (pos, obj) = inicio.menu.camara(tierra.pos, tierra.radio);
                cam.pos = pos;
                cam.mirar_a(obj);
                crate::render::dibujar(fb, escena, cielo, &cam, inicio.menu.tiempo(), &aj);
            }
            Pantalla::Intro => match inicio.intro.plano() {
                Plano::Tierra => {
                    let (pos, obj) = inicio.intro.camara_tierra(tierra.pos, tierra.radio);
                    cam.pos = pos;
                    cam.mirar_a(obj);
                    cielo_fondo(fb, cielo, &cam, inicio.intro.t());
                    let mut items = vec![];
                    escena.emitir(&cam, &aj, &mut items, &mut Stats::default());
                    // Atenuación temporal: no cambia los materiales ni la iluminación de la partida.
                    let shaders: Vec<_> = items
                        .iter()
                        .map(|i| Atenuado {
                            original: i.shader,
                            luz: 1.0 - inicio.intro.oscuridad_sol(),
                        })
                        .collect();
                    for (item, shader) in items.iter_mut().zip(&shaders) {
                        item.shader = shader;
                    }
                    let (mut opacos, mut alpha): (Vec<_>, Vec<_>) =
                        items.into_iter().partition(|i| i.mezcla == Mezcla::Opaco);
                    alpha.sort_by(|a, b| b.dist.total_cmp(&a.dist));
                    opacos.extend(alpha);
                    dibujar_items_con_sol(fb, &opacos, &cam, inicio.intro.t(), escena.pos_sol());
                }
                Plano::Cabina => {
                    let (pos, obj) = inicio.intro.camara_cabina();
                    cam.pos = pos;
                    cam.mirar_a(obj);
                    let mut ext = Camara::nueva(fb.width as f32, fb.height as f32);
                    ext.pos = nave.pos;
                    let (f, r, u) = cam.base();
                    let der = nave.fwd.cross(nave.up).norm();
                    let arriba = der.cross(nave.fwd).norm();
                    let m = |v: V3| der * v.x + arriba * v.y + nave.fwd * (-v.z);
                    ext.base_ext = Some((m(f), m(r), m(u)));
                    ext.fov = cam.fov;
                    crate::render::dibujar(fb, escena, cielo, &ext, inicio.intro.t(), &aj);
                    let mut items: Vec<_> =
                        self.cabina.partes.iter().map(|p| p.estatico()).collect();
                    items.extend(self.personaje.items(&inicio.intro));
                    items.extend(self.tars.items());
                    dibujar_items(fb, &items, &cam, inicio.intro.t());
                }
                Plano::Titulo => {
                    for p in fb.pixels.chunks_exact_mut(4) {
                        p.copy_from_slice(&[2, 5, 12, 255]);
                    }
                    fb.zbuf.fill(0.0);
                }
            },
            _ => {
                fb.pixels.fill(0);
                fb.zbuf.fill(0.0);
            }
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn intro_renderiza_personaje_sin_cambiar_la_nave_ni_el_sistema() {
        let escena = Escena::nueva();
        let cielo = Cielo::nuevo();
        let fondos = EscenaInicio::nueva();
        let mut nave = Nave::predeterminada().unwrap();
        let (pos, obj) = crate::guia::inicio_solar(&escena);
        nave.teletransportar(pos, obj);
        let antes = (nave.pos, nave.fwd, nave.up, nave.vel, nave.integridad);
        let angulos: Vec<_> = escena.cuerpos.iter().map(|c| c.angulo).collect();
        let mut inicio = Inicio::nuevo();
        let mut fb = Framebuffer::new(240, 150);
        fondos.dibujar(&mut fb, &inicio, &escena, &cielo, &nave);
        let menu = fb.pixels.clone();
        inicio.orden(crate::inicio::Orden::Aceptar);
        inicio.actualizar(8.0);
        fondos.dibujar(&mut fb, &inicio, &escena, &cielo, &nave);
        assert_ne!(menu, fb.pixels);
        assert!(
            fb.pixels
                .chunks_exact(4)
                .filter(|p| p[0] > 70 && p[1] > 50 && p[0] as u16 > p[2] as u16 * 2)
                .count()
                > 15,
            "no aparece el impermeable"
        );
        let cuerpo = fb.pixels.clone();
        inicio.actualizar(5.0);
        fondos.dibujar(&mut fb, &inicio, &escena, &cielo, &nave);
        assert_ne!(cuerpo, fb.pixels);
        assert_eq!(
            antes,
            (nave.pos, nave.fwd, nave.up, nave.vel, nave.integridad)
        );
        assert_eq!(
            angulos,
            escena.cuerpos.iter().map(|c| c.angulo).collect::<Vec<_>>()
        );
        inicio.orden(crate::inicio::Orden::Atras);
        assert_eq!(inicio.pantalla(), Pantalla::Juego);
        assert!(!crate::progreso::Progreso::default().estacion_completada());
    }
}
