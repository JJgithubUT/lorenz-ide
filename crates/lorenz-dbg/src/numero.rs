//! Numeros escritos por el usuario: decimal, `0x` hexadecimal y `0b` binario.

use crate::error::ErrorDepurador;

pub fn analizar(texto: &str) -> Result<u64, ErrorDepurador> {
    let t = texto.trim().replace('_', "");
    let malo = || ErrorDepurador::NumeroInvalido(texto.trim().to_string());
    let minusculas = t.to_ascii_lowercase();
    if let Some(h) = minusculas.strip_prefix("0x") {
        u64::from_str_radix(h, 16).map_err(|_| malo())
    } else if let Some(b) = minusculas.strip_prefix("0b") {
        u64::from_str_radix(b, 2).map_err(|_| malo())
    } else {
        minusculas.parse::<u64>().map_err(|_| malo())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn tres_bases() {
        assert_eq!(analizar("42"), Ok(42));
        assert_eq!(analizar("0x2A"), Ok(42));
        assert_eq!(analizar("0b101010"), Ok(42));
        assert_eq!(analizar(" 1_000 "), Ok(1000));
    }

    #[test]
    fn lo_que_no_es_un_numero() {
        assert!(analizar("").is_err());
        assert!(analizar("abc").is_err());
        assert!(analizar("0xZZ").is_err());
        assert!(analizar("-1").is_err());
    }
}
