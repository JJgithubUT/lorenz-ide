//! Condiciones de los puntos de ruptura (RF-DEP-013).
//!
//! Una condicion compara expresiones sobre el estado de la maquina:
//!
//! ```text
//! A == 5
//! mem[0x2000] != 0 && Z
//! ciclos >= 1000 || pc == fin
//! ```
//!
//! Las expresiones son las del ensamblador mas `nombre[indice]`. Se pueden usar registros, banderas,
//! `pc`, `sp`, `ciclos`, `instrucciones`, `mem[direccion]` y los simbolos del programa. Una
//! expresion sin comparador es verdadera si no vale cero. `&&` liga mas que `||`.

use lorenz_ens::expr::{Entorno, Expr};

use crate::error::ErrorDepurador;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Comparador {
    Igual,
    Distinto,
    Menor,
    MenorIgual,
    Mayor,
    MayorIgual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Nodo {
    O(Vec<Nodo>),
    Y(Vec<Nodo>),
    Compara(Expr, Comparador, Expr),
    Verdadera(Expr),
}

/// Una condicion ya analizada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condicion {
    texto: String,
    raiz: Nodo,
}

/// Divide por un operador de dos caracteres fuera de parentesis, corchetes y comillas.
fn dividir<'a>(texto: &'a str, operador: &str) -> Vec<&'a str> {
    let mut partes = Vec::new();
    let mut nivel = 0i32;
    let mut comilla = false;
    let mut inicio = 0;
    let mut chars = texto.char_indices().peekable();
    while let Some((pos, c)) = chars.next() {
        match c {
            '\'' => comilla = !comilla,
            '(' | '[' if !comilla => nivel += 1,
            ')' | ']' if !comilla => nivel -= 1,
            _ if !comilla && nivel == 0 && texto[pos..].starts_with(operador) => {
                partes.push(&texto[inicio..pos]);
                inicio = pos + operador.len();
                for _ in 1..operador.len() {
                    chars.next();
                }
            }
            _ => {}
        }
    }
    partes.push(&texto[inicio..]);
    partes
}

