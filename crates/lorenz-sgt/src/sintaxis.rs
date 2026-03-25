//! Analizador sintactico de SGT: de lexemas a arbol.
//!
//! Es un descenso recursivo con escalada de precedencia para las expresiones. Se detiene en el
//! primer error, con el lugar exacto y, cuando la causa es reconocible, una sugerencia.

use crate::ast::*;
use crate::diagnostico::{Diagnostico, Lugar};
use crate::lexico::{analizar, Lexema, Token};

/// Analiza un archivo completo.
pub fn analizar_programa(archivo: &str, texto: &str) -> Result<Programa, Diagnostico> {
    let lexemas = analizar(archivo, texto)?;
    let mut p = Parser {
        archivo: archivo.to_string(),
        lexemas,
        pos: 0,
    };
    let mut elementos = Vec::new();
    while p.ver() != &Token::Fin {
        elementos.push(p.elemento()?);
    }
    Ok(Programa {
        elementos,
        modulos: Vec::new(),
    })
}

/// Analiza una sola expresion; sirve para las pruebas y para las constantes de la linea de comandos.
pub fn analizar_expresion(archivo: &str, texto: &str) -> Result<Expr, Diagnostico> {
    let lexemas = analizar(archivo, texto)?;
    let mut p = Parser {
        archivo: archivo.to_string(),
        lexemas,
        pos: 0,
    };
    let e = p.expr()?;
    if p.ver() != &Token::Fin {
        return Err(p.inesperado("el final de la expresion"));
    }
    Ok(e)
}

struct Parser {
    archivo: String,
    lexemas: Vec<Lexema>,
    pos: usize,
}

type Res<T> = Result<T, Diagnostico>;

impl Parser {
    fn ver(&self) -> &Token {
        &self.lexemas[self.pos].token
    }

    fn lugar(&self) -> Lugar {
        self.lexemas[self.pos].lugar
    }

    fn lugar_anterior(&self) -> Lugar {
        self.lexemas[self.pos.saturating_sub(1)].lugar
    }

    fn avanzar(&mut self) -> Token {
        let t = self.lexemas[self.pos].token.clone();
        if self.pos + 1 < self.lexemas.len() {
            self.pos += 1;
        }
        t
    }

    fn si_es(&mut self, t: &Token) -> bool {
        if self.ver() == t {
            self.avanzar();
            true
        } else {
            false
        }
    }

    fn error(&self, lugar: Lugar, mensaje: impl Into<String>) -> Diagnostico {
        Diagnostico::nuevo(&self.archivo, lugar, mensaje)
    }

    fn inesperado(&self, esperaba: &str) -> Diagnostico {
        self.error(
            self.lugar(),
            format!(
                "se esperaba {esperaba} y se encontro {}",
                self.ver().descripcion()
            ),
        )
    }

    fn esperar(&mut self, t: Token, contexto: &str) -> Res<Lugar> {
        if self.ver() == &t {
            let l = self.lugar();
            self.avanzar();
            return Ok(l);
        }
        let nombre = t.descripcion();
        // Un `;` o un cierre que falta se senala donde termino lo anterior: es ahi donde se escribe.
        if matches!(
            t,
            Token::PuntoYComa | Token::ParenCierra | Token::LlaveCierra
        ) {
            let mut d = self
                .error(self.lugar_anterior(), format!("falta {nombre} {contexto}"))
                .con_sugerencia(format!("escribe {nombre} {contexto}"));
            if self.ver() == &Token::Fin {
                d.mensaje = format!("falta {nombre} {contexto} antes del final del archivo");
            }
            return Err(d);
        }
        Err(self.inesperado(&format!("{nombre} {contexto}")))
    }

    fn ident(&mut self, que: &str) -> Res<(String, Lugar)> {
        let lugar = self.lugar();
        match self.ver().clone() {
            Token::Ident(s) => {
                self.avanzar();
                Ok((s, lugar))
            }
            _ => Err(self.inesperado(que)),
        }
    }

    // --- elementos de primer nivel ---

    fn elemento(&mut self) -> Res<Elemento> {
        let lugar = self.lugar();
        let mut atributos = Vec::new();
        while self.ver() == &Token::Arroba {
            atributos.push(self.atributo()?);
        }
        match self.ver() {
            Token::Funcion => Ok(Elemento::Funcion(self.funcion(lugar, atributos)?)),
            _ if !atributos.is_empty() => Err(self.inesperado("`funcion` despues del atributo")),
            Token::Const => self.def_const(lugar),
            Token::Var => self.def_global(lugar),
            Token::Estructura => self.def_estructura(lugar),
            Token::Importar => self.importar(lugar),
            _ => Err(self.inesperado(
                "una declaracion (`funcion`, `const`, `var`, `estructura` o `importar`)",
            )),
        }
    }

