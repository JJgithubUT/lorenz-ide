//! Pares de registros usados como un registro ancho (RF-ARQ-010).

use lorenz_emu::Emulador;

const MAQUINA: &str = r#"
[maquina]
nombre = "con-pares"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[[registro]]
nombre = "H"
bits = 8
rol = "general"

[[registro]]
nombre = "L"
bits = 8
rol = "general"

[[registro]]
nombre = "PC"
bits = 16
rol = "pc"

[[par]]
nombre = "HL"
alto = "H"
bajo = "L"

[[instruccion]]
mnemonico = "LDP"
sintaxis = "LDP {imm}"
codigo = "0011 .... | imm:8 | imm:8"
semantica = "HL = imm"
[[instruccion.operando]]
nombre = "imm"
tipo = "inmediato"
signo = false
rango = [0, 65535]

[[instruccion]]
mnemonico = "INCP"
sintaxis = "INCP"
codigo = "0100 ...."
semantica = "HL = HL + 1"

[[instruccion]]
mnemonico = "CPL"
sintaxis = "CPL {rd}"
codigo = "0101 000 rd:1"
semantica = "L = rd"
[[instruccion.operando]]
nombre = "rd"
tipo = "registro"
de = ["H", "HL"]

[[instruccion]]
mnemonico = "HLT"
sintaxis = "HLT"
codigo = "1111 ...."
semantica = "detener"

[[region]]
nombre = "ram"
desde = 0x0000
hasta = 0xFFFF
acceso = "rw"
"#;

fn emulador(programa: &[u64]) -> Emulador {
    let m = lorenz_arch::cargar(MAQUINA).unwrap();
    let mut e = Emulador::nuevo(m).unwrap();
    e.cargar_programa(0, programa).unwrap();
    e
}

#[test]
fn escribir_el_par_reparte_el_valor_entre_sus_registros() {
    let mut e = emulador(&[0x30, 0x34, 0x12, 0xF0]);
    e.ejecutar(10).unwrap();
    assert_eq!(e.estado().registro("H").unwrap(), 0x12);
    assert_eq!(e.estado().registro("L").unwrap(), 0x34);
    assert_eq!(e.estado().registro("HL").unwrap(), 0x1234);
}

#[test]
fn el_acarreo_del_registro_bajo_pasa_al_alto() {
    let mut e = emulador(&[0x30, 0xFF, 0x00, 0x40, 0xF0]);
    e.ejecutar(10).unwrap();
    assert_eq!(e.estado().registro("H").unwrap(), 0x01);
    assert_eq!(e.estado().registro("L").unwrap(), 0x00);
}

#[test]
fn el_par_desborda_con_el_ancho_de_los_dos_registros() {
    let mut e = emulador(&[0x30, 0xFF, 0xFF, 0x40, 0xF0]);
    e.ejecutar(10).unwrap();
    assert_eq!(e.estado().registro("HL").unwrap(), 0);
}

#[test]
fn el_par_vale_como_operando_de_registro() {
    // CPL HL copia en L el valor de 16 bits del par, truncado a 8 bits.
    let mut e = emulador(&[0x30, 0x34, 0x12, 0x51, 0xF0]);
    e.ejecutar(10).unwrap();
    assert_eq!(e.estado().registro("L").unwrap(), 0x34);
}

#[test]
fn el_par_tiene_la_mascara_de_ambos_registros() {
    let e = emulador(&[0xF0]);
    assert_eq!(e.estado().mascara_de_registro("HL").unwrap(), 0xFFFF);
}

#[test]
fn leer_un_par_sin_escribir_avisa_de_sus_dos_registros() {
    let mut e = emulador(&[0x40, 0xF0]);
    e.estado_mut().activar_seguimiento();
    e.paso().unwrap();
    let mut leidos: Vec<String> = e
        .estado_mut()
        .tomar_lecturas_sin_inicializar()
        .into_iter()
        .map(|l| l.registro)
        .collect();
    leidos.sort();
    assert_eq!(leidos, vec!["H", "L"]);
}
