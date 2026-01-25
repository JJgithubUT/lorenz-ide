//! Compilador de SGT (Saggitarius), el lenguaje de alto nivel de Lorenz.
//!
//! El compilador no conoce una arquitectura concreta: lo que sabe de la maquina destino lo lee de
//! su descripcion (RF-SGT-024). El recorrido es fuente, arbol sintactico, representacion
//! intermedia independiente de la arquitectura, y de ahi a ensamblador de la maquina activa.

pub mod analisis;
pub mod ast;
pub mod biblioteca;
pub mod compilar;
pub mod costes;
pub mod destino;
pub mod diagnostico;
pub mod emisor;
pub mod generador;
pub mod hir;
pub mod interprete;
pub mod layout;
pub mod lexico;
pub mod optimizar;
pub mod podar;
pub mod seleccion;
pub mod sintaxis;
pub mod tipos;
pub mod valores;
