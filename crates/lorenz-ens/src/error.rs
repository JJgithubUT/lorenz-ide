//! Errores del ensamblador, siempre con archivo, linea y columna (RF-ENS-014).

use std::fmt;

use crate::expr::ErrorExpr;

/// Que salio mal, explicado en terminos de la maquina.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Causa {
    /// El texto no encaja con la gramatica de una linea.
    Sintaxis(String),
    /// Ninguna instruccion de la maquina se llama asi.
    MnemonicoDesconocido(String),
    /// El mnemonico existe pero ninguna de sus sintaxis encaja con los operandos escritos.
    OperandosNoEncajan {
        mnemonico: String,
        formas: Vec<String>,
    },
    /// Se esperaba uno de los registros permitidos.
    RegistroInvalido {
        operando: String,
        escrito: String,
        permitidos: Vec<String>,
    },
    Expresion(ErrorExpr),
    /// Un inmediato fuera del rango declarado por el operando (RF-ENS-011).
    InmediatoFueraDeRango {
        operando: String,
        valor: i128,
        minimo: i64,
        maximo: i64,
    },
    /// Un valor que no cabe en los bits del campo de la codificacion.
    NoCabeEnElCampo {
        operando: String,
        valor: i128,
        bits: u32,
    },
    /// Direccion fuera del espacio direccionable de la maquina.
    DireccionFueraDeRango {
        valor: i128,
        maximo: u64,
    },
    /// Salto relativo demasiado lejos (RF-ENS-012).
    SaltoFueraDeAlcance {
        desplazamiento: i128,
        minimo: i128,
        maximo: i128,
    },
    EtiquetaDuplicada {
        nombre: String,
        previa: usize,
    },
    DirectivaDesconocida(String),
    ArgumentosDeDirectiva {
        directiva: String,
        esperado: String,
    },
    /// Un dato que no cabe en una palabra de la maquina.
    DatoFueraDeRango {
        valor: i128,
        bits: u32,
    },
    /// El codigo cae fuera de toda region o en una region que no permite escribirlo (RF-ENS-013).
    SinRegion {
        direccion: u64,
    },
    RegionNoEscribible {
        direccion: u64,
        region: String,
    },
    /// Dos fragmentos del programa escriben la misma direccion.
    Solape {
        direccion: u64,
    },
    /// `.org` hacia atras o fuera de espacio.
    OrigenInvalido {
        valor: i128,
    },
    NoSePudoLeer {
        archivo: String,
        motivo: String,
    },
    InclusionCircular {
        archivo: String,
    },
    Otra(String),
}

impl fmt::Display for Causa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Causa::Sintaxis(m) => write!(f, "{m}"),
            Causa::MnemonicoDesconocido(m) => {
                write!(f, "la maquina no tiene ninguna instruccion llamada `{m}`")
            }
            Causa::OperandosNoEncajan { mnemonico, formas } => write!(
                f,
                "los operandos no encajan con ninguna forma de `{mnemonico}`; formas validas: {}",
                formas.join(" | ")
            ),
            Causa::RegistroInvalido {
                operando,
                escrito,
                permitidos,
            } => write!(
                f,
                "`{escrito}` no es un registro valido para `{operando}`; se admiten: {}",
                permitidos.join(", ")
            ),
            Causa::Expresion(e) => write!(f, "{e}"),
            Causa::InmediatoFueraDeRango {
                operando,
                valor,
                minimo,
                maximo,
            } => write!(
                f,
                "el valor {valor} no cabe en `{operando}`: la instruccion admite de {minimo} a {maximo}"
            ),
            Causa::NoCabeEnElCampo {
                operando,
                valor,
                bits,
            } => write!(
                f,
                "el valor {valor} no cabe en los {bits} bits que la codificacion reserva a `{operando}`"
            ),
            Causa::DireccionFueraDeRango { valor, maximo } => write!(
                f,
                "la direccion {valor} esta fuera del espacio de la maquina (maximo {maximo:#x})"
            ),
            Causa::SaltoFueraDeAlcance {
                desplazamiento,
                minimo,
                maximo,
            } => write!(
                f,
                "el salto necesita un desplazamiento de {desplazamiento} y el campo solo llega de {minimo} a {maximo}"
            ),
            Causa::EtiquetaDuplicada { nombre, previa } => {
                write!(f, "el simbolo `{nombre}` ya se definio en la linea {previa}")
            }
            Causa::DirectivaDesconocida(d) => write!(f, "directiva desconocida `{d}`"),
            Causa::ArgumentosDeDirectiva {
                directiva,
                esperado,
            } => write!(f, "la directiva `{directiva}` espera {esperado}"),
            Causa::DatoFueraDeRango { valor, bits } => {
                write!(f, "el dato {valor} no cabe en una palabra de {bits} bits")
            }
            Causa::SinRegion { direccion } => write!(
                f,
                "la direccion {direccion:#x} no pertenece a ninguna region de memoria"
            ),
            Causa::RegionNoEscribible { direccion, region } => write!(
                f,
                "la direccion {direccion:#x} esta en la region `{region}`, que no permite escribir"
            ),
            Causa::Solape { direccion } => {
                write!(f, "dos fragmentos escriben la direccion {direccion:#x}")
            }
            Causa::OrigenInvalido { valor } => {
                write!(f, "el origen {valor} no es una direccion valida")
            }
            Causa::NoSePudoLeer { archivo, motivo } => {
                write!(f, "no se pudo leer `{archivo}`: {motivo}")
            }
            Causa::InclusionCircular { archivo } => {
                write!(f, "`{archivo}` se incluye a si mismo, directa o indirectamente")
            }
            Causa::Otra(m) => write!(f, "{m}"),
        }
    }
}

/// Un error con su posicion en el fuente. Columna y linea empiezan en 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorEns {
    pub archivo: String,
    pub linea: usize,
    pub columna: usize,
    pub causa: Causa,
}

impl fmt::Display for ErrorEns {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.archivo, self.linea, self.columna, self.causa
        )
    }
}

impl std::error::Error for ErrorEns {}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn el_mensaje_lleva_archivo_linea_y_columna() {
        let e = ErrorEns {
            archivo: "a.s".into(),
            linea: 3,
            columna: 5,
            causa: Causa::MnemonicoDesconocido("FOO".into()),
        };
        assert_eq!(
            e.to_string(),
            "a.s:3:5: la maquina no tiene ninguna instruccion llamada `FOO`"
        );
    }

    #[test]
    fn el_rango_se_explica_con_los_limites_de_la_isa() {
        let c = Causa::InmediatoFueraDeRango {
            operando: "imm".into(),
            valor: 300,
            minimo: 0,
            maximo: 255,
        };
        assert!(c.to_string().contains("de 0 a 255"));
    }

    #[test]
    fn el_salto_indica_el_alcance_maximo() {
        let c = Causa::SaltoFueraDeAlcance {
            desplazamiento: 200,
            minimo: -128,
            maximo: 127,
        };
        assert!(c.to_string().contains("-128 a 127"));
    }
}
