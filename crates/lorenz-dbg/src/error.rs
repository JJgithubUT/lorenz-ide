//! Errores del depurador, en terminos de la maquina y de lo que el usuario escribio.

use lorenz_emu::ErrorEjecucion;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorDepurador {
    #[error("no se que ubicacion es `{0}`; usa una direccion, una etiqueta o archivo:linea")]
    UbicacionDesconocida(String),

    #[error(
        "la linea {linea} de `{archivo}` no tiene codigo ni la sigue ninguna linea con codigo"
    )]
    LineaSinCodigo { archivo: String, linea: usize },

    #[error("`{0}` no es un numero")]
    NumeroInvalido(String),

    #[error("`{0}` no es ni un registro ni una bandera de la maquina")]
    NombreDesconocido(String),

    #[error("la maquina no tiene puntero de pila")]
    SinPila,

    #[error("la maquina esta detenida: no hay ninguna instruccion que explicar")]
    MaquinaDetenida,

    #[error("la traza no esta grabando; activala con `traza on`")]
    SinTraza,

    #[error("el instante {instante} no existe: la traza llega hasta el {maximo}")]
    InstanteInvalido { instante: u64, maximo: u64 },

    #[error("no se puede reproducir el fallo: {0}")]
    Reproduccion(String),

    #[error("no hay ningun punto con el numero {0}")]
    PuntoInexistente(u32),

    #[error("condicion invalida: {0}")]
    Condicion(String),

    #[error("la orden `{0}` no existe; escribe `ayuda` para ver las ordenes")]
    OrdenDesconocida(String),

    #[error("a `{orden}` le falta {falta}")]
    FaltaArgumento {
        orden: &'static str,
        falta: &'static str,
    },

    #[error("a `{orden}` le sobra `{sobra}`")]
    SobraArgumento { orden: &'static str, sobra: String },

    #[error("{0}")]
    Maquina(#[from] ErrorEjecucion),
}
