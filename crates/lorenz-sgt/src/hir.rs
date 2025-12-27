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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Funcion {
    pub nombre: String,
    pub lugar: Lugar,
    /// Ids de las variables locales que son parametros, en orden.
    pub parametros: Vec<IdLocal>,
    pub locales: Vec<Variable>,
    pub retorno: Tipo,
    pub cuerpo: Vec<Sent>,
    /// Linea de interrupcion que atiende, si es un manejador.
    pub interrupcion: Option<u32>,
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
