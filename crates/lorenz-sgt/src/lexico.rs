//! Analizador lexico de SGT.
//!
//! Los comentarios (`// ...` y `/* ... */`) y los espacios no producen lexemas. Cada lexema lleva el
//! lugar donde empieza, para que todo diagnostico posterior pueda decir linea y columna.

use crate::diagnostico::{Diagnostico, Lugar};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Entero(u64),
    Cadena(String),
    Ident(String),

    // palabras reservadas
    Const,
    Var,
    Funcion,
    Estructura,
    Si,
    Sino,
    Mientras,
    Bucle,
    Romper,
    Continuar,
    Devolver,
    Segun,
    Otro,
    Como,
    Verdadero,
    Falso,
    Importar,
    Asm,

    // puntuacion
    ParenAbre,
    ParenCierra,
    LlaveAbre,
    LlaveCierra,
    CorcheteAbre,
    CorcheteCierra,
    Coma,
    PuntoYComa,
    DosPuntos,
    Punto,
    Flecha,
    FlechaDoble,
    Arroba,

    // operadores
    Mas,
    Menos,
    Estrella,
    Barra,
    Porcentaje,
    MasSat,
    MenosSat,
    Ampersand,
    Tuberia,
    Circunflejo,
    Tilde,
    DesplazaIzq,
    DesplazaDer,
    Admiracion,
    AmpAmp,
    TuberiaTuberia,
    IgualIgual,
    Distinto,
    Menor,
    MenorIgual,
    Mayor,
    MayorIgual,
    Igual,
    MasIgual,
    MenosIgual,
    EstrellaIgual,
    AmpersandIgual,
    TuberiaIgual,
    CircunflejoIgual,
    DesplazaIzqIgual,
    DesplazaDerIgual,

    Fin,
}

impl Token {
    /// Como se nombra un lexema en un mensaje de error.
    pub fn descripcion(&self) -> String {
        match self {
            Token::Entero(n) => format!("el numero {n}"),
            Token::Cadena(_) => "un texto".to_string(),
            Token::Ident(s) => format!("`{s}`"),
            Token::Fin => "el final del archivo".to_string(),
            otro => format!("`{}`", otro.texto()),
        }
    }

    fn texto(&self) -> &'static str {
        match self {
            Token::Const => "const",
            Token::Var => "var",
            Token::Funcion => "funcion",
            Token::Estructura => "estructura",
            Token::Si => "si",
            Token::Sino => "sino",
            Token::Mientras => "mientras",
            Token::Bucle => "bucle",
            Token::Romper => "romper",
            Token::Continuar => "continuar",
            Token::Devolver => "devolver",
            Token::Segun => "segun",
            Token::Otro => "otro",
            Token::Como => "como",
            Token::Verdadero => "verdadero",
            Token::Falso => "falso",
            Token::Importar => "importar",
            Token::Asm => "asm",
            Token::ParenAbre => "(",
            Token::ParenCierra => ")",
            Token::LlaveAbre => "{",
            Token::LlaveCierra => "}",
            Token::CorcheteAbre => "[",
            Token::CorcheteCierra => "]",
            Token::Coma => ",",
            Token::PuntoYComa => ";",
            Token::DosPuntos => ":",
            Token::Punto => ".",
            Token::Flecha => "->",
            Token::FlechaDoble => "=>",
            Token::Arroba => "@",
            Token::Mas => "+",
            Token::Menos => "-",
            Token::Estrella => "*",
            Token::Barra => "/",
            Token::Porcentaje => "%",
            Token::MasSat => "+|",
            Token::MenosSat => "-|",
            Token::Ampersand => "&",
            Token::Tuberia => "|",
            Token::Circunflejo => "^",
            Token::Tilde => "~",
            Token::DesplazaIzq => "<<",
            Token::DesplazaDer => ">>",
            Token::Admiracion => "!",
            Token::AmpAmp => "&&",
            Token::TuberiaTuberia => "||",
            Token::IgualIgual => "==",
            Token::Distinto => "!=",
            Token::Menor => "<",
            Token::MenorIgual => "<=",
            Token::Mayor => ">",
            Token::MayorIgual => ">=",
            Token::Igual => "=",
            Token::MasIgual => "+=",
            Token::MenosIgual => "-=",
            Token::EstrellaIgual => "*=",
            Token::AmpersandIgual => "&=",
            Token::TuberiaIgual => "|=",
            Token::CircunflejoIgual => "^=",
            Token::DesplazaIzqIgual => "<<=",
            Token::DesplazaDerIgual => ">>=",
            Token::Entero(_) | Token::Cadena(_) | Token::Ident(_) | Token::Fin => "",
        }
    }
}

