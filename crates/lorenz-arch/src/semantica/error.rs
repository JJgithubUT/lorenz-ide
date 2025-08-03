//! Errores al analizar el texto de una semantica.

use std::fmt;

/// Un fallo de sintaxis, con la posicion (en caracteres desde el inicio de la cadena) donde se
/// detecto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorSemantica {
    pub posicion: usize,
    pub mensaje: String,
}

impl ErrorSemantica {
    pub fn nuevo(posicion: usize, mensaje: impl Into<String>) -> Self {
        Self { posicion, mensaje: mensaje.into() }
    }
}

impl fmt::Display for ErrorSemantica {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (caracter {})", self.mensaje, self.posicion + 1)
    }
}

impl std::error::Error for ErrorSemantica {}
