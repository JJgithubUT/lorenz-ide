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

/// Una maquina big endian con salto relativo de 8 bits y una direccion repartida en dos palabras.
pub fn con_saltos_relativos(endianness: &str) -> Maquina {
    let t = format!(
        r#"
[maquina]
nombre = "rel"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
endianness = "{endianness}"
arranque = 0x0000

[[registro]]
nombre = "PC"
bits = 16
rol = "pc"

[[instruccion]]
mnemonico = "NOP"
codigo = "0000 ...."
semantica = ""

[[instruccion]]
mnemonico = "JR"
sintaxis = "JR {{desp}}"
codigo = "0111 .... | desp:8"
semantica = "pc = pc + desp"
  [[instruccion.operando]]
  nombre = "desp"
  tipo = "desplazamiento"

[[instruccion]]
mnemonico = "JP"
sintaxis = "JP {{dir}}"
codigo = "1000 .... | dir:8 | dir:8"
semantica = "pc = dir"
  [[instruccion.operando]]
  nombre = "dir"
  tipo = "direccion"

[[instruccion]]
mnemonico = "OUTP"
sintaxis = "OUTP {{p}}"
codigo = "1001 .... | p:8"
semantica = "detener"
  [[instruccion.operando]]
  nombre = "p"
  tipo = "puerto"

[[instruccion]]
mnemonico = "ADDI"
sintaxis = "ADDI {{k}}"
codigo = "1010 .... | k:8"
semantica = "detener"
  [[instruccion.operando]]
  nombre = "k"
  tipo = "inmediato"
  signo = true
  rango = [-128, 127]

[[region]]
nombre = "ram"
desde = 0x0000
hasta = 0xFFFF
acceso = "rw"
"#
    );
    lorenz_arch::cargar(&t).expect("la maquina de saltos debe cargar")
}
