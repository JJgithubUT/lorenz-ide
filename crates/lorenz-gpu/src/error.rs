use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorGpu {
    /// La cola de comandos esta llena.
    ColaLlena {
        profundidad: usize,
    },
    /// Codigo de operacion desconocido.
    OperacionDesconocida {
        codigo: i64,
    },
    /// El comando necesita mas palabras de las que hay.
    ComandoIncompleto {
        operacion: &'static str,
        faltan: usize,
    },
    /// Error al leer un guion o una configuracion.
    Texto {
        linea: usize,
        mensaje: String,
    },
    Config(String),
    /// El kernel no termino dentro del limite de instrucciones de warp.
    LimiteDeEmisiones {
        limite: u64,
    },
}

impl fmt::Display for ErrorGpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ColaLlena { profundidad } => {
                write!(f, "la cola de comandos esta llena ({profundidad} comandos)")
            }
            Self::OperacionDesconocida { codigo } => {
                write!(f, "la operacion {codigo} no existe en la unidad grafica")
            }
            Self::ComandoIncompleto { operacion, faltan } => {
                write!(f, "a `{operacion}` le faltan {faltan} argumentos")
            }
            Self::Texto { linea, mensaje } => write!(f, "linea {linea}: {mensaje}"),
            Self::Config(m) => write!(f, "{m}"),
            Self::LimiteDeEmisiones { limite } => {
                write!(
                    f,
                    "el kernel no termino tras {limite} instrucciones de warp"
                )
            }
        }
    }
}

impl std::error::Error for ErrorGpu {}
