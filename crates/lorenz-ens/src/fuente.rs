//! Descomposicion de una linea de fuente en etiquetas, instruccion o directiva, y comentario.
//!
//! La gramatica de una linea es:
//!
//! ```text
//! [etiqueta:]... [ MNEMONICO operandos | .directiva argumentos | nombre = expresion ] [; comentario]
//! ```
//!
//! Aqui solo se separa el texto; reconocer los operandos de una instruccion es trabajo de
//! [`crate::plantilla`] y evaluar las expresiones, de [`crate::expr`].

use crate::error::Causa;

/// Un trozo de texto con la columna (desde 1) donde empieza.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texto {
    pub texto: String,
    pub columna: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sentencia {
    Instruccion {
        mnemonico: Texto,
        /// Todo lo que sigue al mnemonico, sin recortar el final.
        operandos: Texto,
    },
    Directiva {
        nombre: Texto,
        argumentos: Vec<Texto>,
    },
    /// `nombre = expresion`
    Constante { nombre: Texto, valor: Texto },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Linea {
    pub numero: usize,
    pub etiquetas: Vec<Texto>,
    pub sentencia: Option<Sentencia>,
}

fn es_inicio_ident(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn es_parte_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.'
}

/// Quita el comentario (`;` fuera de comillas) y devuelve lo que queda.
pub fn sin_comentario(linea: &str) -> &str {
    let mut comilla: Option<char> = None;
    let mut previo_barra = false;
    for (pos, c) in linea.char_indices() {
        match comilla {
            Some(q) => {
                if c == q && !previo_barra {
                    comilla = None;
                }
                previo_barra = c == '\\' && !previo_barra;
            }
            None => {
                if c == '\'' || c == '"' {
                    comilla = Some(c);
                    previo_barra = false;
                } else if c == ';' {
                    return &linea[..pos];
                }
            }
        }
    }
    linea
}

/// Divide por comas de nivel cero, respetando comillas y parentesis.
pub fn dividir_argumentos(texto: &str, columna_base: usize) -> Vec<Texto> {
    let mut partes = Vec::new();
    let mut nivel = 0i32;
    let mut comilla: Option<char> = None;
    let mut previo_barra = false;
    let mut inicio = 0usize;
    let cortar = |hasta: usize, inicio: usize, partes: &mut Vec<Texto>| {
        let trozo = &texto[inicio..hasta];
        let espacios = trozo.len() - trozo.trim_start().len();
        partes.push(Texto {
            texto: trozo.trim().to_string(),
            columna: columna_base + texto[..inicio + espacios].chars().count(),
        });
    };
    for (pos, c) in texto.char_indices() {
        match comilla {
            Some(q) => {
                if c == q && !previo_barra {
                    comilla = None;
                }
                previo_barra = c == '\\' && !previo_barra;
            }
            None => match c {
                '\'' | '"' => {
                    comilla = Some(c);
                    previo_barra = false;
                }
                '(' | '[' => nivel += 1,
                ')' | ']' => nivel -= 1,
                ',' if nivel == 0 => {
                    cortar(pos, inicio, &mut partes);
                    inicio = pos + 1;
                }
                _ => {}
            },
        }
    }
    if !texto.trim().is_empty() || !partes.is_empty() {
        cortar(texto.len(), inicio, &mut partes);
    }
    partes
}

/// Analiza una linea. `numero` empieza en 1. El error trae la columna donde se detecto.
pub fn analizar_linea(numero: usize, linea: &str) -> Result<Linea, (usize, Causa)> {
    let codigo = sin_comentario(linea);
    let col = |off: usize| codigo[..off].chars().count() + 1;
    let mut resultado = Linea {
        numero,
        ..Linea::default()
    };
    let mut pos = 0usize;
    loop {
        let resto = &codigo[pos..];
        let saltados = resto.len() - resto.trim_start().len();
        pos += saltados;
        let resto = &codigo[pos..];
        if resto.is_empty() {
            return Ok(resultado);
        }
        // Una etiqueta es un identificador pegado a `:`.
        let mut fin = 0usize;
        for (i, c) in resto.char_indices() {
            let valido = if i == 0 {
                es_inicio_ident(c)
            } else {
                es_parte_ident(c)
            };
            if !valido {
                break;
            }
            fin = i + c.len_utf8();
        }
        if fin > 0 && resto[fin..].starts_with(':') {
            resultado.etiquetas.push(Texto {
                texto: resto[..fin].to_string(),
                columna: col(pos),
            });
            pos += fin + 1;
            continue;
        }
        break;
    }
    let resto = &codigo[pos..];
    if resto.trim().is_empty() {
        return Ok(resultado);
    }
    let palabra_fin = resto
        .find(|c: char| c.is_whitespace() || c == '=')
        .unwrap_or(resto.len());
    let palabra = &resto[..palabra_fin];
    let despues = &resto[palabra_fin..];
    let despues_recortado = despues.trim_start();
    // `nombre = expresion`
    if despues_recortado.starts_with('=') && !despues_recortado.starts_with("==") {
        if palabra.is_empty() || !palabra.chars().next().is_some_and(es_inicio_ident) {
            return Err((
                col(pos),
                Causa::Sintaxis("falta el nombre antes de `=`".into()),
            ));
        }
        let tras_igual = &despues_recortado[1..];
        let desplazamiento = codigo.len() - tras_igual.len();
        let valor = tras_igual.trim();
        if valor.is_empty() {
            return Err((
                col(codigo.len()),
                Causa::Sintaxis(format!("falta el valor de la constante `{palabra}`")),
            ));
        }
        let espacios = tras_igual.len() - tras_igual.trim_start().len();
        resultado.sentencia = Some(Sentencia::Constante {
            nombre: Texto {
                texto: palabra.to_string(),
                columna: col(pos),
            },
            valor: Texto {
                texto: valor.to_string(),
                columna: col(desplazamiento + espacios),
            },
        });
        return Ok(resultado);
    }
    if palabra.is_empty() {
        return Err((
            col(pos),
            Causa::Sintaxis(format!(
                "no se esperaba `{}` aqui",
                resto.chars().next().unwrap_or(' ')
            )),
        ));
    }
    let desplazamiento_resto = codigo.len() - despues.len();
    if let Some(nombre) = palabra.strip_prefix('.') {
        if nombre.is_empty() {
            return Err((col(pos), Causa::Sintaxis("directiva sin nombre".into())));
        }
        let argumentos = dividir_argumentos(despues, col(desplazamiento_resto));
        resultado.sentencia = Some(Sentencia::Directiva {
            nombre: Texto {
                texto: nombre.to_string(),
                columna: col(pos),
            },
            argumentos,
        });
        return Ok(resultado);
    }
    if !palabra.chars().next().is_some_and(es_inicio_ident) {
        return Err((
            col(pos),
            Causa::Sintaxis(format!("`{palabra}` no puede ser un mnemonico")),
        ));
    }
    resultado.sentencia = Some(Sentencia::Instruccion {
        mnemonico: Texto {
            texto: palabra.to_string(),
            columna: col(pos),
        },
        operandos: Texto {
            texto: despues.trim_end().to_string(),
            columna: col(desplazamiento_resto),
        },
    });
    Ok(resultado)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn ok(t: &str) -> Linea {
        analizar_linea(1, t).unwrap()
    }

    #[test]
    fn linea_vacia_o_de_comentario() {
        assert_eq!(
            ok(""),
            Linea {
                numero: 1,
                ..Linea::default()
            }
        );
        assert!(ok("   ; solo un comentario").sentencia.is_none());
    }

    #[test]
    fn etiqueta_sola_y_con_instruccion() {
        let l = ok("inicio:");
        assert_eq!(l.etiquetas[0].texto, "inicio");
        assert!(l.sentencia.is_none());
        let l = ok("bucle: ADD A, B ; suma");
        assert_eq!(l.etiquetas[0].texto, "bucle");
        match l.sentencia.unwrap() {
            Sentencia::Instruccion {
                mnemonico,
                operandos,
            } => {
                assert_eq!(mnemonico.texto, "ADD");
                assert_eq!(mnemonico.columna, 8);
                assert_eq!(operandos.texto, " A, B");
            }
            otra => panic!("{otra:?}"),
        }
    }

    #[test]
    fn varias_etiquetas_en_una_linea() {
        let l = ok("a: b: HLT");
        assert_eq!(l.etiquetas.len(), 2);
        assert_eq!(l.etiquetas[1].texto, "b");
        assert_eq!(l.etiquetas[1].columna, 4);
    }

    #[test]
    fn el_punto_y_coma_dentro_de_comillas_no_comenta() {
        let l = ok(r#".db ';', "a;b" ; resto"#);
        match l.sentencia.unwrap() {
            Sentencia::Directiva { nombre, argumentos } => {
                assert_eq!(nombre.texto, "db");
                assert_eq!(argumentos.len(), 2);
                assert_eq!(argumentos[0].texto, "';'");
                assert_eq!(argumentos[1].texto, "\"a;b\"");
            }
            otra => panic!("{otra:?}"),
        }
    }

    #[test]
    fn constante_con_igual() {
        let l = ok("TAM = 4 * 8");
        match l.sentencia.unwrap() {
            Sentencia::Constante { nombre, valor } => {
                assert_eq!(nombre.texto, "TAM");
                assert_eq!(valor.texto, "4 * 8");
                assert_eq!(valor.columna, 7);
            }
            otra => panic!("{otra:?}"),
        }
    }

    #[test]
    fn constante_sin_valor_es_error() {
        assert!(analizar_linea(1, "X =").is_err());
        assert!(analizar_linea(1, "= 3").is_err());
    }

    #[test]
    fn argumentos_con_comas_dentro_de_parentesis() {
        let a = dividir_argumentos(" f(1,2), 3 ,  4", 1);
        assert_eq!(
            a.iter().map(|t| t.texto.as_str()).collect::<Vec<_>>(),
            ["f(1,2)", "3", "4"]
        );
        assert_eq!(a[2].columna, 15);
    }

    #[test]
    fn directiva_sin_argumentos() {
        match ok(".fin").sentencia.unwrap() {
            Sentencia::Directiva { argumentos, .. } => assert!(argumentos.is_empty()),
            otra => panic!("{otra:?}"),
        }
    }

    #[test]
    fn caracteres_raros_se_rechazan_con_columna() {
        let (col, _) = analizar_linea(1, "  @ A").unwrap_err();
        assert_eq!(col, 3);
        let (col, _) = analizar_linea(1, "1ADD A").unwrap_err();
        assert_eq!(col, 1);
    }
}
