//! Carga y validacion de descripciones de arquitectura de Lorenz.
//!
//! Este crate es la raiz del grafo de dependencias: ninguna otra parte de Lorenz define la ISA por
//! su cuenta, todas la consultan aqui (RNF-MAN-002).

pub mod bits;
pub mod carga;
pub mod documentar;
pub mod error;
mod herencia;
mod localizar;
pub mod logisim;
pub mod modelo;
pub mod plantilla;
pub mod semantica;
pub mod validacion;
pub mod verilog;

pub use bits::{Codificacion, ErrorBits, Pieza};
pub use carga::{cargar, cargar_archivo, mezclar_archivo};
pub use error::{ErrorArquitectura, FalloValidacion, Posicion, Regla};
pub use herencia::cadena as cadena_de_herencia;
pub use modelo::{
    Acceso, Bandera, Ciclos, Dispositivo, Endianness, Instruccion, Interrupciones, Maquina,
    Operando, ParRegistros, Region, Registro, RolRegistro, SentidoPila, TipoDispositivo,
    TipoOperando,
};

/// Version del formato de descripcion que esta biblioteca entiende.
pub const VERSION_FORMATO: &str = "0.1";

/// La marca que llevan los textos que Lorenz genera (RF-ECO-046): quien los lee sabe que no los
/// escribio una persona y con que orden se regeneran.
pub fn marca_generado(orden: &str) -> String {
    format!(
        "Generado por Lorenz {} con `{orden}`. Se regenera: no lo edites a mano.",
        env!("CARGO_PKG_VERSION")
    )
}
