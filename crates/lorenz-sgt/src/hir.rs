//! Representacion tipada del programa, ya resueltos nombres, tipos y constantes.

use crate::ast::{OpBinario, OpUnario};
use crate::diagnostico::Lugar;
use crate::tipos::Tipo;

pub type IdFuncion = usize;
pub type IdGlobal = usize;
pub type IdLocal = usize;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Modulo {
    pub estructuras: Vec<Estructura>,
    pub globales: Vec<Variable>,
    pub funciones: Vec<Funcion>,
    /// Archivo del que salio, para los diagnosticos de fases posteriores.
    pub archivo: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampoH {
    pub nombre: String,
    pub tipo: Tipo,
    /// Desplazamiento en palabras desde el inicio de la estructura.
    pub desplazamiento: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Estructura {
    pub nombre: String,
    pub campos: Vec<CampoH>,
    /// Tamano en palabras.
    pub tamano: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub nombre: String,
    pub tipo: Tipo,
    pub lugar: Lugar,
    /// Direccion fijada con `@`.
    pub direccion: Option<u64>,
    /// Valor inicial de un escalar.
    pub inicial: Option<Expr>,
    /// Contenido inicial de un arreglo de enteros escrito como texto: un valor por elemento, desde
    /// el primero; el resto empieza en cero.
    pub datos: Vec<i128>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Funcion {
    /// Nombre unico en el modulo; una copia para otra raiz lleva un sufijo.
    pub nombre: String,
    /// Nombre escrito en el fuente.
    pub base: String,
    /// Raiz de ejecucion a la que pertenece esta copia: 0 es el programa principal y las demas
    /// son manejadores de interrupcion (decision D-063).
    pub raiz: usize,
    pub lugar: Lugar,
    /// Ids de las variables locales que son parametros, en orden.
    pub parametros: Vec<IdLocal>,
    pub locales: Vec<Variable>,
    pub retorno: Tipo,
    pub cuerpo: Vec<Sent>,
    /// Linea de interrupcion que atiende, si es un manejador.
    pub interrupcion: Option<u32>,
    /// Ciclos maximos que puede costar, fijados con `@presupuesto(n)`.
    pub presupuesto: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ref {
    Global(IdGlobal),
    Local(IdLocal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    pub lugar: Lugar,
    pub clase: ClaseSent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaseSent {
    Asignar(Sitio, Expr),
    Si(Expr, Vec<Sent>, Vec<Sent>),
    Mientras(Expr, Vec<Sent>),
    Bucle(Vec<Sent>),
    Romper,
    Continuar,
    Devolver(Option<Expr>),
    Segun {
        valor: Expr,
        brazos: Vec<(i128, Vec<Sent>)>,
        otro: Vec<Sent>,
    },
    /// Texto de ensamblador con las variables que cita, en el orden de sus `{nombre}`.
    Asm(String, Vec<(String, Ref)>),
    Salida(Expr, Expr),
    Expr(Expr),
}

/// Algo donde se puede guardar o de donde se puede leer: una variable, un elemento, un campo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sitio {
    pub tipo: Tipo,
    pub clase: ClaseSitio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaseSitio {
    Variable(Ref),
    /// Elemento de un arreglo: base, indice y tamano del elemento en palabras.
    Indice(Box<Sitio>, Box<Expr>, u64),
    /// Campo de una estructura, con su desplazamiento en palabras.
    Campo(Box<Sitio>, u64),
    /// `*p`
    Deref(Box<Expr>),
    /// Un registro de la maquina, por nombre.
    Registro(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub lugar: Lugar,
    pub tipo: Tipo,
    pub clase: ClaseExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaseExpr {
    /// Un valor ya calculado; para un entero con signo se guarda envuelto al ancho del tipo.
    Const(i128),
    Leer(Box<Sitio>),
    /// `&sitio`
    Direccion(Box<Sitio>),
    Binario(OpBinario, Box<Expr>, Box<Expr>),
    Unario(OpUnario, Box<Expr>),
    Cast(Box<Expr>),
    Llamada(IdFuncion, Vec<Expr>),
    Entrada(Box<Expr>),
    Bandera(String),
}

// ----------------------------------------------------------------------
// Recorridos
// ----------------------------------------------------------------------

/// Recorre en profundidad todas las expresiones y referencias a variables de una sentencia.
pub fn recorrer_sent_mut(s: &mut Sent, e: &mut dyn FnMut(&mut Expr), r: &mut dyn FnMut(&mut Ref)) {
    match &mut s.clase {
        ClaseSent::Asignar(sitio, v) => {
            recorrer_sitio_mut(sitio, e, r);
            recorrer_expr_mut(v, e, r);
        }
        ClaseSent::Si(c, a, b) => {
            recorrer_expr_mut(c, e, r);
            a.iter_mut()
                .chain(b.iter_mut())
                .for_each(|s| recorrer_sent_mut(s, e, r));
        }
        ClaseSent::Mientras(c, b) => {
            recorrer_expr_mut(c, e, r);
            b.iter_mut().for_each(|s| recorrer_sent_mut(s, e, r));
        }
        ClaseSent::Bucle(b) => b.iter_mut().for_each(|s| recorrer_sent_mut(s, e, r)),
        ClaseSent::Devolver(Some(v)) | ClaseSent::Expr(v) => recorrer_expr_mut(v, e, r),
        ClaseSent::Segun {
            valor,
            brazos,
            otro,
        } => {
            recorrer_expr_mut(valor, e, r);
            for (_, b) in brazos {
                b.iter_mut().for_each(|s| recorrer_sent_mut(s, e, r));
            }
            otro.iter_mut().for_each(|s| recorrer_sent_mut(s, e, r));
        }
        ClaseSent::Salida(a, b) => {
            recorrer_expr_mut(a, e, r);
            recorrer_expr_mut(b, e, r);
        }
        ClaseSent::Asm(_, refs) => refs.iter_mut().for_each(|(_, x)| r(x)),
        ClaseSent::Romper | ClaseSent::Continuar | ClaseSent::Devolver(None) => {}
    }
}

pub fn recorrer_sitio_mut(
    s: &mut Sitio,
    e: &mut dyn FnMut(&mut Expr),
    r: &mut dyn FnMut(&mut Ref),
) {
    match &mut s.clase {
        ClaseSitio::Variable(x) => r(x),
        ClaseSitio::Indice(b, i, _) => {
            recorrer_sitio_mut(b, e, r);
            recorrer_expr_mut(i, e, r);
        }
        ClaseSitio::Campo(b, _) => recorrer_sitio_mut(b, e, r),
        ClaseSitio::Deref(p) => recorrer_expr_mut(p, e, r),
        ClaseSitio::Registro(_) => {}
    }
}

pub fn recorrer_expr_mut(x: &mut Expr, e: &mut dyn FnMut(&mut Expr), r: &mut dyn FnMut(&mut Ref)) {
    e(x);
    match &mut x.clase {
        ClaseExpr::Leer(s) | ClaseExpr::Direccion(s) => recorrer_sitio_mut(s, e, r),
        ClaseExpr::Binario(_, a, b) => {
            recorrer_expr_mut(a, e, r);
            recorrer_expr_mut(b, e, r);
        }
        ClaseExpr::Unario(_, a) | ClaseExpr::Cast(a) | ClaseExpr::Entrada(a) => {
            recorrer_expr_mut(a, e, r)
        }
        ClaseExpr::Llamada(_, args) => args.iter_mut().for_each(|a| recorrer_expr_mut(a, e, r)),
        ClaseExpr::Const(_) | ClaseExpr::Bandera(_) => {}
    }
}

impl Funcion {
    /// Recorre todo el cuerpo de la funcion.
    pub fn recorrer_mut(&mut self, e: &mut dyn FnMut(&mut Expr), r: &mut dyn FnMut(&mut Ref)) {
        self.cuerpo
            .iter_mut()
            .for_each(|s| recorrer_sent_mut(s, e, r));
    }
}

impl Modulo {
    pub fn funcion_por_nombre(&self, nombre: &str) -> Option<IdFuncion> {
        self.funciones.iter().position(|f| f.nombre == nombre)
    }
}
