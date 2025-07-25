//! Analizador sintactico de la semantica: descenso recursivo con precedencia de operadores.
//!
//! Gramatica:
//!
//! ```text
//! programa  := [ sentencia { ';' sentencia } [ ';' ] ]
//! sentencia := 'detener'
//!            | 'si' expr ':' cuerpo
//!            | destino '=' expr
//! cuerpo    := '{' programa '}' | sentencia
//! destino   := ident | 'mem' '[' expr ']' | 'out' '[' expr ']'
//! expr      := precedencia de menor a mayor: || && comparacion | ^ & << >> + - * / %
//! unario    := ( '!' | '~' | '-' ) unario | primario
//! primario  := numero | ident | 'mem' '[' expr ']' | 'in' '[' expr ']' | '(' expr ')'
//! ```

use super::ast::*;
use super::error::ErrorSemantica;
use super::lexico::{tokenizar, Lexema, Token};

struct Analizador {
    lexemas: Vec<Lexema>,
    indice: usize,
}

type Resultado<T> = Result<T, ErrorSemantica>;

impl Analizador {
    fn nuevo(texto: &str) -> Resultado<Self> {
        Ok(Self { lexemas: tokenizar(texto)?, indice: 0 })
    }

    fn actual(&self) -> &Token {
        &self.lexemas[self.indice].token
    }

    fn posicion(&self) -> usize {
        self.lexemas[self.indice].posicion
    }

    fn siguiente(&self) -> &Token {
        let i = (self.indice + 1).min(self.lexemas.len() - 1);
        &self.lexemas[i].token
    }

    fn avanzar(&mut self) {
        if self.indice < self.lexemas.len() - 1 {
            self.indice += 1;
        }
    }

    fn esperar(&mut self, esperado: Token, descripcion: &str) -> Resultado<()> {
        if *self.actual() == esperado {
            self.avanzar();
            Ok(())
        } else {
            Err(ErrorSemantica::nuevo(
                self.posicion(),
                format!("se esperaba {descripcion} y se encontro {}", describir(self.actual())),
            ))
        }
    }

    // ---- expresiones ----

    fn expresion(&mut self) -> Resultado<Expr> {
        self.primario()
    }

    fn primario(&mut self) -> Resultado<Expr> {
        let posicion = self.posicion();
        match self.actual().clone() {
            Token::Numero(n) => {
                self.avanzar();
                Ok(Expr::Literal(n))
            }
            Token::Ident(nombre) => {
                self.avanzar();
                if (nombre == "mem" || nombre == "in") && *self.actual() == Token::CorcheteAbre {
                    self.avanzar();
                    let interior = self.expresion()?;
                    self.esperar(Token::CorcheteCierra, "']'")?;
                    return Ok(if nombre == "mem" {
                        Expr::Mem(Box::new(interior))
                    } else {
                        Expr::Entrada(Box::new(interior))
                    });
                }
                Ok(Expr::Nombre(nombre))
            }
            Token::ParenAbre => {
                self.avanzar();
                let interior = self.expresion()?;
                self.esperar(Token::ParenCierra, "')'")?;
                Ok(interior)
            }
            otro => Err(ErrorSemantica::nuevo(
                posicion,
                format!("se esperaba un valor y se encontro {}", describir(&otro)),
            )),
        }
    }
}

/// Descripcion de un token para los mensajes de error.
fn describir(t: &Token) -> String {
    match t {
        Token::Numero(n) => format!("el numero {n}"),
        Token::Ident(s) => format!("'{s}'"),
        Token::Si => "'si'".into(),
        Token::Detener => "'detener'".into(),
        Token::Fin => "el final del texto".into(),
        Token::Mas => "'+'".into(),
        Token::Menos => "'-'".into(),
        Token::Estrella => "'*'".into(),
        Token::Barra => "'/'".into(),
        Token::Porcentaje => "'%'".into(),
        Token::DesplazaIzq => "'<<'".into(),
        Token::DesplazaDer => "'>>'".into(),
        Token::Ampersand => "'&'".into(),
        Token::Tuberia => "'|'".into(),
        Token::Circunflejo => "'^'".into(),
        Token::Tilde => "'~'".into(),
        Token::Admiracion => "'!'".into(),
        Token::AmpAmp => "'&&'".into(),
        Token::TuberiaTuberia => "'||'".into(),
        Token::IgualIgual => "'=='".into(),
        Token::Distinto => "'!='".into(),
        Token::Menor => "'<'".into(),
        Token::MenorIgual => "'<='".into(),
        Token::Mayor => "'>'".into(),
        Token::MayorIgual => "'>='".into(),
        Token::Igual => "'='".into(),
        Token::PuntoYComa => "';'".into(),
        Token::DosPuntos => "':'".into(),
        Token::ParenAbre => "'('".into(),
        Token::ParenCierra => "')'".into(),
        Token::CorcheteAbre => "'['".into(),
        Token::CorcheteCierra => "']'".into(),
        Token::LlaveAbre => "'{'".into(),
        Token::LlaveCierra => "'}'".into(),
    }
}

/// Analiza una sola expresion, sin sentencias.
pub fn analizar_expresion(texto: &str) -> Resultado<Expr> {
    let mut a = Analizador::nuevo(texto)?;
    let e = a.expresion()?;
    a.esperar(Token::Fin, "el final de la expresion")?;
    Ok(e)
}
