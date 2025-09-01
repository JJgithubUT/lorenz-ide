//! Guiones de entrada: eventos grabados que se reproducen en un momento exacto de la ejecucion.
//!
//! Un guion hace que una ejecucion con teclado, UART o interrupciones externas sea repetible
//! (RF-EMU-041). Cada linea es un evento:
//!
//! ```text
//! # comentario
//! @0   teclado 0x41          # pulsa una tecla antes de la primera instruccion
//! @10  serie "hola\n"        # entrega texto a la primera UART
//! @25  interrupcion 0        # levanta la linea 0
//! ```
//!
//! `@N` es el numero de orden de la instruccion (0 es la primera): el evento ocurre justo antes de
//! ejecutarla.

use thiserror::Error;

/// Que pasa en un evento del guion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Accion {
    /// Pulsar una tecla en el primer teclado.
    Tecla(u64),
    /// Entregar bytes a la primera UART.
    Serie(Vec<u8>),
    /// Levantar una linea de interrupcion.
    Interrupcion(u32),
}

/// Un evento y el momento en que ocurre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evento {
    pub instruccion: u64,
    pub accion: Accion,
}

/// Un guion mal escrito; siempre dice en que linea.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorGuion {
    #[error("linea {linea}: se esperaba '@INSTRUCCION' al principio, y vino '{texto}'")]
    SinMomento { linea: usize, texto: String },
    #[error("linea {linea}: falta la accion despues de '{momento}'")]
    SinAccion { linea: usize, momento: String },
    #[error("linea {linea}: la accion '{accion}' no existe; las validas son teclado, serie e interrupcion")]
    AccionDesconocida { linea: usize, accion: String },
    #[error("linea {linea}: '{texto}' no es un numero valido")]
    NumeroInvalido { linea: usize, texto: String },
    #[error("linea {linea}: falta el argumento de '{accion}'")]
    SinArgumento { linea: usize, accion: String },
    #[error("linea {linea}: el texto debe ir entre comillas dobles")]
    TextoSinComillas { linea: usize },
    #[error("linea {linea}: el texto no termina: falta la comilla de cierre")]
    TextoSinCerrar { linea: usize },
    #[error("linea {linea}: '\\{caracter}' no es una secuencia de escape valida")]
    EscapeInvalido { linea: usize, caracter: char },
    #[error("linea {linea}: sobra '{texto}' al final")]
    Sobra { linea: usize, texto: String },
}

fn numero(linea: usize, texto: &str) -> Result<u64, ErrorGuion> {
    let invalido = || ErrorGuion::NumeroInvalido {
        linea,
        texto: texto.to_string(),
    };
    let limpio = texto.replace('_', "");
    if let Some(h) = limpio
        .strip_prefix("0x")
        .or_else(|| limpio.strip_prefix("0X"))
    {
        u64::from_str_radix(h, 16).map_err(|_| invalido())
    } else if let Some(b) = limpio
        .strip_prefix("0b")
        .or_else(|| limpio.strip_prefix("0B"))
    {
        u64::from_str_radix(b, 2).map_err(|_| invalido())
    } else {
        limpio.parse().map_err(|_| invalido())
    }
}

/// Corta la primera palabra de `texto` y devuelve (palabra, resto sin espacios iniciales).
fn palabra(texto: &str) -> (&str, &str) {
    let texto = texto.trim_start();
    match texto.find(char::is_whitespace) {
        Some(i) => (&texto[..i], texto[i..].trim_start()),
        None => (texto, ""),
    }
}

/// Lee un texto entre comillas dobles con `\n`, `\t`, `\r`, `\\`, `\"` y `\0`; devuelve los bytes
/// y lo que quede despues de la comilla de cierre.
fn texto_entre_comillas(linea: usize, resto: &str) -> Result<(Vec<u8>, &str), ErrorGuion> {
    let Some(cuerpo) = resto.strip_prefix('"') else {
        return Err(ErrorGuion::TextoSinComillas { linea });
    };
    let mut bytes = Vec::new();
    let mut caracteres = cuerpo.char_indices();
    while let Some((i, c)) = caracteres.next() {
        match c {
            '"' => return Ok((bytes, &cuerpo[i + 1..])),
            '\\' => {
                let Some((_, e)) = caracteres.next() else {
                    return Err(ErrorGuion::TextoSinCerrar { linea });
                };
                let real = match e {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '0' => '\0',
                    '\\' => '\\',
                    '"' => '"',
                    otro => {
                        return Err(ErrorGuion::EscapeInvalido {
                            linea,
                            caracter: otro,
                        })
                    }
                };
                let mut buf = [0u8; 4];
                bytes.extend_from_slice(real.encode_utf8(&mut buf).as_bytes());
            }
            otro => {
                let mut buf = [0u8; 4];
                bytes.extend_from_slice(otro.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    Err(ErrorGuion::TextoSinCerrar { linea })
}

/// Quita un comentario `#` que no este dentro de un texto entre comillas.
fn sin_comentario(texto: &str) -> &str {
    let mut en_texto = false;
    let mut escapado = false;
    for (i, c) in texto.char_indices() {
        match c {
            '\\' if en_texto => escapado = !escapado,
            '"' if !escapado => en_texto = !en_texto,
            '#' if !en_texto => return &texto[..i],
            _ => escapado = false,
        }
        if c != '\\' {
            escapado = false;
        }
    }
    texto
}

/// Lee un guion. Los eventos salen en el orden en que estan escritos.
pub fn leer_guion(texto: &str) -> Result<Vec<Evento>, ErrorGuion> {
    let mut eventos = Vec::new();
    for (n, linea_completa) in texto.lines().enumerate() {
        let linea = n + 1;
        let contenido = sin_comentario(linea_completa).trim();
        if contenido.is_empty() {
            continue;
        }
        let (momento, resto) = palabra(contenido);
        let Some(orden) = momento.strip_prefix('@') else {
            return Err(ErrorGuion::SinMomento {
                linea,
                texto: momento.to_string(),
            });
        };
        let instruccion = numero(linea, orden)?;
        if resto.is_empty() {
            return Err(ErrorGuion::SinAccion {
                linea,
                momento: momento.to_string(),
            });
        }
        let (accion, resto) = palabra(resto);
        let sin_argumento = || ErrorGuion::SinArgumento {
            linea,
            accion: accion.to_string(),
        };
        let (accion, resto) = match accion {
            "teclado" => {
                let (valor, resto) = palabra(resto);
                if valor.is_empty() {
                    return Err(sin_argumento());
                }
                (Accion::Tecla(numero(linea, valor)?), resto)
            }
            "interrupcion" => {
                let (valor, resto) = palabra(resto);
                if valor.is_empty() {
                    return Err(sin_argumento());
                }
                let n = numero(linea, valor)?;
                let n = u32::try_from(n).map_err(|_| ErrorGuion::NumeroInvalido {
                    linea,
                    texto: valor.to_string(),
                })?;
                (Accion::Interrupcion(n), resto)
            }
            "serie" => {
                if resto.is_empty() {
                    return Err(sin_argumento());
                }
                let (bytes, resto) = texto_entre_comillas(linea, resto)?;
                (Accion::Serie(bytes), resto.trim())
            }
            otra => {
                return Err(ErrorGuion::AccionDesconocida {
                    linea,
                    accion: otra.to_string(),
                });
            }
        };
        if !resto.is_empty() {
            return Err(ErrorGuion::Sobra {
                linea,
                texto: resto.to_string(),
            });
        }
        eventos.push(Evento {
            instruccion,
            accion,
        });
    }
    Ok(eventos)
}
