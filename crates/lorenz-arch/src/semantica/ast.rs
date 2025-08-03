//! Arbol de sintaxis de la expresion de `semantica` de una instruccion.
//!
//! El texto se analiza una sola vez al cargar la descripcion; el emulador interpreta este arbol y
//! nunca vuelve a leer la cadena original (D-009).

use std::fmt;

/// Operadores de un solo operando.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpUnario {
    /// `!x`: 1 si `x` es cero, 0 en caso contrario.
    NegacionLogica,
    /// `~x`: complemento bit a bit.
    NegacionBits,
    /// `-x`: cambio de signo.
    Menos,
}

impl OpUnario {
    pub fn simbolo(self) -> &'static str {
        match self {
            OpUnario::NegacionLogica => "!",
            OpUnario::NegacionBits => "~",
            OpUnario::Menos => "-",
        }
    }
}

/// Operadores de dos operandos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpBinario {
    Suma,
    Resta,
    Producto,
    Division,
    Modulo,
    DesplazaIzq,
    DesplazaDer,
    Y,
    O,
    Xor,
    Igual,
    Distinto,
    Menor,
    MenorIgual,
    Mayor,
    MayorIgual,
    YLogico,
    OLogico,
}

impl OpBinario {
    pub fn simbolo(self) -> &'static str {
        match self {
            OpBinario::Suma => "+",
            OpBinario::Resta => "-",
            OpBinario::Producto => "*",
            OpBinario::Division => "/",
            OpBinario::Modulo => "%",
            OpBinario::DesplazaIzq => "<<",
            OpBinario::DesplazaDer => ">>",
            OpBinario::Y => "&",
            OpBinario::O => "|",
            OpBinario::Xor => "^",
            OpBinario::Igual => "==",
            OpBinario::Distinto => "!=",
            OpBinario::Menor => "<",
            OpBinario::MenorIgual => "<=",
            OpBinario::Mayor => ">",
            OpBinario::MayorIgual => ">=",
            OpBinario::YLogico => "&&",
            OpBinario::OLogico => "||",
        }
    }

    /// Operadores cuyo resultado sin truncar puede salirse del ancho de palabra, y por tanto
    /// producir acarreo o prestamo.
    pub fn puede_producir_acarreo(self) -> bool {
        matches!(
            self,
            OpBinario::Suma | OpBinario::Resta | OpBinario::Producto | OpBinario::DesplazaIzq
        )
    }
}

/// Una expresion que produce un valor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// Una constante.
    Literal(u64),
    /// Un operando de la instruccion, un registro, una bandera, o los alias `pc` y `sp`.
    Nombre(String),
    /// `mem[direccion]`.
    Mem(Box<Expr>),
    /// `in[puerto]`.
    Entrada(Box<Expr>),
    Unario(OpUnario, Box<Expr>),
    Binario(OpBinario, Box<Expr>, Box<Expr>),
}

/// Donde se guarda el resultado de una asignacion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destino {
    Nombre(String),
    Mem(Expr),
    Salida(Expr),
}

/// Una orden dentro de la semantica de una instruccion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sentencia {
    Asignar(Destino, Expr),
    /// `si condicion: cuerpo`.
    Si(Expr, Vec<Sentencia>),
    /// `detener`: la maquina se para.
    Detener,
}

/// La semantica completa de una instruccion. Vacia significa que no hace nada.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Programa {
    pub sentencias: Vec<Sentencia>,
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Literal(n) => write!(f, "{n}"),
            Expr::Nombre(n) => f.write_str(n),
            Expr::Mem(e) => write!(f, "mem[{e}]"),
            Expr::Entrada(e) => write!(f, "in[{e}]"),
            Expr::Unario(op, e) => write!(f, "{}{}", op.simbolo(), e),
            Expr::Binario(op, a, b) => write!(f, "({} {} {})", a, op.simbolo(), b),
        }
    }
}

impl fmt::Display for Destino {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Destino::Nombre(n) => f.write_str(n),
            Destino::Mem(e) => write!(f, "mem[{e}]"),
            Destino::Salida(e) => write!(f, "out[{e}]"),
        }
    }
}

