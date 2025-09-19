//! Utilidades compartidas por las pruebas del ensamblador.
#![allow(dead_code)]

use lorenz_arch::modelo::Maquina;
use lorenz_ens::ensamblador::{Ensamblador, Fuente, Objeto};
use lorenz_ens::error::ErrorEns;

pub fn ejemplo_8() -> Maquina {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    lorenz_arch::cargar(&t).expect("ejemplo-8 debe cargar")
}

pub fn ensamblar(m: &Maquina, texto: &str) -> Result<Objeto, Vec<ErrorEns>> {
    Ensamblador::nuevo(m)
        .unwrap()
        .ensamblar(&Fuente::nueva("prueba.s", texto))
}

/// Las palabras emitidas, en orden de direccion, sin mirar donde caen.
pub fn bytes(m: &Maquina, texto: &str) -> Vec<u64> {
    ensamblar(m, texto)
        .unwrap_or_else(|e| {
            panic!(
                "{}",
                e.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
        .palabras()
        .into_iter()
        .map(|(_, p)| p)
        .collect()
}

pub fn errores(m: &Maquina, texto: &str) -> Vec<ErrorEns> {
    ensamblar(m, texto).expect_err("debia fallar")
}
