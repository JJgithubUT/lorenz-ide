//! El IDE de Lorenz: un servidor local con una API JSON sobre el nucleo (RF-IDE-001).
//!
//! Cada pestana del navegador usa sesiones independientes; el proyecto es la carpeta desde la que
//! se lanza `lorenz ide`, y ninguna ruta puede salir de ella.

mod api;
mod sesion;

pub use api::router;
pub use sesion::{Ide, MAX_SESIONES};

pub const PAGINA: &str = include_str!("pagina.html");
