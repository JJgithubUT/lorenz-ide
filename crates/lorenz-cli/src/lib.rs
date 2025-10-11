//! Interfaz de linea de comandos de Lorenz.
//!
//! La logica vive en esta biblioteca para poder probarla sin lanzar procesos; `main.rs` solo
//! conecta la salida estandar y devuelve el codigo de salida.

pub mod programa;

pub mod args;

pub mod comparar;

pub mod ordenes;

pub use ordenes::{
    ejecutar_cli, ejecutar_cli_con_entrada, SALIDA_DIFIEREN, SALIDA_ERROR_EJECUCION,
    SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_OK,
};

pub mod guion;

pub mod ensamblar;

pub mod depurar;

pub mod probar;

pub mod conformidad;
