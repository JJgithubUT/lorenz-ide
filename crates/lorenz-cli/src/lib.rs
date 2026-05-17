//! Interfaz de linea de comandos de Lorenz.
//!
//! La logica vive en esta biblioteca para poder probarla sin lanzar procesos; `main.rs` solo
//! conecta la salida estandar y devuelve el codigo de salida.

pub mod exportar;

pub mod pdf;

pub mod importar;

pub mod programa;

pub mod args;

pub mod comparar;

pub mod ordenes;

pub use ordenes::{
    ejecutar_cli, ejecutar_cli_con_entrada, SALIDA_DIFIEREN, SALIDA_ERROR_EJECUCION,
    SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_NO_CUMPLE, SALIDA_OK,
};

pub mod guion;

pub mod ensamblar;

pub mod enlazar;

pub mod depurar;

pub mod probar;

pub mod conformidad;

pub mod aleatorio;

pub mod compatibilidad;

/// Documentacion generada de la arquitectura.
pub mod documentar;

/// Compilacion de SGT.
pub mod compilar;

pub mod nuevo;

/// Simulacion ciclo a ciclo.
pub mod ciclos;

/// Instrucciones nuevas que acortarian los programas.
pub mod sugerir;

/// Historico de metricas por version de la arquitectura.
pub mod historial_ciclos;

/// El mismo programa SGT en varias maquinas.
pub mod cotejar;

/// Restricciones de diseno y variantes que las cumplan.
pub mod variantes;

/// La unidad grafica.
pub mod desafios;
pub mod ejercicios;
pub mod gpu;
/// Los hilos de la unidad grafica.
pub mod html;
pub mod kernel;
/// Varios nucleos sobre memoria compartida.
pub mod nucleos;
pub mod paquete;
pub mod plantillas;
pub mod proyecto;
pub mod recientes;
pub mod tareas;
