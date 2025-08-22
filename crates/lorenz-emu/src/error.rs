//! Errores que puede producir la ejecucion de un programa.
//!
//! Todos describen lo ocurrido en terminos de la maquina emulada: una direccion, una region, un
//! puerto. Ninguno expone detalles internos del emulador (RNF-USA-003).

use thiserror::Error;

/// Un fallo durante la ejecucion.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorEjecucion {
    #[error("acceso a la direccion {direccion:#x}, que no esta en ninguna region de memoria")]
    FueraDeRegion { direccion: u64 },

    #[error(
        "escritura en la direccion {direccion:#x}, de la region '{region}', que es de solo lectura"
    )]
    EscrituraEnSoloLectura { direccion: u64, region: String },

    #[error(
        "lectura de la direccion {direccion:#x}, de la region '{region}', que es de solo escritura"
    )]
    LecturaDeSoloEscritura { direccion: u64, region: String },

    #[error("el puerto {puerto:#x} no esta declarado por ningun dispositivo")]
    PuertoNoDeclarado { puerto: u64 },

    #[error("la palabra {palabra:#x} en la direccion {direccion:#x} no corresponde a ninguna instruccion")]
    InstruccionInvalida { direccion: u64, palabra: u64 },

    #[error("division entre cero al ejecutar '{mnemonico}'")]
    DivisionEntreCero { mnemonico: String },

    #[error("la region '{region}' tiene {celdas} celdas y el maximo admitido es {maximo}")]
    RegionDemasiadoGrande {
        region: String,
        celdas: u64,
        maximo: u64,
    },

    #[error("el bloque de {longitud} palabras no cabe desde la direccion {direccion:#x}")]
    BloqueFueraDeRegion { direccion: u64, longitud: usize },

    #[error("la maquina no tiene un registro llamado '{nombre}'")]
    RegistroInexistente { nombre: String },

    #[error("el registro '{nombre}' es de solo lectura")]
    RegistroDeSoloLectura { nombre: String },

    #[error("la maquina no tiene una bandera llamada '{nombre}'")]
    BanderaInexistente { nombre: String },

    #[error("se alcanzo el limite de {limite} instrucciones")]
    LimiteDeInstrucciones { limite: u64 },

    #[error("la linea de interrupcion {linea} no tiene un vector declarado")]
    InterrupcionNoDeclarada { linea: u32 },

    #[error("no se puede atender la interrupcion {linea}: la maquina no tiene puntero de pila")]
    InterrupcionSinPila { linea: u32 },

    #[error("la maquina no tiene un dispositivo llamado '{nombre}'")]
    DispositivoInexistente { nombre: String },

    #[error("el dispositivo '{nombre}' no es {esperado}")]
    DispositivoIncorrecto {
        nombre: String,
        esperado: &'static str,
    },

    #[error("la pantalla '{nombre}' usa el formato '{formato}', que el emulador no sabe dibujar")]
    FormatoDePantallaNoSoportado { nombre: String, formato: String },
}
