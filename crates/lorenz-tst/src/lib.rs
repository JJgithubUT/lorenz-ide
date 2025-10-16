//! Pruebas de proyecto, suite de conformidad, cobertura y generacion aleatoria (RF-TST).
//!
//! Una *prueba de proyecto* es un programa, un estado inicial, unas entradas simuladas y lo que se
//! espera al terminar. Se declara en un archivo TOML junto a la arquitectura (RF-TST-001).

pub mod aleatorio;
pub mod casos;
pub mod cobertura;
pub mod conformidad;
pub mod ejecutor;
pub mod ejecutor_referencia;
pub mod error;
pub mod formato;
pub mod numero;
pub mod referencia;
pub mod regresion;
pub mod suite;