/// Un lexema con el lugar donde empieza.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lexema {
    pub token: Token,
    pub lugar: Lugar,
}

fn palabra_reservada(s: &str) -> Option<Token> {
    Some(match s {
        "const" => Token::Const,
        "var" => Token::Var,
        "funcion" => Token::Funcion,
        "estructura" => Token::Estructura,
        "si" => Token::Si,
        "sino" => Token::Sino,
        "mientras" => Token::Mientras,
        "bucle" => Token::Bucle,
        "romper" => Token::Romper,
        "continuar" => Token::Continuar,
        "devolver" => Token::Devolver,
        "segun" => Token::Segun,
        "otro" => Token::Otro,
        "como" => Token::Como,
        "verdadero" => Token::Verdadero,
        "falso" => Token::Falso,
        "importar" => Token::Importar,
        "asm" => Token::Asm,
        _ => return None,
    })
}

struct Cursor<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    linea: u32,
    columna: u32,
}

impl Cursor<'_> {
    fn ver(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn ver2(&self) -> Option<char> {
        self.chars.clone().nth(1)
    }

    fn tomar(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        if c == '\n' {
            self.linea += 1;
            self.columna = 1;
        } else {
            self.columna += 1;
        }
        Some(c)
    }

    fn lugar(&self) -> Lugar {
        Lugar::nuevo(self.linea, self.columna)
    }
}

/// Convierte el texto de un archivo en lexemas; el ultimo siempre es [`Token::Fin`].
pub fn analizar(archivo: &str, texto: &str) -> Result<Vec<Lexema>, Diagnostico> {
    let mut c = Cursor {
        chars: texto.chars().peekable(),
        linea: 1,
        columna: 1,
    };
    let mut salida = Vec::new();
    let error = |lugar: Lugar, m: String| Diagnostico::nuevo(archivo, lugar, m);

    while let Some(ch) = c.ver() {
        let lugar = c.lugar();
        if ch.is_whitespace() {
            c.tomar();
            continue;
        }
        if ch == '/' && c.ver2() == Some('/') {
            while let Some(x) = c.ver() {
                if x == '\n' {
                    break;
                }
                c.tomar();
            }
            continue;
        }
        if ch == '/' && c.ver2() == Some('*') {
            c.tomar();
            c.tomar();
            let mut cerrado = false;
            while let Some(x) = c.tomar() {
                if x == '*' && c.ver() == Some('/') {
                    c.tomar();
                    cerrado = true;
                    break;
                }
            }
            if !cerrado {
                return Err(error(lugar, "comentario `/*` sin cerrar".into())
                    .con_sugerencia("cierralo con `*/`"));
            }
            continue;
        }
        if ch.is_ascii_digit() {
            salida.push(Lexema {
                token: Token::Entero(numero(&mut c, archivo, lugar)?),
                lugar,
            });
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let mut s = String::new();
            while let Some(x) = c.ver() {
                if x.is_ascii_alphanumeric() || x == '_' {
                    s.push(x);
                    c.tomar();
                } else {
                    break;
                }
            }
            let token = palabra_reservada(&s).unwrap_or(Token::Ident(s));
            salida.push(Lexema { token, lugar });
            continue;
        }
        if ch == '"' {
            c.tomar();
            let mut s = String::new();
            loop {
                match c.tomar() {
                    None | Some('\n') => {
                        return Err(error(lugar, "texto sin cerrar".into())
                            .con_sugerencia("cierralo con `\"` en la misma linea"))
                    }
                    Some('"') => break,
                    Some('\\') => match c.tomar() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('0') => s.push('\0'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        otro => {
                            return Err(error(
                                lugar,
                                format!(
                                    "secuencia de escape desconocida `\\{}`",
                                    otro.unwrap_or(' ')
                                ),
                            ))
                        }
                    },
                    Some(x) => s.push(x),
                }
            }
            salida.push(Lexema {
                token: Token::Cadena(s),
                lugar,
            });
            continue;
        }
        if ch == '\'' {
            c.tomar();
            let valor = match c.tomar() {
                Some('\\') => match c.tomar() {
                    Some('n') => '\n',
                    Some('t') => '\t',
                    Some('0') => '\0',
                    Some('\\') => '\\',
                    Some('\'') => '\'',
                    _ => return Err(error(lugar, "escape desconocido en el caracter".into())),
                },
                Some(x) if x != '\'' && x != '\n' => x,
                _ => return Err(error(lugar, "caracter mal escrito".into())),
            };
            if c.tomar() != Some('\'') {
                return Err(error(lugar, "caracter sin cerrar".into())
                    .con_sugerencia("un caracter se escribe entre comillas simples: 'a'"));
            }
            salida.push(Lexema {
                token: Token::Entero(u64::from(u32::from(valor))),
                lugar,
            });
            continue;
        }
        let token =
            simbolo(&mut c).ok_or_else(|| error(lugar, format!("caracter inesperado `{ch}`")))?;
        salida.push(Lexema { token, lugar });
    }
    salida.push(Lexema {
        token: Token::Fin,
        lugar: c.lugar(),
    });
    Ok(salida)
}

