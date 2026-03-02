//! Simulacion ciclo a ciclo de Lorenz (fase 5).
//!
//! El emulador funcional dice *que* hace un programa; este crate dice *cuanto tarda* en una
//! maquina con pipeline, prediccion de saltos, cache y memorias con latencia. Lo hace sin tocar el
//! programa ni la descripcion funcional: la temporizacion se declara en secciones aparte de la
//! misma descripcion (`[pipeline]`, `[cache]` y la clave `latencia` de cada `[[region]]`), que el
//! emulador funcional ignora (RF-CIC-002).

pub mod cache;
pub mod config;
pub mod pipeline;

pub use cache::{
    Cache, ConfigCache, Escritura, EstadisticasCache, FilaCache, Reemplazo, ResultadoCache,
};
pub use config::{cargar_config, Config, LatenciaRegion};
pub use pipeline::{Causa, ConfigPipeline, Entrada, Etapa, Planificada, Planificador, Prediccion};
