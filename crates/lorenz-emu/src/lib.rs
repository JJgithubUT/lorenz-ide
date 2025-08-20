//! Emulador funcional de Lorenz.
//!
//! Ejecuta programas sobre una maquina descrita con `lorenz-arch`. Todo lo que hace depende de la
//! descripcion: ningun opcode, registro ni mapa de memoria esta fijado en el codigo.
//!
//! El emulador es determinista: la misma maquina, el mismo programa y las mismas entradas producen
//! siempre el mismo resultado (RF-EMU-016).

pub mod error;

pub use error::ErrorEjecucion;
pub mod memoria;

pub use memoria::Memoria;
pub mod estado;

pub use estado::Estado;

pub mod puertos;

pub use puertos::Puertos;

pub mod decodificador;

pub use decodificador::{Decodificada, Decodificador, OperandoDecodificado};

pub mod interprete;

pub use interprete::{Entorno, Interprete, Resultado};

pub mod emulador;

pub use emulador::{Emulador, Paso, Resumen};

pub mod pantalla;

pub use pantalla::{leer_imagen, Imagen};
