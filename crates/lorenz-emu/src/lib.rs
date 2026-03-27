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

pub use memoria::{
    AccesoVigilado, Escritura, EscrituraRegistrada, LecturaSinInicializar, Memoria, TipoAcceso,
    Trafico, MAX_LECTURAS_SIN_INICIALIZAR,
};
pub mod estado;
pub mod programa;

pub use estado::Estado;

pub mod puertos;
pub(crate) mod script;

pub use puertos::{Puertos, SolicitudDma};

pub mod decodificador;

pub use decodificador::{Decodificada, Decodificador, OperandoDecodificado};

pub mod interprete;

pub use interprete::{Entorno, Interprete, Resultado};

pub mod emulador;

pub use emulador::{
    Emulador, Instantanea, InterrupcionAtendida, Paso, Resumen, TransferenciaDma,
    MAX_INTERRUPCIONES_REGISTRADAS,
};

pub mod comparar;

pub use comparar::{comparar, Ejecutor, Veredicto, Vista};

pub mod pantalla;

pub use pantalla::{leer_imagen, Imagen};
