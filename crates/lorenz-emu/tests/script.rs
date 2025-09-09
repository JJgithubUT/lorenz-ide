//! Dispositivos genericos con script (RF-EMU-035).

mod comun;

use comun::con_script;
use lorenz_emu::{Emulador, ErrorEjecucion};

const ENTRADA: u64 = 0x30;
const SALIDA: u64 = 0x31;

fn emulador(script: &str) -> Emulador {
    Emulador::nuevo(con_script(script)).unwrap()
}

#[test]
fn escribir_en_un_puerto_ejecuta_el_script() {
    let mut e = emulador("salida = valor + 1");
    e.puertos_mut().escribir(ENTRADA, 5).unwrap();
    assert_eq!(e.puertos_mut().leer(SALIDA).unwrap(), 6);
}

#[test]
fn un_puerto_que_el_script_no_asigna_vale_cero() {
    let mut e = emulador("salida = valor");
    e.puertos_mut().escribir(ENTRADA, 5).unwrap();
    assert_eq!(e.puertos_mut().leer(ENTRADA).unwrap(), 0);
}

#[test]
fn leer_un_puerto_no_lo_consume() {
    let mut e = emulador("salida = valor");
    e.puertos_mut().escribir(ENTRADA, 9).unwrap();
    assert_eq!(e.puertos_mut().leer(SALIDA).unwrap(), 9);
    assert_eq!(e.puertos_mut().leer(SALIDA).unwrap(), 9);
}

#[test]
fn el_resultado_se_trunca_al_ancho_de_palabra() {
    let mut e = emulador("salida = valor * 100");
    e.puertos_mut().escribir(ENTRADA, 3).unwrap();
    assert_eq!(e.puertos_mut().leer(SALIDA).unwrap(), 300 & 0xFF);
}

#[test]
fn el_script_recuerda_lo_de_la_vez_anterior() {
    let mut e = emulador("salida = salida + valor");
    for v in [1, 2, 3] {
        e.puertos_mut().escribir(ENTRADA, v).unwrap();
    }
    assert_eq!(e.puertos_mut().leer(SALIDA).unwrap(), 6);
}

#[test]
fn un_script_ve_en_que_puerto_se_escribio() {
    let mut e = emulador("si puerto == 48: salida = 1; si puerto == 49: salida = 2");
    e.puertos_mut().escribir(ENTRADA, 0).unwrap();
    assert_eq!(e.puertos_mut().leer(SALIDA).unwrap(), 1);
    e.puertos_mut().escribir(SALIDA, 0).unwrap();
    assert_eq!(e.puertos_mut().leer(SALIDA).unwrap(), 2);
}

#[test]
fn un_script_puede_leer_un_puerto_que_acaba_de_asignar() {
    let mut e = emulador("entrada = valor; salida = entrada + 10");
    e.puertos_mut().escribir(ENTRADA, 4).unwrap();
    assert_eq!(e.puertos_mut().leer(ENTRADA).unwrap(), 4);
    assert_eq!(e.puertos_mut().leer(SALIDA).unwrap(), 14);
}

#[test]
fn un_script_vacio_no_hace_nada() {
    let mut e = emulador("");
    e.puertos_mut().escribir(ENTRADA, 5).unwrap();
    assert_eq!(e.puertos_mut().leer(SALIDA).unwrap(), 0);
}

#[test]
fn dividir_entre_cero_en_el_script_es_un_error() {
    let mut e = emulador("salida = 1 / valor");
    assert!(matches!(
        e.puertos_mut().escribir(ENTRADA, 0),
        Err(ErrorEjecucion::DivisionEntreCeroEnScript { .. })
    ));
}

#[test]
fn un_programa_dispara_el_script_con_out() {
    // OUT ; HLT, con A = 7
    let mut e = emulador("salida = valor * 2");
    e.cargar_programa(0, &[0x10, 0xF0]).unwrap();
    e.estado_mut().fijar_registro("A", 7).unwrap();
    e.ejecutar(10).unwrap();
    assert_eq!(e.puertos_mut().leer(SALIDA).unwrap(), 14);
}

#[test]
fn interrumpir_levanta_la_linea_del_dispositivo() {
    let mut e = emulador("interrumpir = 1");
    e.cargar_programa(0, &[0x10, 0xF0]).unwrap();
    e.paso().unwrap();
    assert_eq!(e.interrupciones_pendientes(), [0]);
}

#[test]
fn sin_interrumpir_no_se_levanta_nada() {
    let mut e = emulador("salida = valor");
    e.cargar_programa(0, &[0x10, 0xF0]).unwrap();
    e.paso().unwrap();
    assert!(e.interrupciones_pendientes().is_empty());
}

#[test]
fn interrumpir_condicional_depende_del_valor() {
    let mut e = emulador("si valor > 5: interrumpir = 1");
    e.cargar_programa(0, &[0x10, 0x10, 0xF0]).unwrap();
    e.estado_mut().fijar_registro("A", 3).unwrap();
    e.paso().unwrap();
    assert!(e.interrupciones_pendientes().is_empty());
    e.estado_mut().fijar_registro("A", 9).unwrap();
    e.paso().unwrap();
    assert_eq!(e.interrupciones_pendientes(), [0]);
}

#[test]
fn reiniciar_pone_los_puertos_del_script_a_cero() {
    let mut e = emulador("salida = valor");
    e.puertos_mut().escribir(ENTRADA, 8).unwrap();
    e.reiniciar();
    assert_eq!(e.puertos_mut().leer(SALIDA).unwrap(), 0);
}

#[test]
fn reiniciar_olvida_las_interrupciones_sin_recoger() {
    let mut e = emulador("interrumpir = 1");
    e.puertos_mut().escribir(ENTRADA, 1).unwrap();
    e.reiniciar();
    assert!(e.puertos_mut().tomar_interrupciones().is_empty());
}

#[test]
fn la_instantanea_conserva_el_estado_del_dispositivo() {
    let mut e = emulador("salida = salida + valor");
    e.puertos_mut().escribir(ENTRADA, 5).unwrap();
    let foto = e.instantanea();
    e.puertos_mut().escribir(ENTRADA, 5).unwrap();
    e.restaurar(&foto).unwrap();
    assert_eq!(e.puertos_mut().leer(SALIDA).unwrap(), 5);
}
