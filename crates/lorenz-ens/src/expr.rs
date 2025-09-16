//! Expresiones constantes de los operandos y las directivas (RF-ENS-005, RF-ENS-006).
//!
//! Se analizan una vez a un arbol y se evaluan las veces que haga falta: en la primera pasada con
//! los simbolos conocidos hasta ese punto y en la segunda con todos.
//!
//! Literales: decimal (`42`), hexadecimal (`0x2A`, `$2A`), binario (`0b101010`) y caracter (`'*'`,
//! con escapes `\n \t \r \0 \\ \'`). `$` solo, o `.`, es la direccion de la instruccion actual.
//! Operadores, de menor a mayor precedencia: `|`, `^`, `&`, `<< >>`, `+ -`, `* / %`, y los
//! unarios `- + ~`.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Numero(i128),
    Simbolo(String),
    /// La direccion donde se esta ensamblando.
    Aqui,
    Unario(char, Box<Expr>),
    Binario(Op, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Suma,
    Resta,
    Producto,
    Cociente,
    Resto,
    DesplIzq,
    DesplDer,
    Y,
    O,
    Xor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorExpr {
    /// El texto no es una expresion valida.
    Sintaxis(String),
    SimboloIndefinido(String),
    DivisionEntreCero,
    /// El resultado no cabe en 128 bits.
    Desborde,
}

impl fmt::Display for ErrorExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorExpr::Sintaxis(m) => write!(f, "{m}"),
            ErrorExpr::SimboloIndefinido(s) => write!(f, "el simbolo `{s}` no esta definido"),
            ErrorExpr::DivisionEntreCero => write!(f, "division entre cero en una expresion"),
            ErrorExpr::Desborde => write!(f, "el resultado de la expresion es demasiado grande"),
        }
    }
}

impl std::error::Error for ErrorExpr {}