    fn atributo(&mut self) -> Res<Atributo> {
        let lugar = self.lugar();
        self.esperar(Token::Arroba, "")?;
        let (nombre, _) = self.ident("el nombre del atributo")?;
        let mut argumentos = Vec::new();
        if self.si_es(&Token::ParenAbre) {
            if self.ver() != &Token::ParenCierra {
                loop {
                    argumentos.push(self.expr()?);
                    if !self.si_es(&Token::Coma) {
                        break;
                    }
                }
            }
            self.esperar(Token::ParenCierra, "al cerrar el atributo")?;
        }
        Ok(Atributo {
            lugar,
            nombre,
            argumentos,
        })
    }

    fn importar(&mut self, lugar: Lugar) -> Res<Elemento> {
        self.avanzar();
        let (modulo, _) = self.ident("el nombre del modulo")?;
        self.esperar(Token::LlaveAbre, "antes de la lista de simbolos")?;
        let mut simbolos = Vec::new();
        loop {
            simbolos.push(self.ident("un simbolo a importar")?.0);
            if !self.si_es(&Token::Coma) {
                break;
            }
            if self.ver() == &Token::LlaveCierra {
                break;
            }
        }
        self.esperar(Token::LlaveCierra, "al cerrar la lista de simbolos")?;
        self.esperar(Token::PuntoYComa, "al final de `importar`")?;
        Ok(Elemento::Importar(Importar {
            lugar,
            modulo,
            simbolos,
        }))
    }

    fn def_const(&mut self, lugar: Lugar) -> Res<Elemento> {
        self.avanzar();
        let (nombre, _) = self.ident("el nombre de la constante")?;
        self.esperar(Token::DosPuntos, "despues del nombre de la constante")?;
        let tipo = self.tipo()?;
        self.esperar(Token::Igual, "antes del valor de la constante")?;
        let valor = self.expr()?;
        self.esperar(Token::PuntoYComa, "al final de la constante")?;
        Ok(Elemento::Const(DefConst {
            lugar,
            nombre,
            tipo,
            valor,
        }))
    }

    fn def_global(&mut self, lugar: Lugar) -> Res<Elemento> {
        self.avanzar();
        let (nombre, _) = self.ident("el nombre de la variable")?;
        self.esperar(Token::DosPuntos, "despues del nombre de la variable")?;
        let tipo = self.tipo()?;
        let direccion = if self.si_es(&Token::Arroba) {
            Some(self.expr()?)
        } else {
            None
        };
        let inicial = if self.si_es(&Token::Igual) {
            Some(self.expr()?)
        } else {
            None
        };
        self.esperar(Token::PuntoYComa, "al final de la variable")?;
        Ok(Elemento::Global(DefGlobal {
            lugar,
            nombre,
            tipo,
            direccion,
            inicial,
        }))
    }

    fn def_estructura(&mut self, lugar: Lugar) -> Res<Elemento> {
        self.avanzar();
        let (nombre, _) = self.ident("el nombre de la estructura")?;
        self.esperar(Token::LlaveAbre, "antes de los campos")?;
        let mut campos = Vec::new();
        while self.ver() != &Token::LlaveCierra {
            let lugar = self.lugar();
            let (nombre, _) = self.ident("el nombre de un campo")?;
            self.esperar(Token::DosPuntos, "despues del nombre del campo")?;
            let tipo = self.tipo()?;
            let desplazamiento = if self.si_es(&Token::Arroba) {
                Some(self.expr()?)
            } else {
                None
            };
            campos.push(Campo {
                lugar,
                nombre,
                tipo,
                desplazamiento,
            });
            if !self.si_es(&Token::Coma) {
                break;
            }
        }
        self.esperar(Token::LlaveCierra, "al cerrar la estructura")?;
        Ok(Elemento::Estructura(DefEstructura {
            lugar,
            nombre,
            campos,
        }))
    }

