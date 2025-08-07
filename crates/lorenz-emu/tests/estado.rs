//! Pruebas del estado de la maquina: registros, banderas y contadores.

mod comun;

use comun::ejemplo_8;
use lorenz_emu::{ErrorEjecucion, Estado};

fn estado() -> Estado {
    Estado::nuevo(&ejemplo_8())
}

#[test]
fn los_registros_arrancan_en_su_valor_inicial() {
    let e = estado();
    assert_eq!(e.registro("A").unwrap(), 0);
    assert_eq!(e.sp(), Some(0xEFFF));
}

#[test]
fn el_pc_arranca_en_la_direccion_de_arranque() {
    let m = ejemplo_8();
    let e = Estado::nuevo(&m);
    assert_eq!(e.pc(), m.arranque);
}

#[test]
fn asignar_un_registro_trunca_a_su_ancho() {
    let mut e = estado();
    e.fijar_registro("A", 0x1FF).unwrap();
    assert_eq!(e.registro("A").unwrap(), 0xFF);
}

#[test]
fn un_registro_de_16_bits_guarda_16_bits() {
    let mut e = estado();
    e.fijar_registro("SP", 0x1_2345).unwrap();
    assert_eq!(e.registro("SP").unwrap(), 0x2345);
}

#[test]
fn un_registro_inexistente_da_error() {
    let mut e = estado();
    assert!(matches!(e.registro("Q"), Err(ErrorEjecucion::RegistroInexistente { .. })));
    assert!(matches!(e.fijar_registro("Q", 1), Err(ErrorEjecucion::RegistroInexistente { .. })));
}

#[test]
fn fijar_el_pc_trunca_a_su_ancho() {
    let mut e = estado();
    e.fijar_pc(0x1_0005);
    assert_eq!(e.pc(), 5);
}

#[test]
fn fijar_el_sp_cambia_el_registro_sp() {
    let mut e = estado();
    e.fijar_sp(0x2100);
    assert_eq!(e.registro("SP").unwrap(), 0x2100);
}

#[test]
fn las_banderas_arrancan_apagadas() {
    let e = estado();
    assert!(!e.bandera("Z").unwrap());
    assert!(!e.bandera("C").unwrap());
}

#[test]
fn encender_una_bandera_no_toca_las_demas() {
    let mut e = estado();
    e.fijar_bandera("C", true).unwrap();
    assert!(e.bandera("C").unwrap());
    assert!(!e.bandera("Z").unwrap());
    assert_eq!(e.registro("F").unwrap(), 0b10);
}

#[test]
fn apagar_una_bandera_no_toca_las_demas() {
    let mut e = estado();
    e.fijar_bandera("Z", true).unwrap();
    e.fijar_bandera("C", true).unwrap();
    e.fijar_bandera("Z", false).unwrap();
    assert_eq!(e.registro("F").unwrap(), 0b10);
}

#[test]
fn una_bandera_inexistente_da_error() {
    let mut e = estado();
    assert!(matches!(e.bandera("N"), Err(ErrorEjecucion::BanderaInexistente { .. })));
    assert!(matches!(e.fijar_bandera("N", true), Err(ErrorEjecucion::BanderaInexistente { .. })));
}

#[test]
fn los_nombres_de_banderas_son_los_declarados() {
    let mut nombres = estado().nombres_de_banderas().into_iter().map(String::from).collect::<Vec<_>>();
    nombres.sort();
    assert_eq!(nombres, ["C", "Z"]);
}

#[test]
fn contar_instrucciones_acumula_ciclos() {
    let mut e = estado();
    e.contar_instruccion(2);
    e.contar_instruccion(3);
    assert_eq!(e.instrucciones(), 2);
    assert_eq!(e.ciclos(), 5);
}

#[test]
fn detener_marca_la_maquina_como_detenida() {
    let mut e = estado();
    assert!(!e.detenida());
    e.detener();
    assert!(e.detenida());
}

#[test]
fn las_interrupciones_se_habilitan_y_deshabilitan() {
    let mut e = estado();
    e.habilitar_interrupciones(true);
    assert!(e.interrupciones_habilitadas());
    e.habilitar_interrupciones(false);
    assert!(!e.interrupciones_habilitadas());
}

#[test]
fn reiniciar_restaura_registros_banderas_y_contadores() {
    let m = ejemplo_8();
    let mut e = Estado::nuevo(&m);
    e.fijar_registro("A", 9).unwrap();
    e.fijar_sp(1);
    e.fijar_pc(77);
    e.fijar_bandera("Z", true).unwrap();
    e.contar_instruccion(4);
    e.detener();
    e.habilitar_interrupciones(true);
    e.reiniciar();
    assert_eq!(e.registro("A").unwrap(), 0);
    assert_eq!(e.sp(), Some(0xEFFF));
    assert_eq!(e.pc(), m.arranque);
    assert!(!e.bandera("Z").unwrap());
    assert_eq!((e.instrucciones(), e.ciclos()), (0, 0));
    assert!(!e.detenida());
    assert!(!e.interrupciones_habilitadas());
}
