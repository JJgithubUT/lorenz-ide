//! Analizador lexico: convierte el texto de una semantica en una secuencia de lexemas.

/// Una pieza minima del texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Numero(u64),
    Ident(String),
    // palabras reservadas
    Si,
    Detener,
    // aritmeticos
    Mas,
    Menos,
    Estrella,
    Barra,
    Porcentaje,
    // bits
    DesplazaIzq,
    DesplazaDer,
    Ampersand,
    Tuberia,
    Circunflejo,
    Tilde,
    // logicos y comparacion
    Admiracion,
    AmpAmp,
    TuberiaTuberia,
    IgualIgual,
    Distinto,
    Menor,
    MenorIgual,
    Mayor,
    MayorIgual,
    // puntuacion
    Igual,
    PuntoYComa,
    DosPuntos,
    ParenAbre,
    ParenCierra,
    CorcheteAbre,
    CorcheteCierra,
    LlaveAbre,
    LlaveCierra,
    /// Fin del texto.
    Fin,
}

/// Un token con el caracter donde empieza.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lexema {
    pub token: Token,
    pub posicion: usize,
}

use super::error::ErrorSemantica;

/// Divide el texto en lexemas. El ultimo es siempre [`Token::Fin`].
pub fn tokenizar(texto: &str) -> Result<Vec<Lexema>, ErrorSemantica> {
    let c: Vec<char> = texto.chars().collect();
    let mut salida = Vec::new();
    let mut i = 0usize;

    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }

        if ch.is_ascii_digit() {
            let inicio = i;
            let (valor, siguiente) = leer_numero(&c, i)?;
            salida.push(Lexema { token: Token::Numero(valor), posicion: inicio });
            i = siguiente;
            continue;
        }

        if ch.is_alphabetic() || ch == '_' {
            let inicio = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            let palabra: String = c[inicio..i].iter().collect();
            let token = match palabra.as_str() {
                "si" => Token::Si,
                "detener" => Token::Detener,
                _ => Token::Ident(palabra),
            };
            salida.push(Lexema { token, posicion: inicio });
            continue;
        }

        return Err(ErrorSemantica::nuevo(i, format!("caracter '{ch}' inesperado")));
    }

    salida.push(Lexema { token: Token::Fin, posicion: c.len() });
    Ok(salida)
}

/// Lee un literal numerico: decimal, `0x` hexadecimal o `0b` binario. Admite `_` como separador.
fn leer_numero(c: &[char], inicio: usize) -> Result<(u64, usize), ErrorSemantica> {
    let mut i = inicio;
    let (base, digitos_desde) = if c[i] == '0' && i + 1 < c.len() && (c[i + 1] == 'x' || c[i + 1] == 'X') {
        (16u32, i + 2)
    } else if c[i] == '0' && i + 1 < c.len() && (c[i + 1] == 'b' || c[i + 1] == 'B') {
        (2u32, i + 2)
    } else {
        (10u32, i)
    };
    i = digitos_desde;

    let mut valor: u64 = 0;
    let mut hubo_digitos = false;
    while i < c.len() && (c[i].is_digit(base) || c[i] == '_') {
        if c[i] != '_' {
            let d = c[i].to_digit(base).unwrap_or(0) as u64;
            valor = valor
                .checked_mul(base as u64)
                .and_then(|v| v.checked_add(d))
                .ok_or_else(|| ErrorSemantica::nuevo(inicio, "el numero no cabe en 64 bits"))?;
            hubo_digitos = true;
        }
        i += 1;
    }

    if !hubo_digitos {
        return Err(ErrorSemantica::nuevo(inicio, "literal numerico sin digitos"));
    }
    if i < c.len() && c[i].is_alphanumeric() {
        return Err(ErrorSemantica::nuevo(
            i,
            format!("el caracter '{}' no es un digito valido en este numero", c[i]),
        ));
    }
    Ok((valor, i))
}
