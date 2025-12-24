//! Diagnosticos del compilador: siempre con archivo, linea y columna (RF-SGT-036).

use std::fmt;

/// Un lugar del codigo fuente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct Lugar {
    pub linea: u32,
    pub columna: u32,
}

impl Lugar {
    pub fn nuevo(linea: u32, columna: u32) -> Self {
        Self { linea, columna }
    }
}

impl fmt::Display for Lugar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.linea, self.columna)
    }
}

/// Un error del compilador.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostico {
    pub archivo: String,
    pub lugar: Lugar,
    pub mensaje: String,
    /// Correccion sugerida cuando la causa es reconocible (RF-SGT-037).
    pub sugerencia: Option<String>,
}

impl Diagnostico {
    pub fn nuevo(archivo: &str, lugar: Lugar, mensaje: impl Into<String>) -> Self {
        Self {
            archivo: archivo.to_string(),
            lugar,
            mensaje: mensaje.into(),
            sugerencia: None,
        }
    }

    pub fn con_sugerencia(mut self, sugerencia: impl Into<String>) -> Self {
        self.sugerencia = Some(sugerencia.into());
        self
    }
}

impl fmt::Display for Diagnostico {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.archivo, self.lugar, self.mensaje)?;
        if let Some(s) = &self.sugerencia {
            write!(f, "\n  sugerencia: {s}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostico {}