/// Busca el primer comparador fuera de parentesis y corchetes. Los desplazamientos `<<` y `>>` no
/// cuentan como comparadores.
fn buscar_comparador(texto: &str) -> Option<(usize, usize, Comparador)> {
    let b: Vec<(usize, char)> = texto.char_indices().collect();
    let mut nivel = 0i32;
    let mut comilla = false;
    let mut i = 0;
    while i < b.len() {
        let (pos, c) = b[i];
        let sig = b.get(i + 1).map(|x| x.1);
        match c {
            '\'' => comilla = !comilla,
            '(' | '[' if !comilla => nivel += 1,
            ')' | ']' if !comilla => nivel -= 1,
            '=' if !comilla && nivel == 0 && sig == Some('=') => {
                return Some((pos, 2, Comparador::Igual))
            }
            '!' if !comilla && nivel == 0 && sig == Some('=') => {
                return Some((pos, 2, Comparador::Distinto))
            }
            '<' if !comilla && nivel == 0 => {
                if sig == Some('<') {
                    i += 1;
                } else if sig == Some('=') {
                    return Some((pos, 2, Comparador::MenorIgual));
                } else {
                    return Some((pos, 1, Comparador::Menor));
                }
            }
            '>' if !comilla && nivel == 0 => {
                if sig == Some('>') {
                    i += 1;
                } else if sig == Some('=') {
                    return Some((pos, 2, Comparador::MayorIgual));
                } else {
                    return Some((pos, 1, Comparador::Mayor));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn expresion(texto: &str) -> Result<Expr, ErrorDepurador> {
    Expr::analizar(texto.trim()).map_err(|e| ErrorDepurador::Condicion(e.to_string()))
}

/// Si todo el texto esta entre un parentesis que abre al principio y cierra al final, devuelve lo
/// de dentro: permite agrupar condiciones, como en `B && (A || Z)`.
fn sin_parentesis_externos(texto: &str) -> &str {
    let t = texto.trim();
    if !(t.starts_with('(') && t.ends_with(')')) {
        return t;
    }
    let mut nivel = 0i32;
    for (pos, c) in t.char_indices() {
        match c {
            '(' => nivel += 1,
            ')' => {
                nivel -= 1;
                if nivel == 0 && pos != t.len() - 1 {
                    return t;
                }
            }
            _ => {}
        }
    }
    sin_parentesis_externos(&t[1..t.len() - 1])
}

fn analizar_nodo(texto: &str) -> Result<Nodo, ErrorDepurador> {
    let texto = sin_parentesis_externos(texto);
    let ors = dividir(texto, "||");
    if ors.len() > 1 {
        return Ok(Nodo::O(
            ors.into_iter()
                .map(analizar_nodo)
                .collect::<Result<_, _>>()?,
        ));
    }
    let ands = dividir(texto, "&&");
    if ands.len() > 1 {
        return Ok(Nodo::Y(
            ands.into_iter()
                .map(analizar_nodo)
                .collect::<Result<_, _>>()?,
        ));
    }
    match buscar_comparador(texto) {
        Some((pos, largo, c)) => Ok(Nodo::Compara(
            expresion(&texto[..pos])?,
            c,
            expresion(&texto[pos + largo..])?,
        )),
        None => Ok(Nodo::Verdadera(expresion(texto)?)),
    }
}

impl Condicion {
    pub fn analizar(texto: &str) -> Result<Self, ErrorDepurador> {
        if texto.trim().is_empty() {
            return Err(ErrorDepurador::Condicion("la condicion esta vacia".into()));
        }
        Ok(Condicion {
            texto: texto.trim().to_string(),
            raiz: analizar_nodo(texto)?,
        })
    }

    pub fn texto(&self) -> &str {
        &self.texto
    }

    /// Evalua la condicion. Un nombre desconocido es un error, no `falso`: una condicion mal escrita
    /// nunca se cumpliria y el usuario no sabria por que.
    pub fn evaluar(&self, entorno: &dyn Entorno) -> Result<bool, ErrorDepurador> {
        evaluar_nodo(&self.raiz, entorno)
    }
}

fn evaluar_nodo(n: &Nodo, e: &dyn Entorno) -> Result<bool, ErrorDepurador> {
    let valor = |x: &Expr| {
        x.evaluar(e)
            .map_err(|er| ErrorDepurador::Condicion(er.to_string()))
    };
    match n {
        Nodo::O(v) => {
            for x in v {
                if evaluar_nodo(x, e)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Nodo::Y(v) => {
            for x in v {
                if !evaluar_nodo(x, e)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        Nodo::Verdadera(x) => Ok(valor(x)? != 0),
        Nodo::Compara(a, c, b) => {
            let (a, b) = (valor(a)?, valor(b)?);
            Ok(match c {
                Comparador::Igual => a == b,
                Comparador::Distinto => a != b,
                Comparador::Menor => a < b,
                Comparador::MenorIgual => a <= b,
                Comparador::Mayor => a > b,
                Comparador::MayorIgual => a >= b,
            })
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use std::collections::HashMap;

    struct E(HashMap<&'static str, i128>);

    impl Entorno for E {
        fn simbolo(&self, n: &str) -> Option<i128> {
            self.0.get(n).copied()
        }
        fn aqui(&self) -> i128 {
            0
        }
        fn indice(&self, n: &str, i: i128) -> Option<i128> {
            (n == "mem").then_some(i + 100)
        }
    }

    fn entorno() -> E {
        E(HashMap::from([("A", 5), ("B", 0), ("Z", 1)]))
    }

    fn ev(t: &str) -> Result<bool, ErrorDepurador> {
        Condicion::analizar(t)?.evaluar(&entorno())
    }

    #[test]
    fn comparadores() {
        assert_eq!(ev("A == 5"), Ok(true));
        assert_eq!(ev("A != 5"), Ok(false));
        assert_eq!(ev("A < 6"), Ok(true));
        assert_eq!(ev("A <= 5"), Ok(true));
        assert_eq!(ev("A > 5"), Ok(false));
        assert_eq!(ev("A >= 5"), Ok(true));
    }

    #[test]
    fn sin_comparador_vale_si_no_es_cero() {
        assert_eq!(ev("A"), Ok(true));
        assert_eq!(ev("B"), Ok(false));
        assert_eq!(ev("Z"), Ok(true));
    }

    #[test]
    fn y_liga_mas_que_o() {
        assert_eq!(ev("B || A == 5 && Z"), Ok(true));
        assert_eq!(ev("B && A || Z"), Ok(true));
        assert_eq!(ev("B && (A || Z)"), Ok(false));
    }

    #[test]
    fn expresiones_en_ambos_lados() {
        assert_eq!(ev("A + 1 == 6"), Ok(true));
        assert_eq!(ev("A << 1 == 10"), Ok(true));
        assert_eq!(ev("A >> 1 == 2"), Ok(true));
        assert_eq!(ev("mem[2 * 3] == 106"), Ok(true));
    }

    #[test]
    fn un_nombre_desconocido_es_un_error() {
        assert!(matches!(ev("Q == 1"), Err(ErrorDepurador::Condicion(_))));
    }

    #[test]
    fn condiciones_mal_escritas() {
        assert!(Condicion::analizar("").is_err());
        assert!(Condicion::analizar("A ==").is_err());
        assert!(Condicion::analizar("== 3").is_err());
        assert!(Condicion::analizar("A && ").is_err());
    }

    #[test]
    fn el_caracter_entre_comillas_no_cuenta_como_comparador() {
        assert_eq!(ev("A == 5 && '=' == 61"), Ok(true));
    }

    #[test]
    fn guarda_el_texto_original() {
        assert_eq!(Condicion::analizar("  A == 5 ").unwrap().texto(), "A == 5");
    }
}
