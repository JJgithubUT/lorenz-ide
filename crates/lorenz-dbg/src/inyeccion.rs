//! Inyeccion de fallos (RF-DEP-042): bits alterados, entradas perdidas e interrupciones que se
//! aplican solas cuando la maquina llega a un instante elegido.
//!
//! El instante es el numero de instrucciones ejecutadas: una inyeccion "en el instante 5" actua
//! justo antes de la sexta instruccion. Cada inyeccion se aplica con las mismas operaciones que
//! los cambios a mano, asi que queda anotada en la traza y se puede retroceder y reproducir.

use std::fmt;

/// Que se estropea.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inyeccion {
    /// Invierte un bit de un registro.
    BitRegistro { nombre: String, bit: u32 },
    /// Invierte un bit de una celda de memoria.
    BitMemoria { direccion: u64, bit: u32 },
    /// Descarta el siguiente valor que espera en la cola de entrada de un puerto.
    PerderEntrada { puerto: u64 },
    /// Levanta una linea de interrupcion sin que la pida ningun dispositivo.
    Interrupcion { linea: u32 },
}

impl fmt::Display for Inyeccion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Inyeccion::BitRegistro { nombre, bit } => write!(f, "bit {bit} de {nombre}"),
            Inyeccion::BitMemoria { direccion, bit } => {
                write!(f, "bit {bit} de [{direccion:#x}]")
            }
            Inyeccion::PerderEntrada { puerto } => {
                write!(f, "entrada perdida en el puerto {puerto:#x}")
            }
            Inyeccion::Interrupcion { linea } => write!(f, "interrupcion {linea}"),
        }
    }
}

/// Una inyeccion con su instante.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Programada {
    pub id: u32,
    pub instante: u64,
    pub inyeccion: Inyeccion,
    /// Ya se aplico; no se repite aunque se vuelva a pasar por ese instante.
    pub aplicada: bool,
}
