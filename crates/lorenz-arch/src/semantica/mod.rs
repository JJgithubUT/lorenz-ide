//! Lenguaje de semantica de las instrucciones.
//!
//! Describe que le hace una instruccion al estado de la maquina, con un subconjunto deliberadamente
//! pequeno: asignaciones, aritmetica, acceso a memoria, condicionales y entrada/salida.

pub mod ast;

pub use ast::{Destino, Expr, OpBinario, OpUnario, Programa, Sentencia};
