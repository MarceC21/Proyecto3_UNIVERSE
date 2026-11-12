//! El mismo OBJ de la nave, Rocky y TARS, iluminados por el sol de la bahía.
use crate::{
    camara::Camara,
    dibujo::Stats,
    encuentro_rocky::{EncuentroRocky, Etapa},
    final_erid::*,
    materiales::Luz,
    math::*,
    nave::{Nave, PoseNave},
    paisaje_erid::{altura_suelo, EscenaEridani},
    raster::Framebuffer,
    rocky::Rocky,
    tars::Tars,
};
use std::sync::Arc;
pub struct PlayaErid {
    paisaje: EscenaEridani,
    rocky: Rocky,
    estado: EncuentroRocky,
    tars: Tars,
}

#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn descenso_y_personajes_usan_el_rasterizador_sin_mover_la_nave_espacial() {
        let escena = PlayaErid::nueva();
        let nave = Nave::predeterminada().unwrap();
        let antes = (nave.pos, nave.fwd, nave.up, nave.vel, nave.integridad);
        assert!(pose_aterrizaje(0.0).pos.y > pose_aterrizaje(1.0).pos.y + 25.0);
        assert_eq!(pose_aterrizaje(1.0).empuje, 0.0);
        let mut fb = Framebuffer::new(160, 100);
        escena.dibujar(
            &mut fb,
            &camara_descenso(160, 100, 0.0),
            &nave,
            0.0,
            Some(0.0),
        );
        let inicial = fb.pixels.clone();
        escena.dibujar(
            &mut fb,
            &camara_descenso(160, 100, 1.0),
            &nave,
            5.0,
            Some(1.0),
        );
        assert_ne!(inicial, fb.pixels);
        assert!(fb.zbuf.iter().any(|z| *z > 0.0));
        assert_eq!(
            antes,
            (nave.pos, nave.fwd, nave.up, nave.vel, nave.integridad)
        );
    }
}
pub fn pose_aterrizaje(t: f32) -> PoseNave {
    let t = t.clamp(0.0, 1.0);
    let s = smoothstep(0.0, 1.0, t);
    let y = altura_suelo(NAVE.0, NAVE.1) + 1.2;
    let inclinacion = -0.13 * (1.0 - s);
    PoseNave {
        pos: v3(
            NAVE.0 - 22.0 * (1.0 - s),
            y + 32.0 * (1.0 - s),
            NAVE.1 + 25.0 * (1.0 - s),
        ),
        fwd: v3(0.0, inclinacion, -1.0).norm(),
        up: v3(0.0, 1.0, inclinacion).norm(),
        empuje: 1.0 - smoothstep(0.70, 1.0, t),
    }
}
pub fn camara_descenso(w: usize, h: usize, t: f32) -> Camara {
    let mut cam = Camara::nueva(w as f32, h as f32);
    let s = smoothstep(0.0, 1.0, t);
    cam.pos = mezcla(v3(62.0, 27.0, 62.0), v3(39.0, 12.0, 47.0), s);
    cam.mirar_a(pose_aterrizaje(t).pos);
    cam
}
impl PlayaErid {
    pub fn nueva() -> Self {
        let luces = Arc::new(vec![Luz {
            pos: v3(-160.0, 150.0, -120.0),
            color: v3(1.0, 0.96, 0.85),
            inten: 1.5,
            radio: 600.0,
        }]);
        let ambiente = v3(0.56, 0.60, 0.65);
        let rocky = Rocky::nuevo(luces.clone(), ambiente);
        let mut estado = EncuentroRocky::default();
        estado.pos = v3(ROCKY.0, altura_suelo(ROCKY.0, ROCKY.1), ROCKY.1);
        estado.etapa = Etapa::Abordo;
        estado.angulo = 1.1;
        let mut tars = Tars::nuevo(luces, ambiente);
        tars.origen = v3(TARS.0, altura_suelo(TARS.0, TARS.1), TARS.1);
        tars.angulo = 1.0;
        Self {
            paisaje: EscenaEridani::nueva(),
            rocky,
            estado,
            tars,
        }
    }
    pub fn dibujar(
        &self,
        fb: &mut Framebuffer,
        cam: &Camara,
        nave: &Nave,
        tiempo: f32,
        descenso: Option<f32>,
    ) {
        let mut items = self.rocky.items(&self.estado, tiempo);
        items.extend(self.tars.items());
        self.componer(fb, cam, nave, tiempo, descenso, items);
    }
    pub fn dibujar_vivo(
        &self,
        fb: &mut Framebuffer,
        cam: &Camara,
        nave: &Nave,
        mision: &FinalErid,
    ) {
        let mut rocky = EncuentroRocky::default();
        rocky.pos = mision.paseo.rocky.pos;
        rocky.angulo = mision.paseo.rocky.angulo;
        rocky.paso = mision.paseo.rocky.paso;
        rocky.moviendo = mision.paseo.rocky.moviendo;
        let mut items = self.rocky.items(&rocky, mision.tiempo);
        let t = &mision.paseo.tars;
        items.extend(self.tars.items_pose(t.pos, t.angulo, t.paso, t.moviendo));
        self.componer(
            fb,
            cam,
            nave,
            mision.tiempo,
            (mision.fase == Fase::Descenso).then_some(mision.descenso),
            items,
        );
    }
    fn componer<'a>(
        &'a self,
        fb: &mut Framebuffer,
        cam: &Camara,
        nave: &'a Nave,
        tiempo: f32,
        descenso: Option<f32>,
        mut items: Vec<crate::dibujo::Item<'a>>,
    ) {
        let inicio = items.len();
        let pose = pose_aterrizaje(descenso.unwrap_or(1.0));
        nave.emitir_pose(cam, pose, &mut items, &mut Stats::default());
        for item in &mut items[inicio..] {
            item.pos = pose.pos + (item.pos - pose.pos) * 3.0;
            item.escala = item.escala * 3.0;
            item.dist = (item.pos - cam.pos).len();
        }
        self.paisaje.dibujar_con(fb, cam, tiempo, items);
    }
}
