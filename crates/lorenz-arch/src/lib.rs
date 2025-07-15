//! Carga y validacion de descripciones de arquitectura de Lorenz.
//!
//! Este crate es la raiz del grafo de dependencias: ninguna otra parte de Lorenz define la ISA por
//! su cuenta, todas la consultan aqui (RNF-MAN-002).

pub mod error;

pub use error::{ErrorArquitectura, FalloValidacion, Posicion, Regla};

/// Version del formato de descripcion que esta biblioteca entiende.
pub const VERSION_FORMATO: &str = "0.1";
