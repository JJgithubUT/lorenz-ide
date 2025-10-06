//! Un numero escrito en un archivo de pruebas: entero de TOML o texto (`"0xFF"`, `"0b101"`).
//!
//! TOML solo guarda enteros de 64 bits con signo; el texto permite palabras de 64 bits completas.

use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Num(pub u64);

/// Lee `42`, `0x2A`, `0b101010` o `1_000`.
pub fn analizar(texto: &str) -> Result<u64, String> {
    let t = texto.trim().replace('_', "").to_ascii_lowercase();
    let r = if let Some(h) = t.strip_prefix("0x") {
        u64::from_str_radix(h, 16)
    } else if let Some(b) = t.strip_prefix("0b") {
        u64::from_str_radix(b, 2)
    } else {
        t.parse::<u64>()
    };
    r.map_err(|_| format!("`{}` no es un numero valido", texto.trim()))
}

impl<'de> Deserialize<'de> for Num {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = Num;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("un numero entero o un texto como \"0xFF\"")
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Num, E> {
                u64::try_from(v).map(Num).map_err(|_| {
                    E::custom(format!("{v} es negativo; se esperaba un numero sin signo"))
                })
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Num, E> {
                Ok(Num(v))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Num, E> {
                analizar(v).map(Num).map_err(E::custom)
            }
        }
        d.deserialize_any(V)
    }
}
