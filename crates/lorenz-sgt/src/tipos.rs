//! Los tipos de SGT.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tipo {
    Entero {
        bits: u32,
        signo: bool,
    },
    Bool,
    Puntero(Box<Tipo>),
    Arreglo(Box<Tipo>, u64),
    /// Indice en la tabla de estructuras del modulo.
    Estructura(usize),
    Vacio,
    /// Un tipo que ya dio error; no genera mas diagnosticos.
    Error,
}

impl Tipo {
    pub fn u(bits: u32) -> Tipo {
        Tipo::Entero { bits, signo: false }
    }

    pub fn es_entero(&self) -> bool {
        matches!(self, Tipo::Entero { .. })
    }

    pub fn es_escalar(&self) -> bool {
        matches!(self, Tipo::Entero { .. } | Tipo::Bool | Tipo::Puntero(_))
    }

    pub fn es_error(&self) -> bool {
        matches!(self, Tipo::Error)
    }
}

impl fmt::Display for Tipo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tipo::Entero { bits, signo } => write!(f, "{}{}", if *signo { 'i' } else { 'u' }, bits),
            Tipo::Bool => f.write_str("bool"),
            Tipo::Puntero(t) => write!(f, "*{t}"),
            Tipo::Arreglo(t, n) => write!(f, "[{t}; {n}]"),
            Tipo::Estructura(i) => write!(f, "estructura#{i}"),
            Tipo::Vacio => f.write_str("()"),
            Tipo::Error => f.write_str("?"),
        }
    }
}

/// Rango de valores de un entero de `bits` bits.
pub fn rango(bits: u32, signo: bool) -> (i128, i128) {
    if signo {
        (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
    } else {
        (0, (1i128 << bits) - 1)
    }
}

/// Lleva un valor a `bits` bits dando la vuelta, como lo hace la maquina (RF-SGT-020).
pub fn envolver(valor: i128, bits: u32, signo: bool) -> i128 {
    let m = (1i128 << bits) - 1;
    let v = valor & m;
    if signo && v >> (bits - 1) == 1 {
        v - (1i128 << bits)
    } else {
        v
    }
}