/// Lo que una expresion necesita saber del programa que se esta ensamblando.
pub trait Entorno {
    fn simbolo(&self, nombre: &str) -> Option<i128>;
    fn aqui(&self) -> i128;
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ficha {
    Numero(i128),
    Ident(String),
    Aqui,
    Op(String),
    Abre,
    Cierra,
}

fn es_inicio_ident(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn es_parte_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.'
}

fn fichas(texto: &str) -> Result<Vec<Ficha>, ErrorExpr> {
    let c: Vec<char> = texto.chars().collect();
    let mut i = 0;
    let mut salida = Vec::new();
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit() {
            let inicio = i;
            while i < c.len() && (c[i].is_ascii_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            let t: String = c[inicio..i].iter().filter(|&&x| x != '_').collect();
            salida.push(Ficha::Numero(numero(&t)?));
        } else if ch == '$' {
            let inicio = i + 1;
            let mut j = inicio;
            while j < c.len() && c[j].is_ascii_hexdigit() {
                j += 1;
            }
            // `$` seguido de digitos hexadecimales es un literal; `$` solo es la direccion actual.
            if j > inicio && !(j < c.len() && es_parte_ident(c[j])) {
                let t: String = c[inicio..j].iter().collect();
                let v = i128::from_str_radix(&t, 16).map_err(|_| {
                    ErrorExpr::Sintaxis(format!("literal hexadecimal invalido `${t}`"))
                })?;
                salida.push(Ficha::Numero(v));
                i = j;
            } else {
                salida.push(Ficha::Aqui);
                i += 1;
            }
        } else if ch == '\'' {
            let (v, usados) = caracter(&c[i..])?;
            salida.push(Ficha::Numero(v));
            i += usados;
        } else if es_inicio_ident(ch)
            || ch == '.' && c.get(i + 1).is_some_and(|x| es_inicio_ident(*x))
        {
            let inicio = i;
            i += 1;
            while i < c.len() && es_parte_ident(c[i]) {
                i += 1;
            }
            salida.push(Ficha::Ident(c[inicio..i].iter().collect()));
        } else if ch == '.' {
            salida.push(Ficha::Aqui);
            i += 1;
        } else if ch == '(' {
            salida.push(Ficha::Abre);
            i += 1;
        } else if ch == ')' {
            salida.push(Ficha::Cierra);
            i += 1;
        } else if (ch == '<' || ch == '>') && c.get(i + 1) == Some(&ch) {
            salida.push(Ficha::Op(format!("{ch}{ch}")));
            i += 2;
        } else if "+-*/%&|^~".contains(ch) {
            salida.push(Ficha::Op(ch.to_string()));
            i += 1;
        } else {
            return Err(ErrorExpr::Sintaxis(format!("caracter inesperado `{ch}`")));
        }
    }
    Ok(salida)
}

fn numero(t: &str) -> Result<i128, ErrorExpr> {
    let malo = || ErrorExpr::Sintaxis(format!("numero invalido `{t}`"));
    let minusculas = t.to_ascii_lowercase();
    if let Some(h) = minusculas.strip_prefix("0x") {
        i128::from_str_radix(h, 16).map_err(|_| malo())
    } else if let Some(b) = minusculas.strip_prefix("0b") {
        i128::from_str_radix(b, 2).map_err(|_| malo())
    } else {
        minusculas.parse::<i128>().map_err(|_| malo())
    }
}

/// Lee `'x'` o `'\x'`; devuelve el valor y cuantos caracteres uso.
fn caracter(c: &[char]) -> Result<(i128, usize), ErrorExpr> {
    let malo = || ErrorExpr::Sintaxis("literal de caracter mal formado".to_string());
    match c {
        ['\'', '\\', e, '\'', ..] => {
            let v = match e {
                'n' => 10,
                't' => 9,
                'r' => 13,
                '0' => 0,
                '\\' => 92,
                '\'' => 39,
                _ => return Err(malo()),
            };
            Ok((v, 4))
        }
        ['\'', x, '\'', ..] if *x != '\\' && *x != '\'' => Ok((*x as i128, 3)),
        _ => Err(malo()),
    }
}

struct Analizador {
    fichas: Vec<Ficha>,
    pos: usize,
}

fn nivel(op: &str) -> Option<(u8, Op)> {
    Some(match op {
        "|" => (1, Op::O),
        "^" => (2, Op::Xor),
        "&" => (3, Op::Y),
        "<<" => (4, Op::DesplIzq),
        ">>" => (4, Op::DesplDer),
        "+" => (5, Op::Suma),
        "-" => (5, Op::Resta),
        "*" => (6, Op::Producto),
        "/" => (6, Op::Cociente),
        "%" => (6, Op::Resto),
        _ => return None,
    })
}

impl Analizador {
    fn expr(&mut self, minimo: u8) -> Result<Expr, ErrorExpr> {
        let mut izq = self.unario()?;
        while let Some(Ficha::Op(o)) = self.fichas.get(self.pos) {
            let Some((prec, op)) = nivel(o) else { break };
            if prec < minimo {
                break;
            }
            self.pos += 1;
            let der = self.expr(prec + 1)?;
            izq = Expr::Binario(op, Box::new(izq), Box::new(der));
        }
        Ok(izq)
    }

    fn unario(&mut self) -> Result<Expr, ErrorExpr> {
        match self.fichas.get(self.pos).cloned() {
            Some(Ficha::Op(o)) if o == "-" || o == "+" || o == "~" => {
                self.pos += 1;
                let e = self.unario()?;
                Ok(Expr::Unario(o.chars().next().unwrap(), Box::new(e)))
            }
            Some(Ficha::Numero(n)) => {
                self.pos += 1;
                Ok(Expr::Numero(n))
            }
            Some(Ficha::Ident(s)) => {
                self.pos += 1;
                Ok(Expr::Simbolo(s))
            }
            Some(Ficha::Aqui) => {
                self.pos += 1;
                Ok(Expr::Aqui)
            }
            Some(Ficha::Abre) => {
                self.pos += 1;
                let e = self.expr(1)?;
                match self.fichas.get(self.pos) {
                    Some(Ficha::Cierra) => {
                        self.pos += 1;
                        Ok(e)
                    }
                    _ => Err(ErrorExpr::Sintaxis(
                        "falta cerrar el parentesis".to_string(),
                    )),
                }
            }
            Some(Ficha::Cierra) => Err(ErrorExpr::Sintaxis(
                "parentesis de cierre sin abrir".to_string(),
            )),
            Some(Ficha::Op(o)) => Err(ErrorExpr::Sintaxis(format!(
                "el operador `{o}` necesita un operando antes"
            ))),
            None => Err(ErrorExpr::Sintaxis(
                "la expresion esta incompleta".to_string(),
            )),
        }
    }
}

impl Expr {
    /// Analiza el texto completo de una expresion.
    pub fn analizar(texto: &str) -> Result<Expr, ErrorExpr> {
        let fichas = fichas(texto)?;
        if fichas.is_empty() {
            return Err(ErrorExpr::Sintaxis("la expresion esta vacia".to_string()));
        }
        let mut a = Analizador { fichas, pos: 0 };
        let e = a.expr(1)?;
        if a.pos != a.fichas.len() {
            return Err(ErrorExpr::Sintaxis(
                "sobra texto despues de la expresion".to_string(),
            ));
        }
        Ok(e)
    }