    fn funcion(&mut self, lugar: Lugar, atributos: Vec<Atributo>) -> Res<DefFuncion> {
        self.avanzar();
        let (nombre, _) = self.ident("el nombre de la funcion")?;
        self.esperar(Token::ParenAbre, "despues del nombre de la funcion")?;
        let mut parametros = Vec::new();
        if self.ver() != &Token::ParenCierra {
            loop {
                let lugar = self.lugar();
                let (nombre, _) = self.ident("el nombre de un parametro")?;
                self.esperar(Token::DosPuntos, "despues del nombre del parametro")?;
                let tipo = self.tipo()?;
                parametros.push(Parametro {
                    lugar,
                    nombre,
                    tipo,
                });
                if !self.si_es(&Token::Coma) {
                    break;
                }
            }
        }
        self.esperar(Token::ParenCierra, "al cerrar los parametros")?;
        let retorno = if self.si_es(&Token::Flecha) {
            Some(self.tipo()?)
        } else {
            None
        };
        let cuerpo = self.bloque()?;
        Ok(DefFuncion {
            lugar,
            atributos,
            nombre,
            parametros,
            retorno,
            cuerpo,
        })
    }

    fn tipo(&mut self) -> Res<TipoAst> {
        let lugar = self.lugar();
        match self.ver().clone() {
            Token::Estrella => {
                self.avanzar();
                Ok(TipoAst {
                    lugar,
                    clase: ClaseTipo::Puntero(Box::new(self.tipo()?)),
                })
            }
            Token::CorcheteAbre => {
                self.avanzar();
                let elemento = self.tipo()?;
                self.esperar(Token::PuntoYComa, "entre el tipo y el tamano del arreglo")?;
                let largo = self.expr()?;
                self.esperar(Token::CorcheteCierra, "al cerrar el tipo del arreglo")?;
                Ok(TipoAst {
                    lugar,
                    clase: ClaseTipo::Arreglo(Box::new(elemento), Box::new(largo)),
                })
            }
            Token::Ident(s) => {
                self.avanzar();
                Ok(TipoAst {
                    lugar,
                    clase: ClaseTipo::Nombre(s),
                })
            }
            _ => Err(self.inesperado("un tipo")),
        }
    }

    // --- sentencias ---

    fn bloque(&mut self) -> Res<Bloque> {
        self.esperar(Token::LlaveAbre, "al abrir el bloque")?;
        let mut sentencias = Vec::new();
        while self.ver() != &Token::LlaveCierra {
            if self.ver() == &Token::Fin {
                return Err(self
                    .error(
                        self.lugar_anterior(),
                        "falta `}` antes del final del archivo",
                    )
                    .con_sugerencia("cierra el bloque con `}`"));
            }
            sentencias.push(self.sentencia()?);
        }
        self.avanzar();
        Ok(Bloque { sentencias })
    }

    fn sentencia(&mut self) -> Res<Sentencia> {
        let lugar = self.lugar();
        let clase = match self.ver().clone() {
            Token::Var => {
                self.avanzar();
                let (nombre, _) = self.ident("el nombre de la variable")?;
                let tipo = if self.si_es(&Token::DosPuntos) {
                    Some(self.tipo()?)
                } else {
                    None
                };
                let inicial = if self.si_es(&Token::Igual) {
                    Some(self.expr()?)
                } else {
                    None
                };
                if tipo.is_none() && inicial.is_none() {
                    return Err(self
                        .error(
                            lugar,
                            format!("`{nombre}` no tiene ni tipo ni valor inicial"),
                        )
                        .con_sugerencia(format!(
                            "escribe `var {nombre}: u8;` o `var {nombre} = 0;`"
                        )));
                }
                self.esperar(Token::PuntoYComa, "al final de la declaracion")?;
                ClaseSentencia::Var {
                    nombre,
                    tipo,
                    inicial,
                }
            }
            Token::Si => return self.si(),
            Token::Mientras => {
                self.avanzar();
                let condicion = self.expr()?;
                ClaseSentencia::Mientras {
                    condicion,
                    cuerpo: self.bloque()?,
                }
            }
            Token::Bucle => {
                self.avanzar();
                ClaseSentencia::Bucle(self.bloque()?)
            }
            Token::Romper => {
                self.avanzar();
                self.esperar(Token::PuntoYComa, "despues de `romper`")?;
                ClaseSentencia::Romper
            }
            Token::Continuar => {
                self.avanzar();
                self.esperar(Token::PuntoYComa, "despues de `continuar`")?;
                ClaseSentencia::Continuar
            }
            Token::Devolver => {
                self.avanzar();
                let valor = if self.ver() == &Token::PuntoYComa {
                    None
                } else {
                    Some(self.expr()?)
                };
                self.esperar(Token::PuntoYComa, "despues de `devolver`")?;
                ClaseSentencia::Devolver(valor)
            }
            Token::Segun => return self.segun(),
            Token::Asm => {
                self.avanzar();
                self.esperar(Token::ParenAbre, "despues de `asm`")?;
                let texto = match self.ver().clone() {
                    Token::Cadena(s) => {
                        self.avanzar();
                        s
                    }
                    _ => return Err(self.inesperado("el texto del ensamblador")),
                };
                self.esperar(Token::ParenCierra, "al cerrar `asm`")?;
                self.esperar(Token::PuntoYComa, "despues de `asm(...)`")?;
                ClaseSentencia::Asm(texto)
            }
            _ => {
                let e = self.expr()?;
                if let Some(op) = self.operador_de_asignacion() {
                    let valor = self.expr()?;
                    self.esperar(Token::PuntoYComa, "al final de la asignacion")?;
                    ClaseSentencia::Asignar {
                        destino: e,
                        op,
                        valor,
                    }
                } else {
                    self.esperar(Token::PuntoYComa, "al final de la sentencia")?;
                    ClaseSentencia::Expr(e)
                }
            }
        };
        Ok(Sentencia { lugar, clase })
    }

