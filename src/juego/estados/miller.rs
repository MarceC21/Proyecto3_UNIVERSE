// Reglas de Miller independientes de ventana, reloj del sistema y renderizado.
use crate::{math::*, ruido::Rng};

pub const CAJAS: [V3; 3] = [
    v3(-9.0, 1.7, -16.0),
    v3(11.0, 1.7, -25.0),
    v3(-12.0, 1.7, -35.0),
];
pub const NAVE: V3 = v3(0.0, 1.7, 0.0);
pub const INICIO: V3 = v3(0.0, 1.7, -6.0);
pub const RADIO_INTERACCION: f32 = 3.5;
pub const RADIO_LLEGADA: f32 = 3.0;
pub const RADIO_OLA_INICIAL: f32 = 200.0;
pub const DURACION_DESCENSO: f64 = 5.0;

// Dirección de transporte asistido, usando los mismos círculos que la cámara.
pub fn direccion_regreso(pos: V3, colisionadores: &[(V3, f32)]) -> V3 {
    let mut objetivo = NAVE;
    let dir = (NAVE - pos).norm();
    for (c, r) in colisionadores {
        let d = *c - pos;
        let avance = d.dot(dir);
        if avance > 0.0 && avance < (objetivo - pos).len() && (d - dir * avance).len() < r + 1.0 {
            objetivo = *c + v3(-dir.z, 0.0, dir.x) * (r + 2.0);
        }
    }
    (objetivo - pos).norm()
}

