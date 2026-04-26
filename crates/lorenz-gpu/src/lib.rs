//! La unidad grafica de Lorenz (fase 7): un modelo educativo, no uno realista.
//!
//! Una unidad grafica tiene un framebuffer, sabe dibujar unas pocas primitivas y cuenta sus
//! propios ciclos, aparte de los de la CPU.

pub mod superficie;

pub use superficie::{
    componer, descomponer, ConfigSuperficie, ProfundidadColor, Superficie, PROFUNDIDAD_LEJANA,
};
pub mod raster;
pub use raster::{linea, rellenar, triangulo, Vertice};
pub mod comandos;
pub mod error;
pub mod imagen;
pub use comandos::{leer_guion, Comando};
pub use error::ErrorGpu;
pub mod config;
pub use config::{cargar_config, cargar_config_archivo, ConfigGpu, ConfigHilos, Costes};
pub mod unidad;
pub use unidad::{Gpu, Metricas};
pub mod sistema;
pub use sistema::{color_nativo, convertir_palabras, SistemaGrafico};
pub mod hilos;
pub mod kernel;
pub use hilos::{EjecucionHilos, Emision, MetricasHilos};
pub use kernel::{ensamblar as ensamblar_kernel, Kernel};
