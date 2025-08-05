//! Utilidades compartidas por las pruebas del emulador.
#![allow(dead_code)]

use lorenz_arch::modelo::Maquina;

/// La maquina de ejemplo de 8 bits del repositorio.
pub fn ejemplo_8() -> Maquina {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    lorenz_arch::cargar(&texto).expect("ejemplo-8 debe cargar")
}

/// Una maquina minima con tres regiones: rom (r), ram (rw) y buzon (w).
pub fn con_tres_regiones() -> Maquina {
    let texto = r#"
[maquina]
nombre = "tres"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[[registro]]
nombre = "PC"
bits = 16
rol = "pc"

[[instruccion]]
mnemonico = "HLT"
codigo = "1111 ...."
semantica = "detener"

[[region]]
nombre = "rom"
desde = 0x0000
hasta = 0x00FF
acceso = "r"

[[region]]
nombre = "ram"
desde = 0x0100
hasta = 0x01FF
acceso = "rw"

[[region]]
nombre = "buzon"
desde = 0x0300
hasta = 0x030F
acceso = "w"
"#;
    lorenz_arch::cargar(texto).expect("la maquina de prueba debe cargar")
}
