//! Simulacion ciclo a ciclo de Lorenz (fase 5).
//!
//! El emulador funcional dice *que* hace un programa; este crate dice *cuanto tarda* en una
//! maquina con pipeline, prediccion de saltos, cache y memorias con latencia.

pub mod cache;

pub use cache::{
    Cache, ConfigCache, Escritura, EstadisticasCache, FilaCache, Reemplazo, ResultadoCache,
};
