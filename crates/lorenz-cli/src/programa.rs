//! Lectura de programas en texto hexadecimal.
//!
//! Un programa es una lista de palabras separadas por espacios, comas o saltos de linea. Cada
//! palabra es hexadecimal (`2A`, `0x2A`) o binaria (`0b101010`). Todo lo que sigue a `#` en una
//! linea es comentario.

use thiserror::Error;

/// Fallo al leer un programa.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorPrograma {
    #[error("linea {linea}: '{texto}' no es una palabra hexadecimal ni binaria")]
    PalabraInvalida { linea: usize, texto: String },

    #[error("linea {linea}: {valor:#x} no cabe en una palabra de {bits} bits")]
    PalabraDemasiadoGrande { linea: usize, valor: u64, bits: u32 },
}

/// Lee un programa de palabras de `bits` bits.
pub fn leer_palabras(texto: &str, bits: u32) -> Result<Vec<u64>, ErrorPrograma> {
    let mascara = if bits >= 64 { u64::MAX } else { (1u64 << bits) - 1 };
    let mut palabras = Vec::new();
    for (n, linea) in texto.lines().enumerate() {
        let linea_numero = n + 1;
        let sin_comentario = linea.split('#').next().unwrap_or("");
        for token in sin_comentario.split(|c: char| c.is_whitespace() || c == ',') {
            if token.is_empty() {
                continue;
            }
            let invalida = || ErrorPrograma::PalabraInvalida { linea: linea_numero, texto: token.to_string() };
            let valor = if let Some(b) = token.strip_prefix("0b").or_else(|| token.strip_prefix("0B")) {
                u64::from_str_radix(b, 2).map_err(|_| invalida())?
            } else {
                let h = token.strip_prefix("0x").or_else(|| token.strip_prefix("0X")).unwrap_or(token);
                u64::from_str_radix(h, 16).map_err(|_| invalida())?
            };
            if valor & !mascara != 0 {
                return Err(ErrorPrograma::PalabraDemasiadoGrande { linea: linea_numero, valor, bits });
            }
            palabras.push(valor);
        }
    }
    Ok(palabras)
}
