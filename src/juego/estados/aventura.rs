// Máquina de estados de la aventura: atraque a la estación -> interior de la nave ->
// esclusa -> estación (recoger materiales) -> instalar en el Núcleo -> desacoplar.
// Todo el estado vive aquí; main.rs solo reenvía teclas y dibuja el HUD.
use crate::camara::Camara;
use crate::cielo::Cielo;
use crate::construccion::Geo;
use crate::dibujo::{Ajustes, Item};
use crate::dibujo_interior::*;
use crate::encuentro_rocky::{EncuentroRocky, Etapa, Linea, RADIO_ROCKY};
use crate::escena::Escena;
use crate::estacion_interior::{self, PuntosEstacion};
use crate::inventario::{Inventario, Material};
use crate::jugador::*;
use crate::malla;
use crate::materiales::*;
use crate::math::*;
use crate::nave_interior::{self, Puntos, FIN_ESCLUSA, FIN_TUNEL};
use crate::raster::Framebuffer;
use crate::render::dibujar;
use crate::rocky::Rocky;
use crate::tars::Tars;
use crate::tunel_blip_a::{TunelBlipA, Z_CRISTAL};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Fase {
    Inactiva,
    Atraque,
    Nave,
    Estacion,
    TunelBlip,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Evento {
    Ninguno,
    Desacoplar,
    VolverAlVuelo,
}

#[derive(Clone, Copy)]
enum Destino {
    NaveInicial,
    AEstacion,
    ANave,
    ATunelBlip,
}

#[derive(Clone, Copy, PartialEq)]
enum Muelle {
    Estacion,
    BlipA,
}

struct Fundido {
    t: f32,
    dur: f32,
    destino: Destino,
    hecho: bool,
}

struct Atraque {
    t: f32,
    dur: f32,
    desde: V3,
    hasta: V3,
    fwd_ini: V3,
    fwd_fin: V3,
}

#[derive(Default)]
pub struct EntradaAventura {
    pub j: EntradaJ,
    pub interactuar: bool, // true solo en el cuadro en que se pulsó la tecla
}

pub struct Hud {
    pub fase: Fase,
    pub dialogo: Option<Linea>,
    pub comentario: Option<Linea>,
    pub prompt: Option<String>,
    pub mensaje: Option<String>,
    pub objetivo: String,
    pub mochila: Vec<([u8; 3], &'static str)>,
    pub instalados: usize,
    pub transmision: Option<f32>,
    pub fundido: f32,
    pub jugador: (f32, f32, f32), // x, z, yaw (para el minimapa de la estación)
    pub pendientes: Vec<(f32, f32)>, // materiales aún por recoger (posición en la estación)
}

enum Accion {
    Transmitir,
    Hablar,
    Instalar,
    Pilotaje,
    Escotilla,
    SalirAEstacion,
    Recoger(usize),
    VolverANave,
}

struct Candidato {
    dist: f32,
    texto: String,
    accion: Option<Accion>,
}

pub struct Aventura {
    cabina: Escenario,
    pts: Puntos,
    estacion: Escenario,
    pe: PuntosEstacion,
    tars: Tars,
    hoja_puerta: Parte,
    hoja_escotilla: Parte,
    esfera_sol: Parte,
    esfera_planeta: Parte,
    octaedro: Parte,
    pub jugador: Jugador,
    pub inv: Inventario,
    pub fase: Fase,
    fundido: Option<Fundido>,
    atraque: Option<Atraque>,
    puerta: f32,
    escotilla: f32,
    escotilla_abierta: bool,
    recogido: [bool; 5],
    sistemas_activos: bool,
    mensaje: Option<(String, f32)>,
    hablando: f32,
    linea_tars: usize,
    tiempo: f32,
    nave_pos: V3,
    nave_fwd: V3,
    nave_up: V3,
    crucero: bool,
    est_pos: V3,
    est_rot: Rot,
    muelle: Muelle,
    tunel_blip: Option<TunelBlipA>,
    encuentro_blip: bool,
    rocky: Rocky,
    pub encuentro_rocky: EncuentroRocky,
    mundo_blip: Mundo,
    datos_disponibles: bool,
    datos_enviados: bool,
    transmision: Option<f32>,
}

fn parte_caja(s: V3, mat: Mat, luces: Arc<Vec<Luz>>, amb: V3) -> Parte {
    let mut g = Geo::vacio();
    g.caja(V3::ZERO, s);
    Parte {
        malla: g.a_malla(),
        shader: ShaderInterior::nuevo(mat, luces, amb),
        tinte: v3(1.0, 1.0, 1.0),
        doble: false,
    }
}

fn dxz(ax: f32, az: f32, b: V3) -> f32 {
    ((ax - b.x).powi(2) + (az - b.z).powi(2)).sqrt()
}

impl Aventura {
    pub fn nueva() -> Aventura {
        let (cabina, pts) = nave_interior::construir();
        let (estacion, pe) = estacion_interior::construir();
        let tars = Tars::nuevo(cabina.luces.clone(), cabina.ambiente);
        let (l, a) = (cabina.luces.clone(), cabina.ambiente);
        let mut luces_rocky = (*l).clone();
        luces_rocky.push(Luz {
            pos: v3(0.0, 3.8, -26.0),
            color: v3(1.0, 0.83, 0.63),
            inten: 1.3,
            radio: 9.0,
        });
        let rocky = Rocky::nuevo(Arc::new(luces_rocky), a);
        let vacias = Arc::new(vec![]);
        let esfera = |mat: Mat, luces: Arc<Vec<Luz>>| Parte {
            malla: malla::esfera(18, 12),
            shader: ShaderInterior::nuevo(mat, luces, a),
            tinte: v3(1.0, 1.0, 1.0),
            doble: false,
        };
        Aventura {
            hoja_puerta: parte_caja(v3(1.92, 4.5, 0.26), Mat::Compuerta, l.clone(), a),
            hoja_escotilla: parte_caja(v3(1.22, 2.6, 0.2), Mat::Compuerta, l.clone(), a),
            esfera_sol: esfera(Mat::Objeto, vacias.clone()),
            esfera_planeta: esfera(Mat::Pintura, l.clone()),
            octaedro: Parte {
                malla: malla::octaedro(),
                shader: ShaderInterior::nuevo(Mat::Objeto, vacias, a),
                tinte: v3(1.0, 1.0, 1.0),
                doble: false,
            },
            cabina,
            pts,
            estacion,
            pe,
            tars,
            jugador: Jugador::nuevo(0.0, 4.8, 0.0),
            inv: Inventario::nuevo(),
            fase: Fase::Inactiva,
            fundido: None,
            atraque: None,
            puerta: 0.0,
            escotilla: 0.0,
            escotilla_abierta: false,
            recogido: [false; 5],
            sistemas_activos: false,
            mensaje: None,
            hablando: 0.0,
            linea_tars: 0,
            tiempo: 0.0,
            nave_pos: V3::ZERO,
            nave_fwd: v3(0.0, 0.0, -1.0),
            nave_up: v3(0.0, 1.0, 0.0),
            crucero: false,
            est_pos: V3::ZERO,
            est_rot: Rot::identidad(),
            muelle: Muelle::Estacion,
            tunel_blip: None,
            encuentro_blip: false,
            rocky,
            encuentro_rocky: EncuentroRocky::default(),
            datos_disponibles: false,
            datos_enviados: false,
            transmision: None,
            mundo_blip: Mundo {
                zonas: vec![Zona::Rect {
                    x0: -1.24,
                    x1: 1.24,
                    z0: Z_CRISTAL + 0.35,
                    z1: -0.1,
                }],
                ..Default::default()
            },
        }
    }

    pub fn activa(&self) -> bool {
        self.fase != Fase::Inactiva
    }
    pub fn completada(&self) -> bool {
        self.sistemas_activos
    }
    pub fn en_crucero(&self) -> bool {
        self.crucero
    }
    pub fn en_blip_a(&self) -> bool {
        !self.crucero && self.muelle == Muelle::BlipA
    }
    pub fn encuentro_blip_realizado(&self) -> bool {
        self.encuentro_blip
    }

    pub fn sincronizar_datos(&mut self, disponibles: bool, enviados: bool) {
        self.datos_disponibles = disponibles;
        self.datos_enviados |= enviados;
    }
    pub fn datos_enviados(&self) -> bool {
        self.datos_enviados
    }
    pub fn interacciones_bloqueadas(&self) -> bool {
        self.encuentro_rocky.bloquea() || self.transmision.is_some()
    }

    fn restaurar_sistemas(&mut self) {
        if !self.sistemas_activos {
            self.inv.instalados = Material::TODOS.to_vec();
            self.inv.mochila.clear();
            self.recogido = [true; 5];
            self.sistemas_activos = true;
        }
    }

    pub fn entrar_cabina(&mut self, pos: V3, fwd: V3, up: V3, estacion_completada: bool) -> bool {
        if !estacion_completada || self.activa() {
            return false;
        }
        // Los accesos de inspección de Gargantua también tienen los sistemas activos.
        self.restaurar_sistemas();
        self.crucero = true;
        self.nave_pos = pos;
        self.nave_fwd = fwd;
        self.nave_up = up;
        self.atraque = None;
        self.fundido = None;
        self.escotilla_abierta = false;
        self.escotilla = 0.0;
        self.fase = Fase::Nave;
        self.jugador.colocar(0.0, 4.8, 0.0);
        self.jugador.altura_ojos = 1.65;
        self.mensaje("Cabina en crucero. H o consola de pilotaje para volver a conducir.");
        true
    }

    // Salida manual: conserva lo recogido para poder retomar el recorrido.
    pub fn suspender(&mut self) {
        self.fase = Fase::Inactiva;
        self.fundido = None;
        self.atraque = None;
    }

    pub fn puede_atracar(
        pos: V3,
        velocidad: f32,
        estacion: V3,
        radio: f32,
        en_vuelo: bool,
    ) -> bool {
        let distancia = (pos - estacion).len() - radio;
        en_vuelo && velocidad >= 0.0 && velocidad <= 6.0 && (0.0..=12.0).contains(&distancia)
    }

    // ---------------------------------------------------------------- atraque
    pub fn iniciar_atraque(
        &mut self,
        nave_pos: V3,
        nave_fwd: V3,
        est_pos: V3,
        est_radio: f32,
        est_rot: Rot,
    ) {
        let radial = nave_pos - est_pos;
        let dir = if radial.len() < 0.001 {
            v3(0.0, 0.0, 1.0)
        } else {
            radial.norm()
        };
        let hasta = est_pos + dir * (est_radio + 3.5);
        self.atraque = Some(Atraque {
            t: 0.0,
            dur: 4.0,
            desde: nave_pos,
            hasta,
            fwd_ini: nave_fwd.norm(),
            fwd_fin: -dir,
        });
        self.nave_pos = hasta;
        self.nave_fwd = -dir;
        self.nave_up = v3(0.0, 1.0, 0.0);
        self.crucero = false;
        self.muelle = Muelle::Estacion;
        self.est_pos = est_pos;
        self.est_rot = est_rot;
        self.fase = Fase::Atraque;
        self.fundido = None;
    }

    pub fn iniciar_atraque_blip(
        &mut self,
        nave_pos: V3,
        nave_fwd: V3,
        blip_pos: V3,
        radio: f32,
        rot: Rot,
    ) {
        self.restaurar_sistemas();
        self.iniciar_atraque(nave_pos, nave_fwd, blip_pos, radio, rot);
        self.muelle = Muelle::BlipA;
        self.escotilla_abierta = false;
        self.escotilla = 0.0;
        if self.tunel_blip.is_none() {
            self.tunel_blip = Some(TunelBlipA::nuevo());
        }
    }

    // Pose de la nave (posición, hacia dónde mira) durante la maniobra de atraque.
    pub fn pose_atraque(&self) -> Option<(V3, V3)> {
        let a = self.atraque.as_ref()?;
        let s = smoothstep(0.0, 1.0, a.t / a.dur);
        Some((
            a.desde.lerp(a.hasta, s),
            a.fwd_ini.lerp(a.fwd_fin, s).norm(),
        ))
    }

    // Dónde reaparece la nave al desacoplar: alejada de la estación y mirando hacia afuera.
    pub fn pose_desacople(&self) -> (V3, V3) {
        let dir = (self.nave_pos - self.est_pos).norm();
        let pos = self.nave_pos + dir * 10.0;
        (pos, pos + dir * 10.0)
    }

    pub fn fundido(&self) -> f32 {
        self.fundido.as_ref().map_or(0.0, |f| {
            (1.0 - (2.0 * f.t / f.dur - 1.0).abs()).clamp(0.0, 1.0)
        })
    }

    // ----------------------------------------------------------- interacciones
    fn candidato(&self) -> Option<Candidato> {
        if self.interacciones_bloqueadas() {
            return None;
        }
        let (jx, jz) = (self.jugador.x, self.jugador.z);
        let mut v: Vec<Candidato> = vec![];
        match self.fase {
            Fase::Nave => {
                let d = self.tars.distancia(jx, jz);
                if d < 3.0 {
                    v.push(Candidato {
                        dist: d - 0.8,
                        texto: "Hablar con TARS".into(),
                        accion: Some(Accion::Hablar),
                    });
                }
                let d = dxz(jx, jz, self.pts.nucleo);
                if d < 2.0 && self.datos_disponibles && !self.datos_enviados {
                    v.push(Candidato {
                        dist: d - 1.0,
                        texto: "Transmitir datos de las taumoebas a la estacion".into(),
                        accion: Some(Accion::Transmitir),
                    });
                }
                if d < 2.0 && !(self.datos_disponibles && !self.datos_enviados) {
                    let faltan = self.inv.faltan().len();
                    v.push(if !self.inv.mochila.is_empty() {
                        Candidato {
                            dist: d,
                            texto: format!(
                                "Instalar {} material(es) en el Núcleo",
                                self.inv.mochila.len()
                            ),
                            accion: Some(Accion::Instalar),
                        }
                    } else if faltan > 0 {
                        Candidato {
                            dist: d,
                            texto: format!("Núcleo de sistemas: faltan {faltan} materiales"),
                            accion: None,
                        }
                    } else {
                        Candidato {
                            dist: d,
                            texto: "Núcleo de sistemas: todo instalado".into(),
                            accion: None,
                        }
                    });
                }
                let d = dxz(jx, jz, self.pts.pilotaje);
                if d < 2.0 {
                    v.push(if self.sistemas_activos {
                        Candidato {
                            dist: d,
                            texto: if self.crucero {
                                "Volver al pilotaje"
                            } else {
                                "Desacoplar y partir"
                            }
                            .into(),
                            accion: Some(Accion::Pilotaje),
                        }
                    } else {
                        Candidato {
                            dist: d,
                            texto: "Sistemas apagados: instala los 5 materiales en el Núcleo"
                                .into(),
                            accion: None,
                        }
                    });
                }
                let d = dxz(jx, jz, self.pts.esclusa);
                if d < 1.8 {
                    if self.crucero {
                        v.push(Candidato {
                            dist: d,
                            texto: "Escotilla bloqueada: requiere atraque".into(),
                            accion: None,
                        });
                    } else {
                        let t = if self.escotilla_abierta {
                            "Cerrar escotilla"
                        } else {
                            "Abrir escotilla"
                        };
                        v.push(Candidato {
                            dist: d,
                            texto: t.into(),
                            accion: Some(Accion::Escotilla),
                        });
                    }
                }
                if !self.crucero && self.escotilla > 0.9 && jz > FIN_TUNEL - 1.8 && jz < FIN_TUNEL {
                    v.push(Candidato {
                        dist: 0.1,
                        texto: if self.en_blip_a() {
                            "Entrar al pasillo de Blip-A"
                        } else {
                            "Salir a la estación"
                        }
                        .into(),
                        accion: Some(Accion::SalirAEstacion),
                    });
                }
            }
            Fase::Estacion => {
                for (i, p) in self.pe.recogibles.iter().enumerate() {
                    if self.recogido[i] {
                        continue;
                    }
                    let d = dxz(jx, jz, *p);
                    if d < 1.9 {
                        v.push(Candidato {
                            dist: d,
                            texto: format!("Recoger {}", Material::TODOS[i].nombre()),
                            accion: Some(Accion::Recoger(i)),
                        });
                    }
                }
                let d = dxz(jx, jz, self.pe.nave);
                if d < 1.9 {
                    v.push(Candidato {
                        dist: d,
                        texto: "Volver a la nave".into(),
                        accion: Some(Accion::VolverANave),
                    });
                }
            }
            Fase::TunelBlip if jz > -2.2 => {
                v.push(Candidato {
                    dist: 0.0,
                    texto: if self.encuentro_rocky.etapa == Etapa::Guiando
                        && !self.encuentro_rocky.en_muelle()
                    {
                        "Espera a Rocky junto a la escotilla"
                    } else {
                        "Volver a la cabina"
                    }
                    .into(),
                    accion: if self.encuentro_rocky.etapa == Etapa::Guiando
                        && !self.encuentro_rocky.en_muelle()
                    {
                        None
                    } else {
                        Some(Accion::VolverANave)
                    },
                });
            }
            _ => {}
        }
        v.into_iter().min_by(|a, b| a.dist.total_cmp(&b.dist))
    }

    fn mensaje(&mut self, t: impl Into<String>) {
        self.mensaje = Some((t.into(), 4.5));
    }

    fn linea_tars(&mut self) -> String {
        let faltan = self.inv.faltan().len();
        let lineas: Vec<String> = if self.sistemas_activos {
            vec!["TARS: Sistemas al 100%. Honestidad al 90%, humor al 75%. Cuando quieras, a la consola de pilotaje.".into()]
        } else {
            vec![
                "TARS: Necesitamos cinco materiales de la estación para despertar los sistemas.".into(),
                "TARS: Celda de energía, cable de fibra, cristal de navegación, tanque de oxígeno y chip cuántico.".into(),
                format!("TARS: Faltan {faltan} por instalar. Están repartidos por el núcleo, los brazos y el anillo."),
                "TARS: Entrégalos en el Núcleo, en el muro de mando de la derecha.".into(),
            ]
        };
        let linea = lineas[self.linea_tars % lineas.len()].clone();
        self.linea_tars = (self.linea_tars + 1) % lineas.len();
        linea
    }

    fn ejecutar(&mut self, a: Accion) -> Evento {
        match a {
            Accion::Transmitir => {
                if self.datos_disponibles && !self.datos_enviados {
                    self.transmision = Some(0.0);
                    self.mensaje("Abriendo enlace con la estacion espacial...");
                }
            }
            Accion::Hablar => {
                self.hablando = 3.5;
                let l = self.linea_tars();
                self.mensaje(l);
            }
            Accion::Instalar => {
                let n = self.inv.instalar();
                let total = self.inv.instalados.len();
                if self.inv.sistemas_listos() {
                    self.sistemas_activos = true;
                    self.mensaje(
                        "¡Sistemas en línea! Ve a la consola de pilotaje para desacoplar.",
                    );
                } else {
                    self.mensaje(format!("Instalaste {n} material(es): {total}/5"));
                }
            }
            Accion::Pilotaje => {
                self.fase = Fase::Inactiva;
                return if self.crucero {
                    Evento::VolverAlVuelo
                } else {
                    Evento::Desacoplar
                };
            }
            Accion::Escotilla => {
                if self.encuentro_rocky.etapa == Etapa::Guiando && self.encuentro_rocky.en_nave {
                    return Evento::Ninguno;
                }
                self.escotilla_abierta = !self.escotilla_abierta;
            }
            Accion::SalirAEstacion => {
                self.fundido = Some(Fundido {
                    t: 0.0,
                    dur: 1.6,
                    destino: if self.en_blip_a() {
                        Destino::ATunelBlip
                    } else {
                        Destino::AEstacion
                    },
                    hecho: false,
                })
            }
            Accion::VolverANave => {
                self.fundido = Some(Fundido {
                    t: 0.0,
                    dur: 1.6,
                    destino: Destino::ANave,
                    hecho: false,
                })
            }
            Accion::Recoger(i) => match self.inv.recoger(Material::TODOS[i]) {
                Ok(()) => {
                    self.recogido[i] = true;
                    self.mensaje(format!("Recogiste: {}", Material::TODOS[i].nombre()));
                }
                Err(e) => self.mensaje(e),
            },
        }
        Evento::Ninguno
    }

    fn aplicar(&mut self, d: Destino) {
        self.jugador.altura_ojos = 1.65;
        match d {
            Destino::NaveInicial => {
                self.fase = Fase::Nave;
                self.jugador.colocar(0.0, 4.8, 0.0);
            }
            Destino::AEstacion => {
                self.fase = Fase::Estacion;
                let (x, z, y) = self.pe.llegada;
                self.jugador.colocar(x, z, y);
            }
            Destino::ANave => {
                self.encuentro_rocky.abordar(FIN_TUNEL);
                self.fase = Fase::Nave;
                self.jugador.colocar(0.0, FIN_TUNEL - 1.5, 0.0);
            }
            Destino::ATunelBlip => {
                self.fase = Fase::TunelBlip;
                self.jugador.colocar(0.0, -1.0, 0.0);
                self.jugador.altura_ojos = 1.79;
                self.mensaje("Recorre el pasillo hasta el cristal hexagonal.");
            }
        }
    }

    // ------------------------------------------------------------- actualización
    pub fn actualizar(&mut self, ent: &EntradaAventura, dt: f32) -> Evento {
        if self.fase == Fase::Inactiva || !dt.is_finite() || dt <= 0.0 {
            return Evento::Ninguno;
        }
        if let Some(t) = &mut self.transmision {
            *t += dt;
            if *t >= 4.0 {
                self.transmision = None;
                self.datos_enviados = true;
                self.mensaje("ESTACION: Datos recibidos. Protocolo contra astrofagos confirmado. Ruta a Erid habilitada.");
            }
            return Evento::Ninguno;
        }
        if self.encuentro_rocky.bloquea() {
            // Consumir E/Enter aquí evita que la última línea accione también una consola.
            if ent.interactuar {
                if let Some(m) = self.encuentro_rocky.avanzar() {
                    self.mensaje(m);
                }
                self.encuentro_blip = self.encuentro_rocky.completo();
            }
            return Evento::Ninguno;
        }
        self.tiempo += dt;
        self.hablando = (self.hablando - dt).max(0.0);
        if let Some((_, t)) = &mut self.mensaje {
            *t -= dt;
        }
        if matches!(self.mensaje, Some((_, t)) if t <= 0.0) {
            self.mensaje = None;
        }

        let entrada_bloqueada = EntradaAventura::default();
        let ent = if self.fundido.is_some() {
            &entrada_bloqueada
        } else {
            ent
        };
        let mut aplicar = None;
        if let Some(f) = &mut self.fundido {
            f.t += dt;
            if !f.hecho && f.t >= f.dur * 0.5 {
                f.hecho = true;
                aplicar = Some(f.destino);
            }
            if f.t >= f.dur {
                self.fundido = None;
            }
        }
        if let Some(d) = aplicar {
            self.aplicar(d);
        }

        match self.fase {
            Fase::Atraque => {
                if let Some(a) = &mut self.atraque {
                    a.t = (a.t + dt).min(a.dur);
                    if a.t >= a.dur - 1.0 && self.fundido.is_none() {
                        self.fundido = Some(Fundido {
                            t: 0.0,
                            dur: 2.0,
                            destino: Destino::NaveInicial,
                            hecho: false,
                        });
                    }
                }
            }
            Fase::Nave => {
                self.actualizar_nave(ent, dt);
            }
            Fase::Estacion => {
                self.jugador.actualizar(&ent.j, dt, &self.estacion.mundo);
            }
            Fase::TunelBlip => {
                self.mundo_blip.dinamicos = if !self.encuentro_rocky.en_nave {
                    vec![Obst::caja(
                        self.encuentro_rocky.pos.x,
                        self.encuentro_rocky.pos.z,
                        RADIO_ROCKY * 2.0,
                        RADIO_ROCKY * 2.0,
                    )]
                } else {
                    vec![]
                };
                self.jugador.actualizar(&ent.j, dt, &self.mundo_blip);
                self.mundo_blip.dinamicos.clear();
                self.encuentro_rocky.actualizar(
                    dt,
                    &self.mundo_blip,
                    self.jugador.x,
                    self.jugador.z,
                    false,
                );
                if self.encuentro_rocky.apertura >= 1.0 {
                    self.mundo_blip.zonas = vec![Zona::Rect {
                        x0: -1.24,
                        x1: 1.24,
                        z0: -31.0,
                        z1: -0.1,
                    }];
                }
            }
            Fase::Inactiva => {}
        }

        if ent.interactuar
            && !self.encuentro_rocky.bloquea()
            && self.fundido.is_none()
            && matches!(self.fase, Fase::Nave | Fase::Estacion | Fase::TunelBlip)
        {
            if let Some(Candidato {
                accion: Some(a), ..
            }) = self.candidato()
            {
                return self.ejecutar(a);
            }
        }
        Evento::Ninguno
    }

    fn actualizar_nave(&mut self, ent: &EntradaAventura, dt: f32) {
        let (jx, jz) = (self.jugador.x, self.jugador.z);
        let pc = self.pts.puerta_centro;
        let cerca = dxz(jx, jz, pc) < 4.5 || self.tars.distancia(pc.x, pc.z) < 3.0;
        let obj = if cerca { 1.0 } else { 0.0 };
        self.puerta += (obj - self.puerta).clamp(-dt / 1.2, dt / 1.2);
        let obj = if self.escotilla_abierta { 1.0 } else { 0.0 };
        self.escotilla += (obj - self.escotilla).clamp(-dt / 1.4, dt / 1.4);

        let mut din = vec![];
        if self.puerta < 0.85 {
            din.push(Obst {
                cx: pc.x,
                cz: pc.z,
                hx: 1.9,
                hz: 0.22,
                yaw: 0.0,
            });
        }
        if self.escotilla < 0.85 {
            din.push(Obst {
                cx: 0.0,
                cz: FIN_ESCLUSA - 0.1,
                hx: 1.3,
                hz: 0.25,
                yaw: 0.0,
            });
        }
        self.cabina.mundo.dinamicos = din;
        self.tars
            .actualizar(dt, &self.cabina.mundo, jx, jz, self.hablando > 0.0);
        self.cabina.mundo.dinamicos.push(self.tars.obstaculo());
        if self.encuentro_rocky.en_nave {
            self.encuentro_rocky
                .actualizar(dt, &self.cabina.mundo, jx, jz, true);
            self.cabina.mundo.dinamicos.push(Obst::caja(
                self.encuentro_rocky.pos.x,
                self.encuentro_rocky.pos.z,
                RADIO_ROCKY * 2.0,
                RADIO_ROCKY * 2.0,
            ));
        }
        self.jugador.actualizar(&ent.j, dt, &self.cabina.mundo);
    }

    // ------------------------------------------------------------------ dibujo
    pub fn dibujar(&self, fb: &mut Framebuffer, escena: &Escena, cielo: &Cielo, aj: &Ajustes) {
        if self.fase == Fase::TunelBlip {
            let mut cam = Camara::nueva(fb.width as f32, fb.height as f32);
            self.jugador.camara(&mut cam);
            self.tunel_blip
                .as_ref()
                .expect("tunel de atraque")
                .dibujar_encuentro(
                    fb,
                    &cam,
                    self.tiempo,
                    if !self.encuentro_rocky.en_nave {
                        self.rocky.items(&self.encuentro_rocky, self.tiempo)
                    } else {
                        vec![]
                    },
                    self.encuentro_rocky.apertura,
                );
            return;
        }
        if !matches!(self.fase, Fase::Nave | Fase::Estacion) {
            return;
        }
        let mut cam = Camara::nueva(fb.width as f32, fb.height as f32);
        self.jugador.camara(&mut cam);
        let (f, r, u) = cam.base();
        let mut ext = Camara::nueva(fb.width as f32, fb.height as f32);
        ext.fov = cam.fov;
        let t = self.tiempo;
        let mut items: Vec<Item> = vec![];
        if self.fase == Fase::Nave {
            let fwd = self.nave_fwd;
            let up0 = if self.nave_up.cross(fwd).len() > 0.001 {
                self.nave_up
            } else if fwd.y.abs() > 0.98 {
                v3(0.0, 0.0, -1.0)
            } else {
                v3(0.0, 1.0, 0.0)
            };
            let der = fwd.cross(up0).norm();
            let arriba = der.cross(fwd).norm();
            let m = |v: V3| der * v.x + arriba * v.y + fwd * (-v.z);
            ext.pos = self.nave_pos;
            ext.base_ext = Some((m(f), m(r), m(u)));
            dibujar(fb, escena, cielo, &ext, t, aj);

            items.extend(self.cabina.partes.iter().map(|p| p.estatico()));
            // puerta trasera y escotilla
            let rp = Rot::rot_y(self.pts.puerta_yaw);
            for s in [-1.0f32, 1.0] {
                let off = self.pts.puerta_dir * (s * (0.96 + self.puerta * 1.92));
                items.push(self.hoja_puerta.item(
                    self.pts.puerta_centro + off,
                    rp,
                    v3(1.0, 1.0, 1.0),
                ));
                let off = self.pts.escotilla_dir * (s * (0.61 + self.escotilla * 1.2));
                items.push(self.hoja_escotilla.item(
                    self.pts.escotilla_centro + off,
                    Rot::rot_y(self.pts.escotilla_yaw),
                    v3(1.0, 1.0, 1.0),
                ));
            }
            // planetario holográfico sobre la mesa
            let c = self.pts.planetario;
            items.push(self.esfera_sol.item_con(
                c,
                Rot::identidad(),
                v3(0.26, 0.26, 0.26),
                v3(1.0, 0.72, 0.25),
            ));
            for (radio, vel, tam, col, fase) in [
                (0.55f32, 1.6f32, 0.055f32, v3(0.62, 0.6, 0.58), 0.0f32),
                (0.8, 1.1, 0.09, v3(0.9, 0.72, 0.4), 1.7),
                (1.1, 0.8, 0.10, v3(0.25, 0.5, 0.95), 3.1),
                (1.5, 0.55, 0.07, v3(0.9, 0.35, 0.2), 4.4),
            ] {
                let a = t * vel * 0.6 + fase;
                items.push(self.esfera_planeta.item_con(
                    v3(c.x + radio * a.cos(), c.y - 0.3, c.z + radio * a.sin()),
                    Rot::identidad(),
                    v3(tam, tam, tam),
                    col,
                ));
            }
            items.extend(self.tars.items());
            if self.encuentro_rocky.en_nave {
                items.extend(self.rocky.items(&self.encuentro_rocky, self.tiempo));
            }
        } else {
            let r_est = self.est_rot;
            ext.base_ext = Some((r_est.aplicar(f), r_est.aplicar(r), r_est.aplicar(u)));
            cielo_fondo(fb, cielo, &ext, t);
            items.extend(self.estacion.partes.iter().map(|p| p.estatico()));
            for (i, p) in self.pe.recogibles.iter().enumerate() {
                if self.recogido[i] {
                    continue;
                }
                let pos = *p + v3(0.0, 0.06 * (t * 2.0 + i as f32).sin(), 0.0);
                items.push(self.octaedro.item_con(
                    pos,
                    Rot::rot_y(t * 1.3 + i as f32).mul(&Rot::rot_x(0.4)),
                    v3(0.2, 0.28, 0.2),
                    Material::TODOS[i].color(),
                ));
            }
        }
        dibujar_items(fb, &items, &cam, t);
    }

    // --------------------------------------------------------------------- HUD
    pub fn hud(&self) -> Hud {
        let objetivo = if self.transmision.is_some() {
            "Transmitiendo muestras y protocolo. Espera la confirmacion de la estacion.".to_string()
        } else if self.datos_disponibles && !self.datos_enviados && self.fase == Fase::Nave {
            "Envia los datos: activa con E la consola del Nucleo, junto al muro de mando derecho."
                .to_string()
        } else if self.encuentro_rocky.bloquea() {
            "Habla con Rocky: E / Enter para continuar".to_string()
        } else if self.encuentro_rocky.etapa == Etapa::Guiando
            && (self.fase == Fase::TunelBlip || self.encuentro_rocky.en_nave)
        {
            if self.encuentro_rocky.esperando {
                "Rocky te espera: acercate para continuar"
            } else if self.encuentro_rocky.en_muelle() {
                "Rocky espera junto a la escotilla: vuelve a la cabina con E"
            } else {
                "SIGUE A ROCKY hacia la cabina. Deja libre un lado si necesita pasar."
            }
            .to_string()
        } else if self.fase == Fase::TunelBlip {
            if self.encuentro_blip {
                "Encuentro alcanzado: vuelve a la cabina y parte hacia Adrien"
            } else {
                "Recorre el pasillo hasta el cristal hexagonal"
            }
            .to_string()
        } else if self.en_blip_a() {
            if self.encuentro_blip {
                "Ve a la consola de pilotaje para partir hacia Adrien"
            } else {
                "Ve a la esclusa trasera, abre la escotilla y entra al pasillo de Blip-A"
            }
            .to_string()
        } else if self.crucero {
            "Cabina en crucero: H o consola de pilotaje para volver a conducir".to_string()
        } else if self.sistemas_activos {
            "Sistemas en línea: ve a la consola de pilotaje (frente de la cabina) y desacopla"
                .to_string()
        } else if self.fase == Fase::Estacion {
            format!(
                "Recoge los materiales de la estación ({}/5)",
                self.inv.total_recogido()
            )
        } else if !self.inv.mochila.is_empty() {
            "Instala los materiales en el Núcleo (muro de mando, a la derecha)".to_string()
        } else {
            format!(
                "Sal por la esclusa y recoge los materiales ({}/5)",
                self.inv.total_recogido()
            )
        };
        let pendientes = self
            .pe
            .recogibles
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.recogido[*i])
            .map(|(_, p)| (p.x, p.z))
            .collect();
        Hud {
            fase: self.fase,
            dialogo: self.encuentro_rocky.dialogo(),
            comentario: self.encuentro_rocky.comentario(),
            prompt: self.candidato().map(|c| c.texto),
            mensaje: self.mensaje.as_ref().map(|m| m.0.clone()),
            objetivo,
            mochila: self
                .inv
                .mochila
                .iter()
                .map(|m| {
                    let c = m.color();
                    (
                        [
                            (c.x * 255.0) as u8,
                            (c.y * 255.0) as u8,
                            (c.z * 255.0) as u8,
                        ],
                        m.nombre(),
                    )
                })
                .collect(),
            instalados: self.inv.instalados.len(),
            transmision: self.transmision.map(|t| (t / 4.0).clamp(0.0, 1.0)),
            fundido: self.fundido(),
            jugador: (self.jugador.x, self.jugador.z, self.jugador.yaw),
            pendientes,
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn transmision_requiere_datos_y_confirma_una_sola_vez() {
        use crate::progreso::{Galaxia, Progreso};
        let mut p = Progreso::default();
        assert!(!p.transmitir_datos());
        p.completar_estacion();
        p.completar_marte();
        p.completar(Galaxia::Gargantua);
        p.viajar(Galaxia::TauCeti);
        let mut a = Aventura::nueva();
        a.entrar_cabina(V3::ZERO, v3(0.0, 0.0, -1.0), v3(0.0, 1.0, 0.0), true);
        a.jugador.colocar(a.pts.nucleo.x, a.pts.nucleo.z, 0.0);
        assert!(!a.candidato().unwrap().texto.contains("Transmitir"));
        p.completar(Galaxia::TauCeti);
        assert!(!p.desbloqueada(Galaxia::Eridani));
        a.sincronizar_datos(true, false);
        assert!(a.candidato().unwrap().texto.contains("Transmitir"));
        interactuar(&mut a);
        assert!(a.interacciones_bloqueadas());
        avanzar(&mut a, 2.0);
        assert!(!a.datos_enviados());
        interactuar(&mut a);
        avanzar(&mut a, 2.1);
        assert!(a.datos_enviados());
        assert!(!a.interacciones_bloqueadas());
        assert!(p.transmitir_datos());
        assert!(!p.transmitir_datos());
        assert!(p.viajar(Galaxia::Eridani));
        let mut f = crate::final_erid::FinalErid::nuevo(false);
        f.actualizar(5.1, &EntradaJ::default());
        assert!(!f.completado() && !p.completada(Galaxia::Eridani));
        for _ in 0..crate::final_erid::DIALOGOS.len() {
            f.continuar();
        }
        assert!(f.completado());
        assert!(p.completar(Galaxia::Eridani));
        assert!(p.viajar(Galaxia::TauCeti));
        a.suspender();
        a.entrar_cabina(V3::ZERO, v3(0.0, 0.0, -1.0), v3(0.0, 1.0, 0.0), true);
        a.sincronizar_datos(true, p.datos_enviados());
        a.jugador.colocar(a.pts.nucleo.x, a.pts.nucleo.z, 0.0);
        assert!(!a.candidato().unwrap().texto.contains("Transmitir"));
        let nueva = Progreso::default();
        assert!(!nueva.datos_enviados() && !nueva.completada(Galaxia::Eridani));
    }
    #[test]
    fn blip_requiere_recorrer_el_pasillo_hasta_el_cristal_y_permite_volver() {
        use crate::{
            guia,
            progreso::{Galaxia, Progreso},
        };
        let escena = Escena::tau_ceti();
        let blip = escena.nave_cafe.as_ref().unwrap();
        let mut p = Progreso::default();
        p.completar_estacion();
        p.completar_marte();
        p.viajar(Galaxia::Gargantua);
        p.completar(Galaxia::Gargantua);
        p.viajar(Galaxia::TauCeti);
        let mut a = Aventura::nueva();
        a.iniciar_atraque_blip(
            blip.pos + v3(0.0, 0.0, blip.radio + 5.0),
            v3(0.0, 0.0, -1.0),
            blip.pos,
            blip.radio,
            blip.rot,
        );
        avanzar(&mut a, 6.0);
        assert_eq!(a.fase, Fase::Nave);
        assert!(a.en_blip_a() && !a.encuentro_blip_realizado());
        assert_eq!(
            guia::objetivo(&escena, &p, blip.pos).unwrap().nombre,
            "Blip-A"
        );
        a.jugador.colocar(a.pts.esclusa.x, a.pts.esclusa.z, 0.0);
        interactuar(&mut a);
        avanzar(&mut a, 2.0);
        assert!(a.escotilla > 0.9);
        a.jugador.colocar(0.0, FIN_TUNEL - 1.0, 0.0);
        assert_eq!(a.candidato().unwrap().texto, "Entrar al pasillo de Blip-A");
        interactuar(&mut a);
        avanzar(&mut a, 2.0);
        assert_eq!(a.fase, Fase::TunelBlip);
        assert!(!a.encuentro_blip_realizado());
        let adelante = EntradaAventura {
            j: EntradaJ {
                adelante: 1.0,
                ..Default::default()
            },
            ..Default::default()
        };
        for _ in 0..300 {
            a.actualizar(&adelante, 1.0 / 30.0);
            assert!(
                a.jugador.z >= Z_CRISTAL + 0.35 + RADIO - 0.01,
                "atraveso el cristal"
            );
        }
        assert!(!a.encuentro_blip_realizado());
        assert_eq!(a.encuentro_rocky.etapa, Etapa::Inicial(0));
        let pos = (a.jugador.x, a.jugador.z);
        a.actualizar(&adelante, 0.5);
        assert_eq!(pos, (a.jugador.x, a.jugador.z), "dialogo permite caminar");
        for _ in 0..5 {
            interactuar(&mut a);
        }
        assert_eq!(a.encuentro_rocky.etapa, Etapa::Guiando);
        assert!(!a.encuentro_blip_realizado(), "aun falta charla de partida");
        // El jugador deja pasar a Rocky por la izquierda y lo sigue hasta la escotilla.
        for _ in 0..1800 {
            let r = a.encuentro_rocky.pos;
            a.jugador
                .colocar(0.65, (r.z - 2.0).max(Z_CRISTAL + 0.8), std::f32::consts::PI);
            a.actualizar(&EntradaAventura::default(), 1.0 / 30.0);
            if a.encuentro_rocky.apertura >= 1.0 {
                assert!(a.mundo_blip.libre(
                    a.encuentro_rocky.pos.x,
                    a.encuentro_rocky.pos.z,
                    RADIO_ROCKY
                ));
            }
            if a.encuentro_rocky.en_muelle() {
                break;
            }
        }
        assert!(
            a.encuentro_rocky.en_muelle(),
            "Rocky no llego a la escotilla: {:?}",
            a.encuentro_rocky.pos
        );
        a.jugador.colocar(0.65, -1.8, 0.0);
        assert_eq!(a.candidato().unwrap().texto, "Volver a la cabina");
        interactuar(&mut a);
        avanzar(&mut a, 2.0);
        assert_eq!(a.fase, Fase::Nave);
        assert!(a.encuentro_rocky.en_nave);
        assert_eq!(a.jugador.altura_ojos, 1.65);
        // Recorre la cabina con sus puertas, mesa y TARS reales como colisionadores.
        for _ in 0..1000 {
            let r = a.encuentro_rocky.pos;
            a.jugador.colocar(r.x, r.z + 2.0, 0.0);
            a.actualizar(&EntradaAventura::default(), 1.0 / 30.0);
            if a.encuentro_rocky.bloquea() {
                break;
            }
        }
        assert_eq!(
            a.encuentro_rocky.etapa,
            Etapa::Partida(0),
            "Rocky atascado {:?}",
            a.encuentro_rocky.pos
        );
        assert!(!a.encuentro_blip_realizado());
        for _ in 0..4 {
            interactuar(&mut a);
        }
        assert_eq!(a.encuentro_rocky.etapa, Etapa::Abordo);
        assert!(p.completar_encuentro_blip());
        assert!(!p.completar_encuentro_blip());
        assert_eq!(
            guia::objetivo(&escena, &p, blip.pos).unwrap().nombre,
            "ADRIEN"
        );
        a.suspender();
        assert!(a.entrar_cabina(V3::ZERO, v3(0.0, 0.0, -1.0), v3(0.0, 1.0, 0.0), true));
        avanzar(&mut a, 2.0);
        assert_eq!(
            a.encuentro_rocky.etapa,
            Etapa::Abordo,
            "conversacion repetida"
        );
        assert!(a.encuentro_blip_realizado() && a.inv.sistemas_listos());
        a.jugador.colocar(a.pts.pilotaje.x, a.pts.pilotaje.z, 0.0);
        assert_eq!(interactuar(&mut a), Evento::VolverAlVuelo);
    }

    #[test]
    fn salir_antes_del_cristal_no_completa_el_encuentro() {
        let mut a = Aventura::nueva();
        a.iniciar_atraque_blip(
            v3(0.0, 0.0, 20.0),
            v3(0.0, 0.0, -1.0),
            V3::ZERO,
            5.0,
            Rot::identidad(),
        );
        avanzar(&mut a, 6.0);
        a.aplicar(Destino::ATunelBlip);
        avanzar(&mut a, 1.0);
        a.suspender();
        assert!(!a.encuentro_blip_realizado());
        a.iniciar_atraque_blip(
            v3(0.0, 0.0, 20.0),
            v3(0.0, 0.0, -1.0),
            V3::ZERO,
            5.0,
            Rot::identidad(),
        );
        avanzar(&mut a, 6.0);
        assert!(!a.encuentro_blip_realizado());
    }
    #[test]
    fn cabina_en_crucero_requiere_estacion_y_no_regresa_a_un_muelle_antiguo() {
        let mut a = Aventura::nueva();
        assert!(!a.entrar_cabina(V3::ZERO, v3(0.0, 0.0, -1.0), v3(0.0, 1.0, 0.0), false));
        for pos in [v3(60.0, 2.0, 90.0), crate::gargantua::INICIO] {
            let fwd = v3(1.0, 0.0, 0.0);
            let up = v3(0.0, 0.7, 0.7).norm();
            assert!(a.entrar_cabina(pos, fwd, up, true));
            assert!(a.en_crucero() && a.pose_atraque().is_none());
            assert!(a.inv.sistemas_listos());
            assert_eq!(a.nave_pos, pos);
            assert_eq!(a.nave_up, up);
            a.jugador.colocar(a.pts.esclusa.x, a.pts.esclusa.z, 0.0);
            assert!(a.candidato().unwrap().texto.contains("bloqueada"));
            interactuar(&mut a);
            avanzar(&mut a, 2.0);
            assert_eq!(a.escotilla, 0.0);
            a.jugador.colocar(a.pts.pilotaje.x, a.pts.pilotaje.z, 0.0);
            assert_eq!(interactuar(&mut a), Evento::VolverAlVuelo);
            assert_eq!(a.nave_pos, pos);
            assert!(!a.activa());
        }
    }
    #[test]
    fn atracar_requiere_nave_real_cerca_y_lenta_y_salir_conserva_materiales() {
        let escena = Escena::nueva();
        let est = escena.estacion.as_ref().unwrap();
        let vista = escena
            .vistas()
            .into_iter()
            .find(|v| v.0 == "estacion")
            .unwrap();
        assert!(Aventura::puede_atracar(
            vista.1, 0.0, est.pos, est.radio, true
        ));
        assert!(!Aventura::puede_atracar(
            vista.1, 0.0, est.pos, est.radio, false
        ));
        assert!(!Aventura::puede_atracar(
            vista.1, 12.0, est.pos, est.radio, true
        ));
        assert!(!Aventura::puede_atracar(
            est.pos + v3(40.0, 0.0, 0.0),
            0.0,
            est.pos,
            est.radio,
            true
        ));
        let mut a = atracada();
        a.inv.recoger(Material::CeldaEnergia).unwrap();
        a.suspender();
        assert!(!a.activa() && !a.completada());
        a.iniciar_atraque(vista.1, v3(0.0, 0.0, -1.0), est.pos, est.radio, est.rot);
        avanzar(&mut a, 6.0);
        assert_eq!(a.inv.mochila, vec![Material::CeldaEnergia]);
    }

    #[test]
    fn todos_los_materiales_tienen_ruta_caminable_desde_el_muelle() {
        // Búsqueda sobre los colisionadores reales; detecta brazos/anillos
        // desconectados que una prueba con teletransportes no encontraría.
        use std::collections::VecDeque;
        let (e, p) = estacion_interior::construir();
        const N: usize = 201;
        let punto = |x: usize, z: usize| (x as f32 * 0.25 - 25.0, z as f32 * 0.25 - 25.0);
        let inicio = (
            ((p.llegada.0 + 25.0) * 4.0).round() as usize,
            ((p.llegada.1 + 25.0) * 4.0).round() as usize,
        );
        let mut vistos = vec![false; N * N];
        let mut cola = VecDeque::from([inicio]);
        vistos[inicio.1 * N + inicio.0] = true;
        while let Some((x, z)) = cola.pop_front() {
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, nz) = (x as isize + dx, z as isize + dz);
                if nx < 0 || nz < 0 || nx >= N as isize || nz >= N as isize {
                    continue;
                }
                let (nx, nz) = (nx as usize, nz as usize);
                let (px, pz) = punto(nx, nz);
                if !vistos[nz * N + nx] && e.mundo.libre(px, pz, RADIO) {
                    vistos[nz * N + nx] = true;
                    cola.push_back((nx, nz));
                }
            }
        }
        for (i, p) in p.recogibles.iter().enumerate() {
            assert!(
                vistos.iter().enumerate().any(|(id, v)| {
                    let (x, z) = punto(id % N, id / N);
                    *v && dxz(x, z, *p) < 1.8
                }),
                "material {i} sin ruta desde el muelle"
            );
        }
    }

    fn avanzar(a: &mut Aventura, seg: f32) {
        let mut t = 0.0;
        while t < seg {
            a.actualizar(&EntradaAventura::default(), 1.0 / 30.0);
            t += 1.0 / 30.0;
        }
    }
    fn interactuar(a: &mut Aventura) -> Evento {
        a.actualizar(
            &EntradaAventura {
                interactuar: true,
                ..Default::default()
            },
            1.0 / 60.0,
        )
    }

    fn atracada() -> Aventura {
        let mut a = Aventura::nueva();
        a.iniciar_atraque(
            v3(0.0, 0.0, 40.0),
            v3(0.0, 0.0, -1.0),
            V3::ZERO,
            3.2,
            Rot::identidad(),
        );
        assert_eq!(a.fase, Fase::Atraque);
        assert!(a.pose_atraque().is_some());
        avanzar(&mut a, 6.0);
        assert_eq!(a.fase, Fase::Nave);
        a
    }

    #[test]
    fn atraque_termina_dentro_de_la_nave_y_la_nave_llega_a_la_estacion() {
        let mut a = Aventura::nueva();
        a.iniciar_atraque(
            v3(0.0, 0.0, 40.0),
            v3(0.0, 0.0, -1.0),
            V3::ZERO,
            3.2,
            Rot::identidad(),
        );
        let (p, _) = a.pose_atraque().unwrap();
        assert!((p - v3(0.0, 0.0, 40.0)).len() < 0.01);
        avanzar(&mut a, 6.0);
        let (p, f) = a.pose_atraque().unwrap();
        assert!((p.len() - 6.7).abs() < 0.01 && f.z < -0.99, "{p:?}");
        assert_eq!((a.jugador.x, a.jugador.z), (0.0, 4.8));
    }

    #[test]
    fn la_puerta_y_la_escotilla_bloquean_hasta_abrirse() {
        let mut a = atracada();
        // La esclusa: sin abrir la escotilla el túnel es inalcanzable.
        a.jugador.colocar(0.0, 12.5, 0.0);
        let e = EntradaAventura {
            j: EntradaJ {
                adelante: -1.0,
                ..Default::default()
            },
            ..Default::default()
        };
        // adelante = -1 mira hacia -z con yaw 0: caminamos hacia +z con yaw pi
        a.jugador.colocar(0.0, 12.5, std::f32::consts::PI);
        let _ = e;
        let avance = EntradaAventura {
            j: EntradaJ {
                adelante: 1.0,
                ..Default::default()
            },
            ..Default::default()
        };
        for _ in 0..120 {
            a.actualizar(&avance, 1.0 / 30.0);
        }
        assert!(
            a.jugador.z < FIN_ESCLUSA,
            "atravesó la escotilla cerrada: {}",
            a.jugador.z
        );
        // Se abre desde el panel y ahora sí pasa.
        a.jugador.colocar(1.5, 11.0, 0.0);
        assert!(interactuar(&mut a) == Evento::Ninguno && a.escotilla_abierta);
        avanzar(&mut a, 2.0);
        a.jugador.colocar(0.0, 12.5, std::f32::consts::PI);
        for _ in 0..90 {
            a.actualizar(&avance, 1.0 / 30.0);
        }
        assert!(a.jugador.z > FIN_ESCLUSA + 1.0, "no pasó: {}", a.jugador.z);
    }

    #[test]
    fn mision_completa_recoger_instalar_y_desacoplar() {
        let mut a = atracada();
        // salir a la estación
        a.escotilla_abierta = true;
        a.escotilla = 1.0;
        a.jugador.colocar(0.0, FIN_TUNEL - 1.0, 0.0);
        assert_eq!(a.candidato().unwrap().texto, "Salir a la estación");
        interactuar(&mut a);
        avanzar(&mut a, 2.0);
        assert_eq!(a.fase, Fase::Estacion);
        let (x, z, _) = a.pe.llegada;
        assert!((a.jugador.x - x).abs() < 0.5 && (a.jugador.z - z).abs() < 0.5);
        // los cinco materiales son alcanzables caminando (están en zonas libres)
        let pos = a.pe.recogibles.clone();
        for (i, p) in pos.iter().enumerate() {
            let mundo = &a.estacion.mundo;
            let alcanzable = (0..16).any(|k| {
                let ang = k as f32 * std::f32::consts::TAU / 16.0;
                mundo.libre(p.x + 1.3 * ang.cos(), p.z + 1.3 * ang.sin(), RADIO)
            });
            assert!(alcanzable, "material {i} inalcanzable");
            a.jugador.colocar(p.x + 0.6, p.z, 0.0);
            assert!(a.candidato().unwrap().texto.starts_with("Recoger"), "{i}");
            interactuar(&mut a);
        }
        assert_eq!(a.inv.mochila.len(), 5);
        assert!(a.recogido.iter().all(|r| *r));
        // volver a la nave
        a.jugador.colocar(a.pe.nave.x, a.pe.nave.z, 0.0);
        assert_eq!(a.candidato().unwrap().texto, "Volver a la nave");
        interactuar(&mut a);
        avanzar(&mut a, 2.0);
        assert_eq!(a.fase, Fase::Nave);
        // sin instalar, la consola de pilotaje no responde
        let p = a.pts.pilotaje;
        a.jugador.colocar(p.x, p.z, 0.0);
        assert!(a.candidato().unwrap().accion.is_none());
        // instalar en el Núcleo
        let n = a.pts.nucleo;
        a.jugador.colocar(n.x, n.z, 0.0);
        assert!(a.candidato().unwrap().texto.starts_with("Instalar 5"));
        interactuar(&mut a);
        assert!(a.sistemas_activos && a.inv.sistemas_listos() && a.inv.mochila.is_empty());
        // ahora sí se puede desacoplar
        a.jugador.colocar(p.x, p.z, 0.0);
        assert_eq!(interactuar(&mut a), Evento::Desacoplar);
        assert_eq!(a.fase, Fase::Inactiva);
        let (pos, obj) = a.pose_desacople();
        assert!((obj - pos).dot(pos - a.est_pos) > 0.0);
    }

    #[test]
    fn tars_patrulla_sin_atravesar_muebles() {
        let mut a = atracada();
        a.jugador.colocar(0.0, 6.5, 0.0);
        for _ in 0..(60 * 30) {
            a.actualizar(&EntradaAventura::default(), 1.0 / 30.0);
            let dinamicos = std::mem::take(&mut a.cabina.mundo.dinamicos);
            assert!(a.cabina.mundo.libre(
                a.tars.origen.x,
                a.tars.origen.z,
                crate::tars::RADIO * 0.8
            ));
            a.cabina.mundo.dinamicos = dinamicos;
        }
        let o = a.tars.origen;
        assert!(o.z < -5.0 && o.z > -8.0 && o.x.abs() < 8.5, "{o:?}");
    }

    #[test]
    fn dibuja_ambas_fases_sin_paniquear() {
        let mut a = atracada();
        let escena = Escena::nueva();
        let cielo = Cielo::nuevo();
        let mut fb = Framebuffer::new(320, 200);
        a.dibujar(&mut fb, &escena, &cielo, &Ajustes::default());
        assert!(
            fb.zbuf.iter().filter(|z| **z > 0.0).count() > 20000,
            "la cabina no escribe profundidad"
        );
        a.aplicar(Destino::AEstacion);
        a.dibujar(&mut fb, &escena, &cielo, &Ajustes::default());
        assert!(fb.zbuf.iter().filter(|z| **z > 0.0).count() > 20000);
        let h = a.hud();
        assert_eq!(h.pendientes.len(), 5);
    }
}
