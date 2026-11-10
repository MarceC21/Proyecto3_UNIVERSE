// Operación Taumoeba: reglas y simulación sin ventana. La nave temporal usa
// Nave::actualizar; no se implementa otro integrador de vuelo.
use crate::{
    math::*,
    nave::{Control, Mundo, Nave},
    ruido::Rng,
};

pub const DURACION_PETROVA: f64 = 5.0;
pub const SEMIANCHO: f32 = 7.5;
pub const SEMIALTO: f32 = 4.8;

#[derive(Clone, Copy, Debug)]
pub struct Configuracion {
    pub muestras: u8,
    pub duracion: f64,
    pub intervalo_min: f64,
    pub intervalo_max: f64,
    pub velocidad: f32,
    pub distancia: f32,
    pub max_objetos: usize,
    pub probabilidad_muestra: f32,
}

impl Default for Configuracion {
    fn default() -> Self {
        Self {
            muestras: 8,
            duracion: 90.0,
            intervalo_min: 1.0,
            intervalo_max: 1.5,
            velocidad: 12.0,
            distancia: 45.0,
            max_objetos: 24,
            probabilidad_muestra: 0.6,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resultado {
    Victoria,
    Danos,
    Tiempo,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fase {
    Petrova,
    Instrucciones,
    Recoleccion,
    Extraccion,
    Final(Resultado),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    Muestra,
    Obstaculo,
}
#[derive(Clone, Debug)]
pub struct Objeto {
    pub id: u64,
    pub tipo: Tipo,
    /// Coordenadas en el marco de la misión; Z positivo significa DELANTE.
    pub pos: V3,
    pub radio: f32,
    pub variante: usize,
    pub fase: f32,
}

#[derive(Clone, Copy)]
pub struct Marco {
    pub fwd: V3,
    pub up: V3,
    pub derecha: V3,
}
impl Marco {
    pub fn nuevo(fwd: V3, up: V3) -> Self {
        let fwd = if fwd.len() > 0.1 {
            fwd.norm()
        } else {
            v3(0.0, 0.0, -1.0)
        };
        let referencia = if fwd.cross(up).len() > 0.1 {
            up
        } else if fwd.y.abs() < 0.9 {
            v3(0.0, 1.0, 0.0)
        } else {
            v3(0.0, 0.0, 1.0)
        };
        let derecha = fwd.cross(referencia).norm();
        Self {
            fwd,
            derecha,
            up: derecha.cross(fwd).norm(),
        }
    }
    pub fn mundo(self, p: V3) -> V3 {
        self.derecha * p.x + self.up * p.y + self.fwd * p.z
    }
    pub fn local(self, p: V3) -> V3 {
        v3(p.dot(self.derecha), p.dot(self.up), p.dot(self.fwd))
    }
}

pub struct MisionAdrian {
    pub fase: Fase,
    pub pausada: bool,
    pub intro: f64,
    pub tiempo: f64,
    pub visual: f64,
    pub muestras: u8,
    pub integridad: u8,
    pub invulnerable: f32,
    pub destello_verde: f32,
    pub destello_rojo: f32,
    pub objetos: Vec<Objeto>,
    pub nave: Nave,
    pub marco: Marco,
    pub extraccion: V3,
    pub config: Configuracion,
    rng: Rng,
    siguiente: f64,
    serial: u64,
    ultima_muestra: V3,
}
impl MisionAdrian {
    pub fn nueva(semilla: u64, fwd: V3, up: V3, config: Configuracion) -> Self {
        assert!(config.duracion.is_finite() && config.duracion > 0.0 && config.muestras > 0);
        assert!(
            config.intervalo_min > 0.0
                && config.intervalo_max >= config.intervalo_min
                && config.intervalo_max.is_finite()
        );
        assert!(
            config.velocidad.is_finite()
                && config.velocidad > 0.0
                && config.distancia.is_finite()
                && config.distancia >= 25.0
        );
        assert!(config.max_objetos > 0 && (0.0..=1.0).contains(&config.probabilidad_muestra));
        let marco = Marco::nuevo(fwd, up);
        let mut nave = Nave::fisica();
        nave.teletransportar(V3::ZERO, marco.fwd);
        nave.fwd = marco.fwd;
        nave.up = marco.up;
        Self {
            fase: Fase::Petrova,
            pausada: false,
            intro: 0.0,
            tiempo: 0.0,
            visual: 0.0,
            muestras: 0,
            integridad: 3,
            invulnerable: 0.0,
            destello_verde: 0.0,
            destello_rojo: 0.0,
            objetos: Vec::new(),
            nave,
            marco,
            extraccion: v3(0.0, 0.0, 24.0),
            config,
            rng: Rng::new(semilla),
            siguiente: 0.8,
            serial: 0,
            ultima_muestra: V3::ZERO,
        }
    }
    pub fn comenzar(&mut self) {
        if self.fase == Fase::Instrucciones {
            self.fase = Fase::Recoleccion;
        }
    }
    pub fn alternar_pausa(&mut self) {
        if matches!(
            self.fase,
            Fase::Petrova | Fase::Recoleccion | Fase::Extraccion
        ) {
            self.pausada = !self.pausada;
        }
    }
    pub fn restante(&self) -> f64 {
        (self.config.duracion - self.tiempo).max(0.0)
    }
    pub fn exitosa(&self) -> bool {
        self.fase == Fase::Final(Resultado::Victoria)
    }
    pub fn reiniciar(&mut self, semilla: u64) {
        *self = Self::nueva(semilla, self.marco.fwd, self.marco.up, self.config);
    }
    fn generar(&mut self) {
        if self.objetos.len() >= self.config.max_objetos {
            return;
        }
        let tipo = if self.serial < 2 || self.rng.f() < self.config.probabilidad_muestra {
            Tipo::Muestra
        } else {
            Tipo::Obstaculo
        };
        let mut p = if self.serial < 2 {
            V3::ZERO
        } else if tipo == Tipo::Muestra {
            // Paseo aleatorio: blancos variados, con trayectos posibles entre muestras.
            v3(
                (self.ultima_muestra.x + self.rng.rango(-4.0, 4.0)).clamp(-6.3, 6.3),
                (self.ultima_muestra.y + self.rng.rango(-2.5, 2.5)).clamp(-3.6, 3.6),
                0.0,
            )
        } else {
            v3(self.rng.rango(-6.3, 6.3), self.rng.rango(-3.6, 3.6), 0.0)
        };
        if tipo == Tipo::Obstaculo {
            // No colocar una roca encima de una muestra que está apareciendo.
            for _ in 0..8 {
                if !self.objetos.iter().any(|o| {
                    o.tipo == Tipo::Muestra
                        && o.pos.z > self.config.distancia - 20.0
                        && (v3(o.pos.x, o.pos.y, 0.0) - p).len() < 3.5
                }) {
                    break;
                }
                p = v3(self.rng.rango(-6.3, 6.3), self.rng.rango(-3.6, 3.6), 0.0);
            }
            if self.objetos.iter().any(|o| {
                o.tipo == Tipo::Muestra
                    && o.pos.z > self.config.distancia - 20.0
                    && (v3(o.pos.x, o.pos.y, 0.0) - p).len() < 3.5
            }) {
                return;
            }
        } else {
            self.ultima_muestra = p;
        }
        p.z = self.config.distancia;
        let radio = if tipo == Tipo::Muestra {
            0.9
        } else {
            self.rng.rango(0.95, 1.45)
        };
        self.objetos.push(Objeto {
            id: self.serial,
            tipo,
            pos: p,
            radio,
            variante: (self.rng.siguiente() % 4) as usize,
            fase: self.rng.rango(0.0, 6.28),
        });
        self.serial += 1;
    }
    /// Consume tiempo real mediante subpasos, incluso cuando baja el FPS.
    pub fn actualizar(&mut self, dt: f64, control: &Control) {
        if self.pausada
            || !dt.is_finite()
            || dt <= 0.0
            || matches!(self.fase, Fase::Final(_) | Fase::Instrucciones)
        {
            return;
        }
        if self.fase == Fase::Petrova {
            self.intro = (self.intro + dt).min(DURACION_PETROVA);
            self.visual = self.intro;
            if self.intro >= DURACION_PETROVA {
                self.fase = Fase::Instrucciones;
            }
            return;
        }
        let mut restante = dt;
        while restante > 0.0 && !matches!(self.fase, Fase::Final(_)) {
            let paso = restante.min(1.0 / 120.0);
            self.paso(paso, control);
            restante -= paso;
        }
    }
    fn paso(&mut self, dt: f64, control: &Control) {
        let ds = dt as f32;
        self.tiempo = (self.tiempo + dt).min(self.config.duracion);
        self.visual += dt;
        self.invulnerable = (self.invulnerable - ds).max(0.0);
        self.destello_rojo = (self.destello_rojo - ds).max(0.0);
        self.destello_verde = (self.destello_verde - ds).max(0.0);
        if self.tiempo >= self.config.duracion {
            self.fase = Fase::Final(Resultado::Tiempo);
            return;
        }

        let adaptado = Control {
            lateral: control.lateral * 0.24,
            vertical: control.vertical * 0.24,
            empuje: if self.fase == Fase::Extraccion {
                control.empuje * 0.22
            } else {
                0.0
            },
            ..Default::default()
        };
        let mundo = Mundo {
            cuerpos: &[],
            asteroides: &[],
            radio_limite: 100.0,
            alto_limite: 100.0,
            cinturon: None,
            gravedad: None,
        };
        self.nave.actualizar(&adaptado, ds, &mundo);
        let p = self.marco.local(self.nave.pos);
        let z = if self.fase == Fase::Extraccion {
            p.z.clamp(0.0, 36.0)
        } else {
            0.0
        };
        let limitado = v3(
            p.x.clamp(-SEMIANCHO, SEMIANCHO),
            p.y.clamp(-SEMIALTO, SEMIALTO),
            z,
        );
        let mut vel = self.marco.local(self.nave.vel);
        if p.x != limitado.x {
            vel.x = 0.0;
        }
        if p.y != limitado.y {
            vel.y = 0.0;
        }
        if self.fase == Fase::Recoleccion || p.z != limitado.z {
            vel.z = 0.0;
        }
        self.nave.pos = self.marco.mundo(limitado);
        self.nave.vel = self.marco.mundo(vel);

        if self.fase == Fase::Extraccion {
            if (limitado - self.extraccion).len() <= 3.2 {
                self.fase = Fase::Final(Resultado::Victoria);
            }
            return;
        }
        self.siguiente -= dt;
        if self.siguiente <= 0.0 {
            self.generar();
            self.siguiente += self.rng.rango(
                self.config.intervalo_min as f32,
                self.config.intervalo_max as f32,
            ) as f64;
        }
        let marco = self.marco;
        let nave = &self.nave;
        let mut vivos = Vec::with_capacity(self.objetos.len());
        for mut o in self.objetos.drain(..) {
            o.pos.z -= self.config.velocidad * ds;
            if o.pos.z < -7.0 {
                continue;
            }
            if nave.toca_esfera(marco.mundo(o.pos), o.radio) {
                match o.tipo {
                    Tipo::Muestra => {
                        self.muestras = self.muestras.saturating_add(1).min(self.config.muestras);
                        self.destello_verde = 0.35;
                    }
                    Tipo::Obstaculo if self.invulnerable <= 0.0 => {
                        self.integridad = self.integridad.saturating_sub(1);
                        self.invulnerable = 1.0;
                        self.destello_rojo = 0.3;
                    }
                    _ => {}
                }
            } else {
                vivos.push(o);
            }
        }
        self.objetos = vivos;
        if self.integridad == 0 {
            self.fase = Fase::Final(Resultado::Danos);
        } else if self.muestras >= self.config.muestras {
            self.fase = Fase::Extraccion;
            self.objetos.clear();
            self.nave.vel = V3::ZERO;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lista(config: Configuracion) -> MisionAdrian {
        let mut m = MisionAdrian::nueva(7, v3(0.0, 0.0, -1.0), v3(0.0, 1.0, 0.0), config);
        m.actualizar(5.0, &Control::default());
        m.comenzar();
        m.siguiente = 1000.0;
        m
    }
    fn contacto(m: &mut MisionAdrian, tipo: Tipo, id: u64) {
        m.objetos.push(Objeto {
            id,
            tipo,
            pos: V3::ZERO,
            radio: 0.9,
            variante: 0,
            fase: 0.0,
        });
    }
    #[test]
    fn introduccion_instrucciones_y_pausa_no_gastan_tiempo_de_mision() {
        let mut m = MisionAdrian::nueva(
            1,
            v3(1.0, 0.0, 0.0),
            v3(0.0, 1.0, 0.0),
            Configuracion::default(),
        );
        m.comenzar();
        assert_eq!(m.fase, Fase::Petrova);
        m.actualizar(2.0, &Control::default());
        assert_eq!(m.tiempo, 0.0);
        m.alternar_pausa();
        m.actualizar(100.0, &Control::default());
        assert_eq!(m.intro, 2.0);
        m.alternar_pausa();
        m.actualizar(4.0, &Control::default());
        assert_eq!(m.fase, Fase::Instrucciones);
        m.actualizar(100.0, &Control::default());
        assert_eq!(m.tiempo, 0.0);
        m.comenzar();
        m.actualizar(2.0, &Control::default());
        assert!((m.tiempo - 2.0).abs() < 1e-8);
        let objetos = m.objetos.iter().map(|o| (o.id, o.pos)).collect::<Vec<_>>();
        let pos = m.nave.pos;
        m.alternar_pausa();
        m.actualizar(
            100.0,
            &Control {
                lateral: 1.0,
                ..Default::default()
            },
        );
        assert_eq!(m.nave.pos, pos);
        assert_eq!(
            objetos,
            m.objetos.iter().map(|o| (o.id, o.pos)).collect::<Vec<_>>()
        );
        assert!((m.restante() - 88.0).abs() < 1e-8);
    }
    #[test]
    fn objetos_vienen_delante_incluso_con_ejes_rotados_y_se_reciclan() {
        let mut m = MisionAdrian::nueva(
            1,
            v3(1.0, 0.2, 0.4),
            v3(0.0, 1.0, 0.0),
            Configuracion::default(),
        );
        m.actualizar(5.0, &Control::default());
        m.comenzar();
        m.actualizar(0.9, &Control::default());
        let o = m.objetos[0].clone();
        assert!((m.marco.mundo(o.pos) - m.nave.pos).dot(m.nave.fwd) > 35.0);
        m.actualizar(0.5, &Control::default());
        assert!((m.objetos[0].pos.z - o.pos.z + 6.0).abs() < 0.001);
        m.siguiente = 1000.0;
        m.objetos[0].pos = v3(7.0, 4.5, -6.9);
        m.actualizar(0.1, &Control::default());
        assert!(m.objetos.is_empty());
    }
    #[test]
    fn impactos_usan_inmunidad_y_cada_muestra_se_recoge_una_vez() {
        let mut m = lista(Configuracion::default());
        contacto(&mut m, Tipo::Muestra, 1);
        m.actualizar(0.02, &Control::default());
        assert_eq!(m.muestras, 1);
        assert!(m.objetos.is_empty());
        m.actualizar(0.1, &Control::default());
        assert_eq!(m.muestras, 1);
        contacto(&mut m, Tipo::Obstaculo, 2);
        contacto(&mut m, Tipo::Obstaculo, 3);
        m.actualizar(0.02, &Control::default());
        assert_eq!(m.integridad, 2);
        assert!(m.objetos.is_empty());
        contacto(&mut m, Tipo::Obstaculo, 4);
        m.actualizar(0.02, &Control::default());
        assert_eq!(m.integridad, 2);
        for i in 0..2 {
            m.actualizar(1.1, &Control::default());
            contacto(&mut m, Tipo::Obstaculo, 5 + i);
            m.actualizar(0.02, &Control::default());
        }
        assert_eq!(m.fase, Fase::Final(Resultado::Danos));
        let t = m.tiempo;
        m.actualizar(20.0, &Control::default());
        assert_eq!(m.tiempo, t);
    }
    #[test]
    fn ocho_muestras_exigen_extraccion_y_el_reinicio_limpia_todo() {
        let mut m = lista(Configuracion::default());
        for i in 0..8 {
            contacto(&mut m, Tipo::Muestra, i);
            m.actualizar(0.02, &Control::default());
        }
        assert_eq!(m.fase, Fase::Extraccion);
        assert!(!m.exitosa());
        assert!(m.objetos.is_empty());
        for _ in 0..500 {
            m.actualizar(
                1.0 / 120.0,
                &Control {
                    empuje: 1.0,
                    ..Default::default()
                },
            );
            if m.exitosa() {
                break;
            }
        }
        assert!(m.exitosa());
        m.reiniciar(99);
        assert_eq!(m.fase, Fase::Petrova);
        assert_eq!(m.muestras, 0);
        assert_eq!(m.integridad, 3);
        assert_eq!(m.tiempo, 0.0);
        assert_eq!(m.nave.pos, V3::ZERO);
        assert_eq!(m.invulnerable, 0.0);
        assert!(m.objetos.is_empty());
        assert!(!m.pausada);
    }
    #[test]
    fn tiempo_agotado_tambien_pierde_si_falta_atravesar_el_anillo() {
        for extraccion in [false, true] {
            let mut m = lista(Configuracion {
                duracion: 1.0,
                ..Default::default()
            });
            if extraccion {
                m.fase = Fase::Extraccion;
                m.muestras = 8;
            }
            m.actualizar(2.0, &Control::default());
            assert_eq!(m.fase, Fase::Final(Resultado::Tiempo));
            assert_eq!(m.restante(), 0.0);
        }
    }
    #[test]
    fn movimiento_y_objetos_son_consistentes_entre_fps_y_no_salen_del_area() {
        let simular = |fps| {
            let mut m = lista(Configuracion::default());
            m.objetos.push(Objeto {
                id: 1,
                tipo: Tipo::Muestra,
                pos: v3(-6.0, -3.0, 45.0),
                radio: 0.9,
                variante: 0,
                fase: 0.0,
            });
            for _ in 0..fps * 2 {
                m.actualizar(
                    1.0 / fps as f64,
                    &Control {
                        lateral: 1.0,
                        vertical: 1.0,
                        ..Default::default()
                    },
                );
            }
            let p = m.marco.local(m.nave.pos);
            assert!(p.x <= SEMIANCHO && p.y <= SEMIALTO && p.z.abs() < 0.001);
            (p, m.objetos[0].pos, m.tiempo)
        };
        let a = simular(30);
        let b = simular(144);
        assert!((a.0 - b.0).len() < 0.04);
        assert!((a.1 - b.1).len() < 0.01);
        assert!((a.2 - b.2).abs() < 1e-8);
    }
    #[test]
    fn mision_se_puede_ganar_con_los_controles_reales_y_sin_perseguir_objetos() {
        for semilla in 0..3 {
            let mut m = MisionAdrian::nueva(
                semilla,
                v3(1.0, 0.0, 0.0),
                v3(0.0, 1.0, 0.0),
                Configuracion {
                    probabilidad_muestra: 1.0,
                    ..Default::default()
                },
            );
            m.actualizar(5.0, &Control::default());
            m.comenzar();
            for _ in 0..60 * 70 {
                let p = m.marco.local(m.nave.pos);
                let objetivo = if m.fase == Fase::Extraccion {
                    m.extraccion
                } else {
                    m.objetos
                        .iter()
                        .filter(|o| o.tipo == Tipo::Muestra && o.pos.z > 0.0)
                        .min_by(|a, b| a.pos.z.total_cmp(&b.pos.z))
                        .map(|o| o.pos)
                        .unwrap_or(p)
                };
                m.actualizar(
                    1.0 / 60.0,
                    &Control {
                        lateral: ((objetivo.x - p.x) * 0.7).clamp(-1.0, 1.0),
                        vertical: ((objetivo.y - p.y) * 0.7).clamp(-1.0, 1.0),
                        empuje: if m.fase == Fase::Extraccion { 1.0 } else { 0.0 },
                        ..Default::default()
                    },
                );
                if matches!(m.fase, Fase::Final(_)) {
                    break;
                }
            }
            assert!(
                m.exitosa(),
                "semilla {semilla}: {:?}, {} muestras",
                m.fase,
                m.muestras
            );
        }
    }
}
