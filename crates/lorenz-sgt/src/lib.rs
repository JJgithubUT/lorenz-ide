//! Compilador de SGT (Saggitarius), el lenguaje de alto nivel de Lorenz.
//!
//! El compilador no conoce una arquitectura concreta: lo que sabe de la maquina destino lo lee de
//! su descripcion (RF-SGT-024). El recorrido es fuente, arbol sintactico, representacion
//! intermedia independiente de la arquitectura, y de ahi a ensamblador de la maquina activa.

pub mod diagnostico;
pub mod lexico;
