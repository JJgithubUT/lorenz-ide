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
        Ok(Self {
            lexemas: tokenizar(texto)?,
            indice: 0,
        })
    }

    fn actual(&self) -> &Token {
        &self.lexemas[self.indice].token
    }

    fn posicion(&self) -> usize {
        self.lexemas[self.indice].posicion
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
                format!(
                    "se esperaba {descripcion} y se encontro {}",
                    describir(self.actual())
                ),
            ))
        }
    }

    // ---- sentencias ----

    /// Lee sentencias separadas por `;` hasta encontrar `cierre` (que no consume).
    fn lista(&mut self, cierre: &Token) -> Resultado<Vec<Sentencia>> {
        let mut sentencias = Vec::new();
        loop {
            while *self.actual() == Token::PuntoYComa {
                self.avanzar();
            }
            if self.actual() == cierre {
                break;
            }
            sentencias.push(self.sentencia()?);
            if *self.actual() == Token::PuntoYComa {
                continue;
            }
            if self.actual() == cierre {
                break;
            }
            return Err(ErrorSemantica::nuevo(
                self.posicion(),
                format!(
                    "se esperaba ';' o el final y se encontro {}",
                    describir(self.actual())
                ),
            ));
        }
        Ok(sentencias)
    }

    fn sentencia(&mut self) -> Resultado<Sentencia> {
        match self.actual().clone() {
            Token::Detener => {
                self.avanzar();
                Ok(Sentencia::Detener)
            }
            Token::Si => {
                self.avanzar();
                let condicion = self.expresion()?;
                self.esperar(Token::DosPuntos, "':' despues de la condicion")?;
                let cuerpo = if *self.actual() == Token::LlaveAbre {
                    self.avanzar();
                    let interior = self.lista(&Token::LlaveCierra)?;
                    self.esperar(Token::LlaveCierra, "'}'")?;
                    interior
                } else {
                    vec![self.sentencia()?]
                };
                Ok(Sentencia::Si(condicion, cuerpo))
            }
            Token::Ident(_) => self.asignacion(),
            otro => Err(ErrorSemantica::nuevo(
                self.posicion(),
                format!(
                    "se esperaba una sentencia y se encontro {}",
                    describir(&otro)
                ),
            )),
        }
    }

    fn asignacion(&mut self) -> Resultado<Sentencia> {
        let nombre = match self.actual().clone() {
            Token::Ident(n) => n,
            _ => unreachable!("asignacion solo se llama sobre un identificador"),
        };
        self.avanzar();

        let destino =
            if (nombre == "mem" || nombre == "out") && *self.actual() == Token::CorcheteAbre {
                self.avanzar();
                let interior = self.expresion()?;
                self.esperar(Token::CorcheteCierra, "']'")?;
                if nombre == "mem" {
                    Destino::Mem(interior)
                } else {
                    Destino::Salida(interior)
                }
            } else {
                Destino::Nombre(nombre)
            };

        self.esperar(Token::Igual, "'=' en la asignacion")?;
        let valor = self.expresion()?;
        Ok(Sentencia::Asignar(destino, valor))
    }

    // ---- expresiones ----

    /// Punto de entrada de las expresiones: empieza por el nivel de menor precedencia.
    fn expresion(&mut self) -> Resultado<Expr> {
        self.nivel(NIVEL_MENOR)
    }

    /// Precedencia por niveles, de menor (1) a mayor (9), con asociatividad a la izquierda:
    ///
    /// | Nivel | Operadores |
    /// | --- | --- |
    /// | 1 | `\|\|` |
    /// | 2 | `&&` |
    /// | 3 | `==` `!=` `<` `<=` `>` `>=` |
    /// | 4 | `\|` |
    /// | 5 | `^` |
    /// | 6 | `&` |
    /// | 7 | `<<` `>>` |
    /// | 8 | `+` `-` |
    /// | 9 | `*` `/` `%` |
    fn nivel(&mut self, nivel: u8) -> Resultado<Expr> {
        if nivel > NIVEL_MAYOR {
            return self.unario();
        }
        let mut izquierda = self.nivel(nivel + 1)?;
        while let Some(op) = operador_del_nivel(self.actual(), nivel) {
            self.avanzar();
            let derecha = self.nivel(nivel + 1)?;
            izquierda = Expr::Binario(op, Box::new(izquierda), Box::new(derecha));
        }
        Ok(izquierda)
    }

    fn unario(&mut self) -> Resultado<Expr> {
        let op = match self.actual() {
            Token::Admiracion => Some(OpUnario::NegacionLogica),
            Token::Tilde => Some(OpUnario::NegacionBits),
            Token::Menos => Some(OpUnario::Menos),
            _ => None,
        };
        match op {
            Some(op) => {
                self.avanzar();
                let operando = self.unario()?;
                Ok(Expr::Unario(op, Box::new(operando)))
            }
            None => self.primario(),
        }
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

const NIVEL_MENOR: u8 = 1;
const NIVEL_MAYOR: u8 = 9;

/// Operador binario que corresponde a un token en un nivel de precedencia dado.
fn operador_del_nivel(t: &Token, nivel: u8) -> Option<OpBinario> {
    let op = match (nivel, t) {
        (1, Token::TuberiaTuberia) => OpBinario::OLogico,
        (2, Token::AmpAmp) => OpBinario::YLogico,
        (3, Token::IgualIgual) => OpBinario::Igual,
        (3, Token::Distinto) => OpBinario::Distinto,
        (3, Token::Menor) => OpBinario::Menor,
        (3, Token::MenorIgual) => OpBinario::MenorIgual,
        (3, Token::Mayor) => OpBinario::Mayor,
        (3, Token::MayorIgual) => OpBinario::MayorIgual,
        (4, Token::Tuberia) => OpBinario::O,
        (5, Token::Circunflejo) => OpBinario::Xor,
        (6, Token::Ampersand) => OpBinario::Y,
        (7, Token::DesplazaIzq) => OpBinario::DesplazaIzq,
        (7, Token::DesplazaDer) => OpBinario::DesplazaDer,
        (8, Token::Mas) => OpBinario::Suma,
        (8, Token::Menos) => OpBinario::Resta,
        (9, Token::Estrella) => OpBinario::Producto,
        (9, Token::Barra) => OpBinario::Division,
        (9, Token::Porcentaje) => OpBinario::Modulo,
        _ => return None,
    };
    Some(op)
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

/// Analiza la semantica completa de una instruccion.
///
/// Un texto vacio es valido y produce un programa sin sentencias.
pub fn analizar(texto: &str) -> Resultado<Programa> {
    let mut a = Analizador::nuevo(texto)?;
    let sentencias = a.lista(&Token::Fin)?;
    a.esperar(Token::Fin, "el final del texto")?;
    Ok(Programa { sentencias })
}