    /// Si el lexema actual es `=` o `op=`, lo consume y devuelve `Some(op)`.
    fn operador_de_asignacion(&mut self) -> Option<Option<OpBinario>> {
        let op = match self.ver() {
            Token::Igual => None,
            Token::MasIgual => Some(OpBinario::Suma),
            Token::MenosIgual => Some(OpBinario::Resta),
            Token::EstrellaIgual => Some(OpBinario::Mul),
            Token::AmpersandIgual => Some(OpBinario::Y),
            Token::TuberiaIgual => Some(OpBinario::O),
            Token::CircunflejoIgual => Some(OpBinario::Xor),
            Token::DesplazaIzqIgual => Some(OpBinario::DespIzq),
            Token::DesplazaDerIgual => Some(OpBinario::DespDer),
            _ => return None,
        };
        self.avanzar();
        Some(op)
    }

    fn si(&mut self) -> Res<Sentencia> {
        let lugar = self.lugar();
        self.avanzar();
        let condicion = self.expr()?;
        let entonces = self.bloque()?;
        let sino = if self.si_es(&Token::Sino) {
            if self.ver() == &Token::Si {
                let otro = self.si()?;
                Some(Bloque {
                    sentencias: vec![otro],
                })
            } else {
                Some(self.bloque()?)
            }
        } else {
            None
        };
        Ok(Sentencia {
            lugar,
            clase: ClaseSentencia::Si {
                condicion,
                entonces,
                sino,
            },
        })
    }

    fn segun(&mut self) -> Res<Sentencia> {
        let lugar = self.lugar();
        self.avanzar();
        let valor = self.expr()?;
        self.esperar(Token::LlaveAbre, "antes de los brazos de `segun`")?;
        let mut brazos = Vec::new();
        let mut otro = None;
        while self.ver() != &Token::LlaveCierra {
            if self.si_es(&Token::Otro) {
                self.esperar(Token::FlechaDoble, "despues de `otro`")?;
                otro = Some(self.bloque()?);
            } else {
                let v = self.expr()?;
                self.esperar(Token::FlechaDoble, "despues del valor del brazo")?;
                brazos.push((v, self.bloque()?));
            }
        }
        self.esperar(Token::LlaveCierra, "al cerrar `segun`")?;
        Ok(Sentencia {
            lugar,
            clase: ClaseSentencia::Segun {
                valor,
                brazos,
                otro,
            },
        })
    }

    // --- expresiones ---

    pub(crate) fn expr(&mut self) -> Res<Expr> {
        self.binaria(1)
    }

    fn precedencia(t: &Token) -> Option<(u8, OpBinario)> {
        Some(match t {
            Token::TuberiaTuberia => (1, OpBinario::OLogico),
            Token::AmpAmp => (2, OpBinario::YLogico),
            Token::IgualIgual => (3, OpBinario::Igual),
            Token::Distinto => (3, OpBinario::Distinto),
            Token::Menor => (3, OpBinario::Menor),
            Token::MenorIgual => (3, OpBinario::MenorIgual),
            Token::Mayor => (3, OpBinario::Mayor),
            Token::MayorIgual => (3, OpBinario::MayorIgual),
            Token::Tuberia => (4, OpBinario::O),
            Token::Circunflejo => (5, OpBinario::Xor),
            Token::Ampersand => (6, OpBinario::Y),
            Token::DesplazaIzq => (7, OpBinario::DespIzq),
            Token::DesplazaDer => (7, OpBinario::DespDer),
            Token::Mas => (8, OpBinario::Suma),
            Token::Menos => (8, OpBinario::Resta),
            Token::MasSat => (8, OpBinario::SumaSat),
            Token::MenosSat => (8, OpBinario::RestaSat),
            Token::Estrella => (9, OpBinario::Mul),
            Token::Barra => (9, OpBinario::Div),
            Token::Porcentaje => (9, OpBinario::Mod),
            _ => return None,
        })
    }

