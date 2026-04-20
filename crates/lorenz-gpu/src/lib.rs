//! La unidad grafica de Lorenz (fase 7): un modelo educativo, no uno realista.
//!
//! Una unidad grafica tiene un framebuffer, sabe dibujar unas pocas primitivas y cuenta sus
//! propios ciclos, aparte de los de la CPU.

pub mod superficie;

pub use superficie::{
    componer, descomponer, ConfigSuperficie, ProfundidadColor, Superficie, PROFUNDIDAD_LEJANA,
};
