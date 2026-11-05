// Fondo de Marte generado en CPU: cielo, terreno con ruido, normales y niebla.
// Reutiliza recorte, proyección, interpolación y z-buffer del motor compartido.
use crate::camara::Camara;
use crate::dibujo::{Item, Mezcla};
use crate::malla::Malla;
use crate::math::*;
use crate::nave::{Nave, PoseNave};
use crate::dibujo::Stats;
use crate::raster::{preparar_item, rasterizar_banda, Framebuffer, Vista};
use crate::ruido::{fbm, ruido};
use crate::shader::{Contexto, Fragmento, Shader};

fn altura(x: f32, z: f32) -> f32 {
    let lejania = smoothstep(12.0, 100.0, -z);
    let monte = 31.0 * (-((x + 59.0) / 39.0).powi(2) - ((z + 125.0) / 48.0).powi(2)).exp();
    let cordillera = 13.0 * (-((x - 98.0) / 65.0).powi(2) - ((z + 154.0) / 65.0).powi(2)).exp();
    let dunas = (x * 0.055 + z * 0.17).sin() * 0.5;
    monte + cordillera + dunas + fbm(v3(x * 0.05, 2.0, z * 0.05), 4, 2026) * (0.7 + lejania * 4.0)
}

struct ShaderSuelo;

fn camara(ancho: usize, alto: usize) -> Camara {
    let mut cam = Camara::nueva(ancho as f32, alto as f32);
    cam.pos = v3(0.0, 3.6, 9.0);
    cam.fov = 58f32.to_radians();
    cam.mirar_a(v3(0.0, 5.0, -100.0));
    cam
}

// Aproximación, descenso vertical y contacto. Solo cambia la pose visual;
// no se teletransporta ni modifica el estado físico de la nave del jugador.
pub fn pose_aterrizaje(t: f32) -> PoseNave {
    let t = t.clamp(0.0, 1.0);
    let avance = smoothstep(0.0, 0.72, t);
    let descenso = smoothstep(0.12, 0.86, t);
    // Mantener la zona de contacto delante de la cámara para que se vea
    // la nave completa, incluso cuando ya está apoyada en el suelo.
    let z = -14.0 + avance * 10.0;
    let x = -1.8 * (1.0 - avance);
    let pos = v3(x, altura(x, z) + 0.27 + (1.0 - descenso) * 7.0, z);
    let inclinacion = (1.0 - descenso) * -0.16;
    PoseNave { pos, fwd: v3(0.0, inclinacion, -1.0).norm(),
        up: v3(0.0, 1.0, inclinacion).norm(), empuje: 1.1 * (1.0 - smoothstep(0.72, 0.94, t)) }
}

pub fn dibujar_aterrizaje(fondo: &Framebuffer, nave: &Nave, t: f32) -> Framebuffer {
    let mut fb = Framebuffer::new(fondo.width, fondo.height);
    fb.pixels.clone_from(&fondo.pixels);
    fb.zbuf.clone_from(&fondo.zbuf);
    let cam = camara(fb.width, fb.height);
    let mut items = vec![];
    nave.emitir_pose(&cam, pose_aterrizaje(t), &mut items, &mut Stats::default());
    let mut tris = vec![];
    let vista = Vista::de(&cam, fb.width, fb.height);
    for (id, item) in items.iter().enumerate() { preparar_item(item, id as u32, &vista, &mut tris); }
    let ctx = Contexto { cam: cam.pos, sol: v3(100.0, 80.0, -170.0), tiempo: t * 5.0 };
    rasterizar_banda(0, &mut fb.pixels, &mut fb.zbuf, fondo.width, &tris, &items, &ctx);
    fb
}
impl Shader for ShaderSuelo {
    fn shade(&self, f: &Fragmento, c: &Contexto) -> [f32; 4] {
        let luz = 0.32 + 0.68 * f.normal.norm().dot((c.sol - f.pos).norm()).max(0.0);
        let n = ruido(f.pos * 0.9, 504) * 0.5 + 0.5;
        let surcos = (f.pos.z * 3.5 + f.pos.x * 0.6 + n * 1.5).sin() * 0.04;
        let col = mezcla(rgb(117.0, 40.0, 23.0), rgb(195.0, 82.0, 40.0), n * 0.5 + 0.25 + surcos) * luz;
        let niebla = 1.0 - (-((f.pos - c.cam).len() / 195.0).powi(2)).exp();
        let col = mezcla(col, rgb(199.0, 95.0, 57.0), niebla * 0.85);
        [col.x, col.y, col.z, 1.0]
    }
}

