//! Ensamblador y desensamblador de Lorenz.
//!
//! Ninguno de los dos conoce una ISA concreta: todo lo que saben de la maquina (mnemonicos,
//! sintaxis, codificacion y rangos de operandos) lo leen de la descripcion de arquitectura
//! (RF-ENS-001). Una instruccion nueva en el TOML aparece en el ensamblador y en el desensamblador
//! sin tocar este crate.

pub mod error;
pub mod expr;
pub mod fuente;

pub use lorenz_arch::plantilla;
pub mod codificar;
pub mod desensamblador;
pub mod ensamblador;
pub mod preprocesar;
pub mod salidas;
