//! Si un registro declara su valor inicial o no (RF-DEP-040).

use lorenz_arch::cargar;

fn maquina(a: &str) -> lorenz_arch::modelo::Maquina {
    let t = format!(
        r#"
[maquina]
nombre = "inicial"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[[registro]]
nombre = "A"
bits = 8
{a}

[[registro]]
nombre = "PC"
bits = 16
rol = "pc"

[[region]]
nombre = "ram"
desde = 0x0000
hasta = 0xFFFF
acceso = "rw"

[[instruccion]]
mnemonico = "NOP"
codigo = "0000 0000"
"#
    );
    cargar(&t).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn sin_inicial_el_registro_arranca_en_cero_sin_estar_declarado() {
    let m = maquina("");
    assert_eq!(m.registros[0].inicial, 0);
    assert!(!m.registros[0].inicial_declarado);
}

#[test]
fn un_inicial_cero_explicito_cuenta_como_declarado() {
    let m = maquina("inicial = 0");
    assert_eq!(m.registros[0].inicial, 0);
    assert!(m.registros[0].inicial_declarado);
}

#[test]
fn un_inicial_distinto_de_cero_se_conserva() {
    let m = maquina("inicial = 7");
    assert_eq!(m.registros[0].inicial, 7);
    assert!(m.registros[0].inicial_declarado);
}
