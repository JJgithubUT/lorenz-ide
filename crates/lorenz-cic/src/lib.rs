//! Simulacion ciclo a ciclo de Lorenz (fase 5).
//!
//! El emulador funcional dice *que* hace un programa; este crate dice *cuanto tarda* en una
//! maquina con pipeline, prediccion de saltos, cache y memorias con latencia. Lo hace sin tocar el
//! programa ni la descripcion funcional: la temporizacion se declara en secciones aparte de la
//! misma descripcion (`[pipeline]`, `[cache]` y la clave `latencia` de cada `[[region]]`), que el
//! emulador funcional ignora (RF-CIC-002).

pub mod analisis;
pub mod cache;
pub mod config;
pub mod pipeline;
pub mod restricciones;
pub mod sim;
pub mod sugerir;
pub mod variantes;

pub use analisis::{dependencias, Dependencias};
pub use cache::{
    Cache, ConfigCache, Escritura, EstadisticasCache, FilaCache, Reemplazo, ResultadoCache,
};
pub use config::{
    cargar_config, cargar_config_archivo, Config, ConfigNucleos, LatenciaRegion, RegionCompartida,
};
pub use pipeline::{Causa, ConfigPipeline, Entrada, Etapa, Planificada, Planificador, Prediccion};
pub use restricciones::{comprobar, Medidas, Metrica, Restriccion, Veredicto};
pub use sim::{EstadoEtapa, FilaPerfil, Metricas, Ocupante, Simulador, Vista};
pub use variantes::{evaluar, variantes, variantes_isa, Espacio, Resultado, Variante, VarianteIsa};