    fn binaria(&mut self, minima: u8) -> Res<Expr> {
        let mut izq = self.unaria()?;
        while let Some((nivel, op)) = Self::precedencia(self.ver()) {
            if nivel < minima {
                break;
            }
            let lugar = self.lugar();
            self.avanzar();
            let der = self.binaria(nivel + 1)?;
            izq = Expr {
                lugar,
                clase: ClaseExpr::Binario(op, Box::new(izq), Box::new(der)),
            };
        }
        Ok(izq)
    }

    fn unaria(&mut self) -> Res<Expr> {
        let lugar = self.lugar();
        let clase = match self.ver() {
            Token::Menos => {
                self.avanzar();
                ClaseExpr::Unario(OpUnario::Negar, Box::new(self.unaria()?))
            }
            Token::Tilde => {
                self.avanzar();
                ClaseExpr::Unario(OpUnario::Complemento, Box::new(self.unaria()?))
            }
            Token::Admiracion => {
                self.avanzar();
                ClaseExpr::Unario(OpUnario::No, Box::new(self.unaria()?))
            }
            Token::Ampersand => {
                self.avanzar();
                ClaseExpr::DireccionDe(Box::new(self.unaria()?))
            }
            Token::Estrella => {
                self.avanzar();
                ClaseExpr::Deref(Box::new(self.unaria()?))
            }
            _ => return self.postfija(),
        };
        Ok(Expr { lugar, clase })
    }

    fn postfija(&mut self) -> Res<Expr> {
        let mut e = self.primaria()?;
        loop {
            let lugar = self.lugar();
            match self.ver() {
                Token::CorcheteAbre => {
                    self.avanzar();
                    let i = self.expr()?;
                    self.esperar(Token::CorcheteCierra, "al cerrar el indice")?;
                    e = Expr {
                        lugar,
                        clase: ClaseExpr::Indice(Box::new(e), Box::new(i)),
                    };
                }
                Token::Punto => {
                    self.avanzar();
                    let (campo, _) = self.ident("el nombre de un campo")?;
                    e = Expr {
                        lugar,
                        clase: ClaseExpr::Campo(Box::new(e), campo),
                    };
                }
                Token::Como => {
                    self.avanzar();
                    let t = self.tipo()?;
                    e = Expr {
                        lugar,
                        clase: ClaseExpr::Cast(Box::new(e), t),
                    };
                }
                _ => return Ok(e),
            }
        }
    }

    fn primaria(&mut self) -> Res<Expr> {
        let lugar = self.lugar();
        let clase = match self.ver().clone() {
            Token::Entero(n) => {
                self.avanzar();
                ClaseExpr::Entero(n)
            }
            Token::Verdadero => {
                self.avanzar();
                ClaseExpr::Bool(true)
            }
            Token::Falso => {
                self.avanzar();
                ClaseExpr::Bool(false)
            }
            Token::Cadena(s) => {
                self.avanzar();
                ClaseExpr::Cadena(s)
            }
            Token::Ident(nombre) => {
                self.avanzar();
                if self.ver() == &Token::ParenAbre {
                    self.avanzar();
                    let mut args = Vec::new();
                    if self.ver() != &Token::ParenCierra {
                        loop {
                            args.push(self.expr()?);
                            if !self.si_es(&Token::Coma) {
                                break;
                            }
                        }
                    }
                    self.esperar(Token::ParenCierra, "al cerrar la llamada")?;
                    ClaseExpr::Llamada(nombre, args)
                } else {
                    ClaseExpr::Nombre(nombre)
                }
            }
            Token::ParenAbre => {
                self.avanzar();
                let e = self.expr()?;
                self.esperar(Token::ParenCierra, "al cerrar el parentesis")?;
                return Ok(e);
            }
            _ => return Err(self.inesperado("una expresion")),
        };
        Ok(Expr { lugar, clase })
    }
}
