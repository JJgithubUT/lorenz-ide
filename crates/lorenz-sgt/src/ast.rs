//! Arbol sintactico de SGT, tal como sale del analizador, sin tipos resueltos.

use crate::diagnostico::Lugar;

/// Un archivo fuente completo.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Programa {
    pub elementos: Vec<Elemento>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Elemento {
    Importar(Importar),
    Const(DefConst),
    Global(DefGlobal),
    Estructura(DefEstructura),
    Funcion(DefFuncion),
}

/// `importar modulo { a, b };`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Importar {
    pub lugar: Lugar,
    pub modulo: String,
    pub simbolos: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefConst {
    pub lugar: Lugar,
    pub nombre: String,
    pub tipo: TipoAst,
    pub valor: Expr,
}

/// `var nombre: tipo [@ direccion] [= inicial];` fuera de una funcion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefGlobal {
    pub lugar: Lugar,
    pub nombre: String,
    pub tipo: TipoAst,
    pub direccion: Option<Expr>,
    pub inicial: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Campo {
    pub lugar: Lugar,
    pub nombre: String,
    pub tipo: TipoAst,
    /// Desplazamiento fijado con `@ n`, en palabras desde el inicio de la estructura.
    pub desplazamiento: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefEstructura {
    pub lugar: Lugar,
    pub nombre: String,
    pub campos: Vec<Campo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parametro {
    pub lugar: Lugar,
    pub nombre: String,
    pub tipo: TipoAst,
}

/// Atributo de una funcion: `@interrupcion(1)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atributo {
    pub lugar: Lugar,
    pub nombre: String,
    pub argumentos: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefFuncion {
    pub lugar: Lugar,
    pub atributos: Vec<Atributo>,
    pub nombre: String,
    pub parametros: Vec<Parametro>,
    pub retorno: Option<TipoAst>,
    pub cuerpo: Bloque,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TipoAst {
    pub lugar: Lugar,
    pub clase: ClaseTipo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaseTipo {
    Nombre(String),
    Puntero(Box<TipoAst>),
    Arreglo(Box<TipoAst>, Box<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bloque {
    pub sentencias: Vec<Sentencia>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentencia {
    pub lugar: Lugar,
    pub clase: ClaseSentencia,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaseSentencia {
    Var {
        nombre: String,
        tipo: Option<TipoAst>,
        inicial: Option<Expr>,
    },
    /// `destino = valor;` o, con `op`, `destino op= valor;`
    Asignar {
        destino: Expr,
        op: Option<OpBinario>,
        valor: Expr,
    },
    Si {
        condicion: Expr,
        entonces: Bloque,
        sino: Option<Bloque>,
    },
    Mientras {
        condicion: Expr,
        cuerpo: Bloque,
    },
    Bucle(Bloque),
    Romper,
    Continuar,
    Devolver(Option<Expr>),
    Segun {
        valor: Expr,
        brazos: Vec<(Expr, Bloque)>,
        otro: Option<Bloque>,
    },
    /// Ensamblador en linea; `{nombre}` dentro del texto se sustituye por la variable.
    Asm(String),
    Expr(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpBinario {
    Suma,
    Resta,
    Mul,
    Div,
    Mod,
    SumaSat,
    RestaSat,
    Y,
    O,
    Xor,
    DespIzq,
    DespDer,
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
            OpBinario::Mul => "*",
            OpBinario::Div => "/",
            OpBinario::Mod => "%",
            OpBinario::SumaSat => "+|",
            OpBinario::RestaSat => "-|",
            OpBinario::Y => "&",
            OpBinario::O => "|",
            OpBinario::Xor => "^",
            OpBinario::DespIzq => "<<",
            OpBinario::DespDer => ">>",
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

    pub fn es_comparacion(self) -> bool {
        matches!(
            self,
            OpBinario::Igual
                | OpBinario::Distinto
                | OpBinario::Menor
                | OpBinario::MenorIgual
                | OpBinario::Mayor
                | OpBinario::MayorIgual
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpUnario {
    /// `-x`
    Negar,
    /// `~x`
    Complemento,
    /// `!x`
    No,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub lugar: Lugar,
    pub clase: ClaseExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaseExpr {
    Entero(u64),
    Bool(bool),
    Cadena(String),
    Nombre(String),
    Binario(OpBinario, Box<Expr>, Box<Expr>),
    Unario(OpUnario, Box<Expr>),
    Llamada(String, Vec<Expr>),
    Indice(Box<Expr>, Box<Expr>),
    Campo(Box<Expr>, String),
    Cast(Box<Expr>, TipoAst),
    /// `&x`
    DireccionDe(Box<Expr>),
    /// `*p`
    Deref(Box<Expr>),
}
