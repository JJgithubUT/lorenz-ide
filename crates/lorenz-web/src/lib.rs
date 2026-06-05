//! Servidor HTTP minimo, solo con la biblioteca estandar.
//!
//! Sirve el IDE y su API. Hace una cosa bien y con limites: una peticion por conexion
//! (`Connection: close`), un hilo por conexion con un tope, tamanos maximos de cabeceras y de
//! cuerpo, plazos de lectura, rutas con parametros y un cierre ordenado que espera a que terminen
//! las peticiones en curso.

pub mod json;
pub mod peticion;
pub mod respuesta;
pub mod ruta;
pub mod servidor;

pub use json::Json;
pub use peticion::Peticion;
pub use respuesta::Respuesta;
pub use ruta::Router;
pub use servidor::{Limites, Servidor};