#[derive(Clone, Copy, Debug)]
pub struct Configuracion {
    pub limite_exito: f64,
    pub llegada_ola: f64,
}
impl Default for Configuracion {
    fn default() -> Self {
        Self {
            limite_exito: 90.0,
            llegada_ola: 110.0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resultado {
    Exito,
    Tardio,
    Ola,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fase {
    Descenso,
    Instrucciones,
    Explorando,
    Cuestionario,
    Estrategia,
    Regreso,
    Final(Resultado),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estrategia {
    Tars,
    Correr,
    Equipo,
}
impl Estrategia {
    pub fn nombre(self) -> &'static str {
        match self {
            Self::Tars => "ASISTENCIA DE TARS",
            Self::Correr => "CORRER A LA NAVE",
            Self::Equipo => "LLEVAR EQUIPO ADICIONAL",
        }
    }
    fn duracion(self) -> f32 {
        match self {
            Self::Tars => 8.0,
            Self::Correr => 18.0,
            Self::Equipo => 35.0,
        }
    }
}
pub struct Pregunta {
    pub lineas: [&'static str; 2],
    pub opciones: [&'static str; 3],
    pub correcta: usize,
    pub explicacion: &'static str,
}
pub const PREGUNTAS: [Pregunta; 4] = [
    Pregunta {
        lineas: [
            "En iluminación, ¿por qué se normaliza",
            "la normal de una superficie?",
        ],
        opciones: [
            "Para limitar el color a 0..255.",
            "Para que su magnitud no altere el producto punto.",
            "Para independizar el especular de la cámara.",
        ],
        correcta: 1,
        explicacion: "La normal debe tener longitud uno para calcular la iluminación.",
    },
    Pregunta {
        lineas: [
            "¿Qué componente de luz desplaza su brillo",
            "sobre el objeto cuando se mueve la cámara?",
        ],
        opciones: [
            "La luz ambiental.",
            "La luz difusa (Lambert).",
            "La luz especular.",
        ],
        correcta: 2,
        explicacion: "El brillo especular depende de la posicion del observador.",
    },
    Pregunta {
        lineas: [
            "¿Cómo se obtiene el eje X (right)",
            "de la base de una cámara orbital?",
        ],
        opciones: [
            "Producto cruz de forward con up.",
            "Restando center menos eye.",
            "Producto punto de eye con center.",
        ],
        correcta: 0,
        explicacion: "right = forward x up; invertir el orden invierte el eje.",
    },
    Pregunta {
        lineas: [
            "En el render pipeline, ¿qué hace",
            "el depth test (z-buffer)?",
        ],
        opciones: [
            "Recorta primitivas fuera del frustum.",
            "Deja pasar el fragmento si esta mas cerca.",
            "Calcula el color muestreando la textura UV.",
        ],
        correcta: 1,
        explicacion: "El z-buffer compara la profundidad guardada por pixel.",
    },
];

pub struct MisionMiller {
    pub descenso: f64,
    pub fase: Fase,
    pub tiempo: f64,
    pub inspeccionadas: [bool; 3],
    pub pregunta: usize,
    pub estrategia: Option<Estrategia>,
    pub mensaje: &'static str,
    config: Configuracion,
    correcta: usize,
    velocidad: f32,
}
impl MisionMiller {
    pub fn nueva(semilla: u64, config: Configuracion) -> Self {
        assert!(config.limite_exito.is_finite() && config.limite_exito > 0.0);
        assert!(config.llegada_ola.is_finite() && config.llegada_ola > config.limite_exito);
        Self {
            descenso: 0.0,
            fase: Fase::Explorando,
            tiempo: 0.0,
            inspeccionadas: [false; 3],
            pregunta: 0,
            estrategia: None,
            mensaje: "Busca el modulo cientifico en los tres contenedores.",
            config,
            correcta: (Rng::new(semilla).siguiente() % 3) as usize,
            velocidad: 3.5,
        }
    }
    pub fn aterrizar(semilla: u64, config: Configuracion) -> Self {
        let mut m = Self::nueva(semilla, config);
        m.fase = Fase::Descenso;
        m
    }
    pub fn comenzar(&mut self) {
        if self.fase == Fase::Instrucciones {
            self.fase = Fase::Explorando;
        }
    }
    pub fn radio_ola(&self) -> f32 {
        (RADIO_OLA_INICIAL
            - (RADIO_OLA_INICIAL - RADIO_LLEGADA) * (self.tiempo / self.config.llegada_ola) as f32)
            .max(0.0)
    }
    pub fn datos_recuperados(&self) -> bool {
        self.pregunta == PREGUNTAS.len()
    }
    pub fn velocidad(&self) -> f32 {
        self.velocidad
    }
    pub fn cercana(&self, pos: V3) -> Option<usize> {
        if self.fase != Fase::Explorando {
            return None;
        }
        CAJAS
            .iter()
            .enumerate()
            .filter(|(i, p)| !self.inspeccionadas[*i] && (**p - pos).len() <= RADIO_INTERACCION)
            .min_by(|a, b| (*a.1 - pos).len().total_cmp(&(*b.1 - pos).len()))
            .map(|(i, _)| i)
    }
    pub fn inspeccionar(&mut self, i: usize, pos: V3) -> bool {
        if self.cercana(pos) != Some(i) {
            return false;
        }
        self.inspeccionadas[i] = true;
        if i == self.correcta {
            self.mensaje = "MODULO DE DATOS ENCONTRADO - Iniciando verificacion.";
            self.fase = Fase::Cuestionario;
        } else {
            self.mensaje = "CONTENEDOR VACIO - Continua buscando.";
        }
        true
    }
    pub fn responder(&mut self, opcion: usize) -> bool {
        if self.fase != Fase::Cuestionario {
            return false;
        }
        let p = &PREGUNTAS[self.pregunta];
        if opcion != p.correcta {
            self.mensaje = "Respuesta incorrecta. Revisa las opciones e intenta otra vez.";
            return false;
        }
        self.mensaje = p.explicacion;
        self.pregunta += 1;
        if self.datos_recuperados() {
            self.fase = Fase::Estrategia;
            self.mensaje = "DATOS RECUPERADOS - REGRESA A LA NAVE.";
        }
        true
    }
    pub fn elegir(&mut self, estrategia: Estrategia, pos: V3) -> bool {
        if self.fase != Fase::Estrategia {
            return false;
        }
        self.estrategia = Some(estrategia);
        self.velocidad = ((pos - NAVE).len() / estrategia.duracion()).max(0.15);
        self.fase = Fase::Regreso;
        self.mensaje = match estrategia {
            Estrategia::Tars => "TARS: asistencia activada. Te guio hasta la nave.",
            Estrategia::Correr => "Corre hacia la baliza de la nave. WASD para desplazarte.",
            Estrategia::Equipo => "Equipo adicional recuperado. El peso reduce tu velocidad.",
        };
        true
    }
    // dt viene de Instant, sin recortarlo a la duración de un fotograma ideal.
    // La ola tiene prioridad si alcanza al jugador al entrar a la zona de llegada.
    pub fn actualizar(&mut self, dt: f64, pos: V3) {
        if matches!(self.fase, Fase::Final(_)) || !dt.is_finite() || dt < 0.0 {
            return;
        }
        if self.fase == Fase::Descenso {
            self.descenso = (self.descenso + dt).min(DURACION_DESCENSO);
            if self.descenso >= DURACION_DESCENSO {
                self.fase = Fase::Instrucciones;
            }
            return;
        }
        if self.fase == Fase::Instrucciones {
            return;
        }
        self.tiempo += dt;
        let radial = (pos.x * pos.x + pos.z * pos.z).sqrt();
        if radial + 0.6 >= self.radio_ola() {
            self.fase = Fase::Final(Resultado::Ola);
        } else if self.fase == Fase::Regreso
            && self.datos_recuperados()
            && (pos - NAVE).len() <= RADIO_LLEGADA
        {
            self.fase = Fase::Final(if self.tiempo <= self.config.limite_exito {
                Resultado::Exito
            } else {
                Resultado::Tardio
            });
        }
    }
    pub fn dias_exteriores(&self) -> f64 {
        self.tiempo * 7.0 * 365.0 / 3600.0
    }
    pub fn exitosa(&self) -> bool {
        self.fase == Fase::Final(Resultado::Exito)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aterrizaje_e_instrucciones_no_consumen_reloj_ni_mueven_ola() {
        let mut m = MisionMiller::aterrizar(5, Configuracion::default());
        m.comenzar();
        assert_eq!(m.fase, Fase::Descenso);
        m.actualizar(2.0, INICIO);
        assert_eq!(m.descenso, 2.0);
        assert_eq!(m.tiempo, 0.0);
        m.actualizar(4.0, INICIO);
        assert_eq!(m.fase, Fase::Instrucciones);
        m.actualizar(200.0, INICIO);
        assert_eq!(m.radio_ola(), RADIO_OLA_INICIAL);
        assert_eq!(m.tiempo, 0.0);
        assert!(!m.inspeccionar(0, CAJAS[0]));
        m.comenzar();
        m.actualizar(2.0, INICIO);
        assert_eq!(m.fase, Fase::Explorando);
        assert_eq!(m.tiempo, 2.0);
        assert!(m.radio_ola() < RADIO_OLA_INICIAL);
    }
    fn datos(m: &mut MisionMiller) {
        assert!(m.inspeccionar(m.correcta, CAJAS[m.correcta]));
        assert!(!m.responder(9));
        assert_eq!(m.pregunta, 0);
        for p in &PREGUNTAS {
            assert!(m.responder(p.correcta));
        }
        assert_eq!(m.fase, Fase::Estrategia);
    }
    #[test]
    fn cajas_requieren_distancia_son_unicas_y_la_semilla_varia_el_modulo() {
        let mut vistas = [false; 3];
        for semilla in 0..100 {
            let mut m = MisionMiller::nueva(semilla, Configuracion::default());
            vistas[m.correcta] = true;
            assert!(!m.inspeccionar(m.correcta, INICIO));
            for i in 0..3 {
                if i != m.correcta {
                    assert!(m.inspeccionar(i, CAJAS[i]));
                    assert!(!m.inspeccionar(i, CAJAS[i]));
                }
            }
            assert!(m.inspeccionar(m.correcta, CAJAS[m.correcta]));
            assert_eq!(m.inspeccionadas, [true; 3]);
        }
        assert_eq!(vistas, [true; 3]);
    }
    #[test]
    fn resultados_reloj_y_conversion_se_congelan_al_final() {
        for (t, esperado) in [
            (80.0, Resultado::Exito),
            (95.0, Resultado::Tardio),
            (112.0, Resultado::Ola),
        ] {
            let mut m = MisionMiller::nueva(4, Configuracion::default());
            m.actualizar(5.0, NAVE);
            assert_eq!(m.fase, Fase::Explorando); // llegada sin datos no gana
            datos(&mut m);
            m.elegir(Estrategia::Correr, CAJAS[m.correcta]);
            m.actualizar(t - 5.0, NAVE);
            assert_eq!(m.fase, Fase::Final(esperado));
            m.actualizar(100.0, INICIO);
            assert_eq!(m.tiempo, t);
            assert!((m.dias_exteriores() - t * 7.0 * 365.0 / 3600.0).abs() < 1e-8);
        }
        let mut m = MisionMiller::nueva(0, Configuracion::default());
        m.tiempo = 3600.0;
        assert_eq!(m.dias_exteriores(), 7.0 * 365.0);
    }
    #[test]
    fn estrategias_no_suman_tiempo_y_reinicio_limpia_todo() {
        let mut velocidades = vec![];
        for e in [Estrategia::Tars, Estrategia::Correr, Estrategia::Equipo] {
            let mut m = MisionMiller::nueva(12, Configuracion::default());
            datos(&mut m);
            assert!(m.elegir(e, CAJAS[m.correcta]));
            assert!(!m.elegir(e, CAJAS[m.correcta]));
            assert_eq!(m.tiempo, 0.0);
            velocidades.push(m.velocidad());
        }
        assert!(velocidades[0] > velocidades[1] && velocidades[1] > velocidades[2]);
        let m = MisionMiller::nueva(13, Configuracion::default());
        assert_eq!(
            (m.tiempo, m.pregunta, m.inspeccionadas, m.estrategia),
            (0.0, 0, [false; 3], None)
        );
        assert_eq!(m.radio_ola(), RADIO_OLA_INICIAL);
        assert_eq!(m.fase, Fase::Explorando);
    }
    #[test]
    fn ola_depende_del_lugar_y_del_tiempo_no_de_los_fps() {
        for fps in [30, 144] {
            let mut m = MisionMiller::nueva(0, Configuracion::default());
            for _ in 0..fps * 90 {
                m.actualizar(1.0 / fps as f64, NAVE);
            }
            assert!((m.tiempo - 90.0).abs() < 1e-8);
            assert!(!matches!(m.fase, Fase::Final(_)));
            m.actualizar(0.0, v3(0.0, 1.7, -50.0));
            assert_eq!(m.fase, Fase::Final(Resultado::Ola));
        }
    }

    #[test]
    fn camara_reutilizada_colisiona_y_el_transporte_regresa_desde_las_tres_cajas() {
        use crate::camara::Entrada;
        let escena = crate::paisaje_miller::EscenaMiller::nueva();
        let cols = escena.colisionadores();
        for fps in [30, 144] {
            for caja in CAJAS {
                for lado in [v3(0.0, 0.0, 1.0), v3(0.0, 0.0, -1.0), v3(1.0, 0.0, 0.0)] {
                    let mut cam = crate::paisaje_miller::EscenaMiller::camara(160, 100);
                    cam.pos = caja + lado * 3.0;
                    let velocidad = (cam.pos - NAVE).len() / 8.0;
                    let mut t = 0.0;
                    while t < 14.0 && (cam.pos - NAVE).len() > RADIO_LLEGADA {
                        let dir = direccion_regreso(cam.pos, &cols);
                        let (f, r, _) = cam.base();
                        let e = Entrada {
                            adelante: dir.dot(v3(f.x, 0.0, f.z).norm()),
                            lateral: dir.dot(r),
                            vertical: 0.0,
                            giro_x: 0.0,
                            giro_y: 0.0,
                            turbo: false,
                        };
                        cam.actualizar_superficie(&e, 1.0 / fps as f32, &cols, velocidad);
                        for (p, rad) in &cols {
                            assert!((cam.pos - *p).len() >= rad + 0.49);
                        }
                        assert_eq!(cam.pos.y, 1.7);
                        t += 1.0 / fps as f32;
                    }
                    assert!(
                        (cam.pos - NAVE).len() <= RADIO_LLEGADA,
                        "transporte atascado desde {caja:?}, lado {lado:?}: {:?}",
                        cam.pos
                    );
                }
            }
        }
    }
}