    pub fn evaluar(&self, entorno: &dyn Entorno) -> Result<i128, ErrorExpr> {
        match self {
            Expr::Numero(n) => Ok(*n),
            Expr::Simbolo(s) => entorno
                .simbolo(s)
                .ok_or_else(|| ErrorExpr::SimboloIndefinido(s.clone())),
            Expr::Aqui => Ok(entorno.aqui()),
            Expr::Unario(op, e) => {
                let v = e.evaluar(entorno)?;
                match op {
                    '-' => v.checked_neg().ok_or(ErrorExpr::Desborde),
                    '~' => Ok(!v),
                    _ => Ok(v),
                }
            }
            Expr::Binario(op, a, b) => {
                let (x, y) = (a.evaluar(entorno)?, b.evaluar(entorno)?);
                match op {
                    Op::Suma => x.checked_add(y).ok_or(ErrorExpr::Desborde),
                    Op::Resta => x.checked_sub(y).ok_or(ErrorExpr::Desborde),
                    Op::Producto => x.checked_mul(y).ok_or(ErrorExpr::Desborde),
                    Op::Cociente => {
                        if y == 0 {
                            Err(ErrorExpr::DivisionEntreCero)
                        } else {
                            x.checked_div(y).ok_or(ErrorExpr::Desborde)
                        }
                    }
                    Op::Resto => {
                        if y == 0 {
                            Err(ErrorExpr::DivisionEntreCero)
                        } else {
                            x.checked_rem(y).ok_or(ErrorExpr::Desborde)
                        }
                    }
                    Op::DesplIzq => {
                        let n = u32::try_from(y).map_err(|_| ErrorExpr::Desborde)?;
                        x.checked_shl(n)
                            .filter(|r| n < 127 && r >> n == x)
                            .ok_or(ErrorExpr::Desborde)
                    }
                    Op::DesplDer => {
                        let n = u32::try_from(y).map_err(|_| ErrorExpr::Desborde)?;
                        Ok(if n >= 127 {
                            if x < 0 {
                                -1
                            } else {
                                0
                            }
                        } else {
                            x >> n
                        })
                    }
                    Op::Y => Ok(x & y),
                    Op::O => Ok(x | y),
                    Op::Xor => Ok(x ^ y),
                }
            }
        }
    }

    /// Simbolos que la expresion menciona, sin repetir.
    pub fn simbolos(&self) -> Vec<&str> {
        let mut v = Vec::new();
        self.recoger(&mut v);
        v
    }

