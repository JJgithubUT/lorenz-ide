//! Depurador de linea de comandos de Lorenz (RF-DEP-001).
//!
//! La logica esta en [`Depurador`], que no lee ni escribe en ninguna terminal: recibe ordenes como
//! texto y devuelve texto. Asi se prueba sin lanzar procesos y asi lo pueden manejar tambien un
//! guion o, mas adelante, un depurador grafico con las mismas operaciones (RF-DEP-044).

pub mod condicion;
pub mod error;
