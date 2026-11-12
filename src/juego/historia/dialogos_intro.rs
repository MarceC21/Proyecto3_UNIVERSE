// Textos de la introducción (versión mínima). Sin raylib: solo datos.
pub const TITULO: &str = "ÚLTIMA LUZ";
pub const SUBTITULO: &str = "Un viaje para salvar al Sol";
pub const PROMPT_TITULO: &str = "Pulsa ENTER";
pub const AYUDA_OMITIR: &str = "ENTER / ESPACIO / ESC para omitir";

pub struct Subtitulo {
    pub desde: f32,
    pub hasta: f32,
    pub nombre: &'static str, // "" = narrador (sin etiqueta)
    pub texto: &'static str,
}

// Tiempos en segundos desde el inicio de la intro (ver intro.rs para los planos).
pub const SUBTITULOS: [Subtitulo; 4] = [
    Subtitulo {
        desde: 0.8,
        hasta: 3.3,
        nombre: "",
        texto: "Año 2026. El Sol se está apagando.",
    },
    Subtitulo {
        desde: 3.5,
        hasta: 5.8,
        nombre: "",
        texto: "Algo se alimenta de su luz.",
    },
    Subtitulo {
        desde: 6.6,
        hasta: 10.0,
        nombre: "",
        texto: "Quedan pocas misiones por intentar. Esta es la tuya.",
    },
    Subtitulo {
        desde: 10.4,
        hasta: 13.6,
        nombre: "TARS",
        texto: "Sistemas apagados. Necesitamos materiales de la estación.",
    },
];

const FUNDIDO: f32 = 0.3;

// Subtítulo activo y su opacidad 0..1 (aparece y desaparece suavemente).
pub fn activo(t: f32) -> Option<(&'static Subtitulo, f32)> {
    SUBTITULOS
        .iter()
        .find(|s| t >= s.desde && t <= s.hasta)
        .map(|s| {
            let a = ((t - s.desde) / FUNDIDO)
                .min((s.hasta - t) / FUNDIDO)
                .clamp(0.0, 1.0);
            (s, a)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subtitulos_ordenados_sin_solaparse() {
        for w in SUBTITULOS.windows(2) {
            assert!(w[0].hasta <= w[1].desde);
        }
        for s in &SUBTITULOS {
            assert!(s.desde < s.hasta);
        }
    }
    #[test]
    fn opacidad_en_rango() {
        assert!(activo(0.0).is_none());
        let (_, a) = activo(0.8).unwrap();
        assert_eq!(a, 0.0);
        let (_, a) = activo(2.0).unwrap();
        assert_eq!(a, 1.0);
    }
}