fn numero(c: &mut Cursor<'_>, archivo: &str, lugar: Lugar) -> Result<u64, Diagnostico> {
    let mut texto = String::new();
    let mut base = 10;
    if c.ver() == Some('0') && matches!(c.ver2(), Some('x' | 'X' | 'b' | 'B')) {
        c.tomar();
        base = if matches!(c.tomar(), Some('x' | 'X')) {
            16
        } else {
            2
        };
    }
    while let Some(x) = c.ver() {
        if x == '_' {
            c.tomar();
        } else if x.is_digit(base) {
            texto.push(x);
            c.tomar();
        } else if x.is_ascii_alphanumeric() {
            return Err(Diagnostico::nuevo(
                archivo,
                lugar,
                format!("`{x}` no es una cifra valida en base {base}"),
            ));
        } else {
            break;
        }
    }
    if texto.is_empty() {
        return Err(Diagnostico::nuevo(
            archivo,
            lugar,
            "falta el numero despues del prefijo",
        ));
    }
    u64::from_str_radix(&texto, base)
        .map_err(|_| Diagnostico::nuevo(archivo, lugar, "el numero no cabe en 64 bits"))
}

fn simbolo(c: &mut Cursor<'_>) -> Option<Token> {
    let a = c.tomar()?;
    let sig = c.ver();
    match (a, sig) {
        ('(', _) => Some(Token::ParenAbre),
        (')', _) => Some(Token::ParenCierra),
        ('{', _) => Some(Token::LlaveAbre),
        ('}', _) => Some(Token::LlaveCierra),
        ('[', _) => Some(Token::CorcheteAbre),
        (']', _) => Some(Token::CorcheteCierra),
        (',', _) => Some(Token::Coma),
        (';', _) => Some(Token::PuntoYComa),
        (':', _) => Some(Token::DosPuntos),
        ('.', _) => Some(Token::Punto),
        ('@', _) => Some(Token::Arroba),
        ('~', _) => Some(Token::Tilde),
        ('%', _) => Some(Token::Porcentaje),
        ('/', _) => Some(Token::Barra),
        ('-', Some('>')) => con(c, Token::Flecha),
        ('-', Some('|')) => con(c, Token::MenosSat),
        ('-', Some('=')) => con(c, Token::MenosIgual),
        ('-', _) => Some(Token::Menos),
        ('+', Some('|')) => con(c, Token::MasSat),
        ('+', Some('=')) => con(c, Token::MasIgual),
        ('+', _) => Some(Token::Mas),
        ('*', Some('=')) => con(c, Token::EstrellaIgual),
        ('*', _) => Some(Token::Estrella),
        ('&', Some('&')) => con(c, Token::AmpAmp),
        ('&', Some('=')) => con(c, Token::AmpersandIgual),
        ('&', _) => Some(Token::Ampersand),
        ('|', Some('|')) => con(c, Token::TuberiaTuberia),
        ('|', Some('=')) => con(c, Token::TuberiaIgual),
        ('|', _) => Some(Token::Tuberia),
        ('^', Some('=')) => con(c, Token::CircunflejoIgual),
        ('^', _) => Some(Token::Circunflejo),
        ('!', Some('=')) => con(c, Token::Distinto),
        ('!', _) => Some(Token::Admiracion),
        ('=', Some('=')) => con(c, Token::IgualIgual),
        ('=', Some('>')) => con(c, Token::FlechaDoble),
        ('=', _) => Some(Token::Igual),
        ('<', Some('<')) => {
            c.tomar();
            if c.ver() == Some('=') {
                con(c, Token::DesplazaIzqIgual)
            } else {
                Some(Token::DesplazaIzq)
            }
        }
        ('<', Some('=')) => con(c, Token::MenorIgual),
        ('<', _) => Some(Token::Menor),
        ('>', Some('>')) => {
            c.tomar();
            if c.ver() == Some('=') {
                con(c, Token::DesplazaDerIgual)
            } else {
                Some(Token::DesplazaDer)
            }
        }
        ('>', Some('=')) => con(c, Token::MayorIgual),
        ('>', _) => Some(Token::Mayor),
        _ => None,
    }
}

/// Consume el caracter que sigue y devuelve el lexema.
fn con(c: &mut Cursor<'_>, t: Token) -> Option<Token> {
    c.tomar();
    Some(t)
}