pub fn dibujar_paisaje(ancho: usize, alto: usize) -> Framebuffer {
    let mut fb = Framebuffer::new(ancho, alto);
    for y in 0..alto {
        for x in 0..ancho {
            let u = x as f32 / ancho as f32;
            let v = y as f32 / alto as f32;
            let dist = ((u - 0.81).powi(2) + ((v - 0.24) * alto as f32 / ancho as f32).powi(2)).sqrt();
            let base = mezcla(rgb(124.0, 43.0, 30.0), rgb(229.0, 132.0, 79.0), (v * 1.5).min(1.0));
            let halo = (-dist * 10.0).exp() * 0.65;
            let mut col = mezcla(base, rgb(255.0, 214.0, 142.0), halo);
            if dist < 0.004 { col = rgb(255.0, 246.0, 199.0); }
            let offset = (y * ancho + x) * 4;
            fb.pixels[offset..offset + 4].copy_from_slice(&[
                (col.x * 255.0) as u8, (col.y * 255.0) as u8, (col.z * 255.0) as u8, 255]);
        }
    }
    let (nx, nz) = (160usize, 120usize);
    let mut m = Malla { pos: vec![], nrm: vec![], uv: vec![], idx: vec![], radio: 400.0 };
    for j in 0..=nz {
        let z = 14.0 - j as f32 / nz as f32 * 270.0;
        for i in 0..=nx {
            let x = -200.0 + i as f32 / nx as f32 * 400.0;
            let h = altura(x, z);
            let dx = (altura(x + 0.3, z) - altura(x - 0.3, z)) / 0.6;
            let dz = (altura(x, z + 0.3) - altura(x, z - 0.3)) / 0.6;
            m.pos.push(v3(x, h, z));
            m.nrm.push(v3(-dx, 1.0, -dz).norm());
            m.uv.push([i as f32 / nx as f32, j as f32 / nz as f32]);
        }
    }
    for j in 0..nz {
        for i in 0..nx {
            let a = (j * (nx + 1) + i) as u32;
            let b = a + (nx + 1) as u32;
            m.idx.extend([[a, a + 1, b], [a + 1, b + 1, b]]);
        }
    }
    let cam = camara(ancho, alto);
    let shader = ShaderSuelo;
    let item = Item { malla: &m, shader: &shader, pos: V3::ZERO, rot: Rot::identidad(),
        escala: v3(1.0, 1.0, 1.0), mezcla: Mezcla::Opaco, doble_cara: false,
        tinte: v3(1.0, 1.0, 1.0), mip: 0.0, dist: 1.0 };
    let mut tris = vec![];
    preparar_item(&item, 0, &Vista::de(&cam, ancho, alto), &mut tris);
    let ctx = Contexto { cam: cam.pos, sol: v3(100.0, 80.0, -170.0), tiempo: 0.0 };
    rasterizar_banda(0, &mut fb.pixels, &mut fb.zbuf, ancho, &tris, &[item], &ctx);
    fb
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paisaje_tiene_terreno_y_cielo_y_es_reproducible() {
        let a = dibujar_paisaje(160, 100);
        let b = dibujar_paisaje(160, 100);
        assert_eq!(a.pixels, b.pixels);
        let terreno = a.zbuf.iter().filter(|z| **z > 0.0).count();
        assert!(terreno > 3000 && terreno < 14000, "terreno: {terreno}");
    }

    #[test]
    fn nave_desciende_y_se_renderiza_sin_modificar_el_vuelo() {
        let fondo = dibujar_paisaje(320, 200);
        let nave = Nave::predeterminada().unwrap();
        let pos_original = nave.pos;
        let inicio = pose_aterrizaje(0.0);
        let fin = pose_aterrizaje(1.0);
        assert!(inicio.pos.y > fin.pos.y + 5.0);
        assert!((fin.pos.y - altura(fin.pos.x, fin.pos.z) - 0.27).abs() < 1e-4);
        for t in [0.35, 0.7, 1.0] {
            let fb = dibujar_aterrizaje(&fondo, &nave, t);
            assert!(fb.pixels.chunks_exact(4).zip(fondo.pixels.chunks_exact(4))
                .filter(|(a, b)| a != b).count() > 30, "nave invisible: {t}");
        }
        assert_eq!(nave.pos, pos_original);
    }
}