impl fmt::Display for Sentencia {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sentencia::Asignar(d, e) => write!(f, "{d} = {e}"),
            Sentencia::Si(c, cuerpo) => {
                write!(f, "si {c}: ")?;
                if cuerpo.len() == 1 {
                    write!(f, "{}", cuerpo[0])
                } else {
                    f.write_str("{ ")?;
                    escribir_lista(f, cuerpo)?;
                    f.write_str(" }")
                }
            }
            Sentencia::Detener => f.write_str("detener"),
        }
    }
}

fn escribir_lista(f: &mut fmt::Formatter<'_>, sentencias: &[Sentencia]) -> fmt::Result {
    for (i, s) in sentencias.iter().enumerate() {
        if i > 0 {
            f.write_str("; ")?;
        }
        write!(f, "{s}")?;
    }
    Ok(())
}

impl fmt::Display for Programa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        escribir_lista(f, &self.sentencias)
    }
}

impl Expr {
    /// Anade a `salida` todos los nombres que la expresion lee, en orden de aparicion.
    pub fn recoger_nombres(&self, salida: &mut Vec<String>) {
        match self {
            Expr::Literal(_) => {}
            Expr::Nombre(n) => salida.push(n.clone()),
            Expr::Mem(e) | Expr::Entrada(e) | Expr::Unario(_, e) => e.recoger_nombres(salida),
            Expr::Binario(_, a, b) => {
                a.recoger_nombres(salida);
                b.recoger_nombres(salida);
            }
        }
    }
}

impl Programa {
    /// Nombres que el programa lee, sin repetir y en orden de aparicion.
    ///
    /// Incluye los que aparecen dentro de los indices de `mem[...]` y `out[...]` de un destino.
    pub fn nombres_leidos(&self) -> Vec<String> {
        let mut todos = Vec::new();
        recoger_leidos(&self.sentencias, &mut todos);
        sin_repetir(todos)
    }

    /// Nombres que el programa asigna directamente (no a memoria ni a puertos).
    pub fn nombres_escritos(&self) -> Vec<String> {
        let mut todos = Vec::new();
        recoger_escritos(&self.sentencias, &mut todos);
        sin_repetir(todos)
    }

    /// Si alguna sentencia puede detener la maquina.
    pub fn puede_detener(&self) -> bool {
        fn alguna(sentencias: &[Sentencia]) -> bool {
            sentencias.iter().any(|s| match s {
                Sentencia::Detener => true,
                Sentencia::Si(_, cuerpo) => alguna(cuerpo),
                Sentencia::Asignar(_, _) => false,
            })
        }
        alguna(&self.sentencias)
    }
}

fn recoger_leidos(sentencias: &[Sentencia], salida: &mut Vec<String>) {
    for s in sentencias {
        match s {
            Sentencia::Asignar(destino, valor) => {
                match destino {
                    Destino::Nombre(_) => {}
                    Destino::Mem(e) | Destino::Salida(e) => e.recoger_nombres(salida),
                }
                valor.recoger_nombres(salida);
            }
            Sentencia::Si(condicion, cuerpo) => {
                condicion.recoger_nombres(salida);
                recoger_leidos(cuerpo, salida);
            }
            Sentencia::Detener => {}
        }
    }
}

fn recoger_escritos(sentencias: &[Sentencia], salida: &mut Vec<String>) {
    for s in sentencias {
        match s {
            Sentencia::Asignar(Destino::Nombre(n), _) => salida.push(n.clone()),
            Sentencia::Asignar(_, _) | Sentencia::Detener => {}
            Sentencia::Si(_, cuerpo) => recoger_escritos(cuerpo, salida),
        }
    }
}

fn sin_repetir(nombres: Vec<String>) -> Vec<String> {
    let mut vistos = Vec::new();
    for n in nombres {
        if !vistos.contains(&n) {
            vistos.push(n);
        }
    }
    vistos
}
