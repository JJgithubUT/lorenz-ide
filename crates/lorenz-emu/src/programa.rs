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
    let mascara = if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    let mut palabras = Vec::new();
    for (n, linea) in texto.lines().enumerate() {
        let linea_numero = n + 1;
        let sin_comentario = linea.split('#').next().unwrap_or("");
        for token in sin_comentario.split(|c: char| c.is_whitespace() || c == ',') {
            if token.is_empty() {
                continue;
            }
            let invalida = || ErrorPrograma::PalabraInvalida {
                linea: linea_numero,
                texto: token.to_string(),
            };
            let valor = if let Some(b) = token
                .strip_prefix("0b")
                .or_else(|| token.strip_prefix("0B"))
            {
                u64::from_str_radix(b, 2).map_err(|_| invalida())?
            } else {
                let h = token
                    .strip_prefix("0x")
                    .or_else(|| token.strip_prefix("0X"))
                    .unwrap_or(token);
                u64::from_str_radix(h, 16).map_err(|_| invalida())?
            };
            if valor & !mascara != 0 {
                return Err(ErrorPrograma::PalabraDemasiadoGrande {
                    linea: linea_numero,
                    valor,
                    bits,
                });
            }
            palabras.push(valor);
        }
    }
    Ok(palabras)
}

/// Escribe palabras de `bits` bits como texto hexadecimal, `por_linea` por linea, con la misma
/// sintaxis que `leer_palabras` entiende (RF-EMU-024).
pub fn escribir_palabras(palabras: &[u64], bits: u32, por_linea: usize) -> String {
    let digitos = (bits as usize).div_ceil(4);
    let por_linea = por_linea.max(1);
    let mut texto = String::new();
    for fila in palabras.chunks(por_linea) {
        let fila: Vec<String> = fila.iter().map(|p| format!("{p:0digitos$X}")).collect();
        texto.push_str(&fila.join(" "));
        texto.push('\n');
    }
    texto
}

/// Carga en cada region el archivo que declara como `contenido` (RF-ARQ-029).
///
/// La ruta es relativa a `base` (la carpeta de la descripcion de arquitectura) y el archivo usa la
/// misma sintaxis que un programa. El contenido empieza en el primer extremo de la region y escribe
/// aunque sea de solo lectura, que es justo para lo que sirve una ROM. Devuelve cuantas regiones
/// se cargaron. Debe hacerse antes de cargar el programa del usuario, que asi puede sobrescribirlo.
pub fn precargar_regiones(
    emulador: &mut crate::Emulador,
    base: &std::path::Path,
) -> Result<usize, String> {
    let regiones: Vec<_> = emulador
        .maquina()
        .regiones
        .iter()
        .filter_map(|r| r.contenido.clone().map(|c| (r.clone(), c)))
        .collect();
    let bits = emulador.maquina().bits_palabra;
    let mut cargadas = 0;
    for (region, ruta) in regiones {
        let completa = base.join(&ruta);
        let texto = std::fs::read_to_string(&completa).map_err(|e| {
            format!(
                "region `{}`: no se pudo leer el contenido `{}`: {e}",
                region.nombre,
                completa.display()
            )
        })?;
        let palabras = leer_palabras(&texto, bits)
            .map_err(|e| format!("region `{}`: `{ruta}`: {e}", region.nombre))?;
        if palabras.len() as u64 > region.tamano() {
            return Err(format!(
                "region `{}`: `{ruta}` tiene {} palabras y la region solo {}",
                region.nombre,
                palabras.len(),
                region.tamano()
            ));
        }
        emulador
            .cargar_programa(region.desde, &palabras)
            .map_err(|e| format!("region `{}`: {e}", region.nombre))?;
        cargadas += 1;
    }
    Ok(cargadas)
}