    fn recoger<'a>(&'a self, v: &mut Vec<&'a str>) {
        match self {
            Expr::Simbolo(s) if !v.contains(&s.as_str()) => v.push(s),
            Expr::Unario(_, e) => e.recoger(v),
            Expr::Binario(_, a, b) => {
                a.recoger(v);
                b.recoger(v);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use std::collections::HashMap;

    struct Prueba(HashMap<&'static str, i128>, i128);

    impl Entorno for Prueba {
        fn simbolo(&self, n: &str) -> Option<i128> {
            self.0.get(n).copied()
        }
        fn aqui(&self) -> i128 {
            self.1
        }
    }

    fn ev(t: &str) -> Result<i128, ErrorExpr> {
        let e = Prueba(HashMap::from([("x", 10), ("fin", 0x40), ("a.b", 7)]), 0x100);
        Expr::analizar(t)?.evaluar(&e)
    }

    #[test]
    fn literales_en_cuatro_bases() {
        assert_eq!(ev("42"), Ok(42));
        assert_eq!(ev("0x2A"), Ok(42));
        assert_eq!(ev("$2a"), Ok(42));
        assert_eq!(ev("0b101010"), Ok(42));
        assert_eq!(ev("'*'"), Ok(42));
        assert_eq!(ev("1_000"), Ok(1000));
    }

    #[test]
    fn escapes_de_caracter() {
        assert_eq!(ev(r"'\n'"), Ok(10));
        assert_eq!(ev(r"'\0'"), Ok(0));
        assert_eq!(ev(r"'\\'"), Ok(92));
        assert_eq!(ev(r"'\''"), Ok(39));
        assert!(ev(r"'\q'").is_err());
        assert!(ev("'ab'").is_err());
    }

    #[test]
    fn precedencia_y_parentesis() {
        assert_eq!(ev("2 + 3 * 4"), Ok(14));
        assert_eq!(ev("(2 + 3) * 4"), Ok(20));
        assert_eq!(ev("1 << 4 | 3"), Ok(19));
        assert_eq!(ev("0xFF & 0x0F ^ 0x01"), Ok(0x0E));
        assert_eq!(ev("10 - 3 - 2"), Ok(5));
        assert_eq!(ev("17 % 5 + 17 / 5"), Ok(5));
    }

    #[test]
    fn unarios() {
        assert_eq!(ev("-x"), Ok(-10));
        assert_eq!(ev("~0 & 0xFF"), Ok(0xFF));
        assert_eq!(ev("- -3"), Ok(3));
        assert_eq!(ev("+5"), Ok(5));
    }

    #[test]
    fn simbolos_y_direccion_actual() {
        assert_eq!(ev("fin - x"), Ok(0x36));
        assert_eq!(ev("$ + 2"), Ok(0x102));
        assert_eq!(ev(". + 2"), Ok(0x102));
        assert_eq!(ev("a.b * 2"), Ok(14));
        assert_eq!(ev("$ - fin"), Ok(0xC0));
    }

    #[test]
    fn errores_claros() {
        assert_eq!(ev("nada"), Err(ErrorExpr::SimboloIndefinido("nada".into())));
        assert_eq!(ev("1 / 0"), Err(ErrorExpr::DivisionEntreCero));
        assert_eq!(ev("1 % 0"), Err(ErrorExpr::DivisionEntreCero));
        assert!(matches!(ev("(1 + 2"), Err(ErrorExpr::Sintaxis(_))));
        assert!(matches!(ev("1 + 2)"), Err(ErrorExpr::Sintaxis(_))));
        assert!(matches!(ev("1 +"), Err(ErrorExpr::Sintaxis(_))));
        assert!(matches!(ev("* 2"), Err(ErrorExpr::Sintaxis(_))));
        assert!(matches!(ev(""), Err(ErrorExpr::Sintaxis(_))));
        assert!(matches!(ev("1 2"), Err(ErrorExpr::Sintaxis(_))));
        assert!(matches!(ev("1 @ 2"), Err(ErrorExpr::Sintaxis(_))));
        assert!(matches!(ev("0xZZ"), Err(ErrorExpr::Sintaxis(_))));
    }

    #[test]
    fn desbordes_se_detectan() {
        assert_eq!(ev("1 << 200"), Err(ErrorExpr::Desborde));
        assert_eq!(ev("1 << 126 << 1"), Err(ErrorExpr::Desborde));
        assert_eq!(ev("-1 >> 500"), Ok(-1));
        assert_eq!(ev("1 >> 500"), Ok(0));
    }

    #[test]
    fn lista_los_simbolos_mencionados() {
        let e = Expr::analizar("a + b * (a - c)").unwrap();
        assert_eq!(e.simbolos(), vec!["a", "b", "c"]);
    }
}
