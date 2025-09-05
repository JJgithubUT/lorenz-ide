//! Frecuencia objetivo y avance por tiempo emulado (RF-EMU-011).

mod comun;

use comun::ejemplo_8;
use lorenz_emu::{Emulador, ErrorEjecucion};

/// Un bucle infinito de NOP de un ciclo cada uno: `NOP ; NOP ; ... ; HLT`.
fn con_nops(n: usize) -> Emulador {
    let mut programa = vec![0x00; n];
    programa.push(0xF0);
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    e.cargar_programa(0, &programa).unwrap();
    e
}

#[test]
fn por_omision_no_hay_frecuencia() {
    assert_eq!(con_nops(1).frecuencia_hz(), None);
}

#[test]
fn se_puede_fijar_y_quitar_la_frecuencia() {
    let mut e = con_nops(1);
    e.fijar_frecuencia_hz(Some(1000)).unwrap();
    assert_eq!(e.frecuencia_hz(), Some(1000));
    e.fijar_frecuencia_hz(None).unwrap();
    assert_eq!(e.frecuencia_hz(), None);
}

#[test]
fn cero_hercios_se_rechaza() {
    let mut e = con_nops(1);
    assert_eq!(
        e.fijar_frecuencia_hz(Some(0)),
        Err(ErrorEjecucion::FrecuenciaInvalida)
    );
}

#[test]
fn avanzar_por_tiempo_sin_frecuencia_da_error() {
    assert_eq!(
        con_nops(1).avanzar_tiempo(1000),
        Err(ErrorEjecucion::SinFrecuencia)
    );
}

#[test]
fn mil_hercios_durante_un_milisegundo_es_un_ciclo() {
    let mut e = con_nops(10);
    e.fijar_frecuencia_hz(Some(1000)).unwrap();
    let r = e.avanzar_tiempo(1_000_000).unwrap();
    assert_eq!((r.instrucciones, r.ciclos), (1, 1));
}

#[test]
fn un_megahercio_durante_un_milisegundo_son_mil_ciclos() {
    let mut e = con_nops(5000);
    e.fijar_frecuencia_hz(Some(1_000_000)).unwrap();
    let r = e.avanzar_tiempo(1_000_000).unwrap();
    assert_eq!(r.ciclos, 1000);
}

#[test]
fn lo_que_no_llega_a_un_ciclo_se_acumula() {
    let mut e = con_nops(10);
    e.fijar_frecuencia_hz(Some(1000)).unwrap();
    assert_eq!(e.avanzar_tiempo(400_000).unwrap().ciclos, 1);
    // el primer ciclo ya se gasto con credito prestado: hay que devolver 0,6 ms
    assert_eq!(e.avanzar_tiempo(400_000).unwrap().ciclos, 0);
    assert_eq!(e.avanzar_tiempo(400_000).unwrap().ciclos, 1);
}

#[test]
fn muchos_intervalos_pequenos_suman_lo_mismo_que_uno_grande() {
    let mut a = con_nops(5000);
    let mut b = con_nops(5000);
    a.fijar_frecuencia_hz(Some(1_000_000)).unwrap();
    b.fijar_frecuencia_hz(Some(1_000_000)).unwrap();
    let grande = a.avanzar_tiempo(2_000_000).unwrap().ciclos;
    let mut suma = 0;
    for _ in 0..200 {
        suma += b.avanzar_tiempo(10_000).unwrap().ciclos;
    }
    assert_eq!(suma, grande);
}

#[test]
fn se_detiene_al_llegar_al_hlt() {
    let mut e = con_nops(2);
    e.fijar_frecuencia_hz(Some(1_000_000)).unwrap();
    let r = e.avanzar_tiempo(1_000_000).unwrap();
    assert_eq!(r.instrucciones, 3);
    assert!(e.estado().detenida());
}

#[test]
fn en_pausa_no_avanza_por_tiempo() {
    let mut e = con_nops(2);
    e.fijar_frecuencia_hz(Some(1000)).unwrap();
    e.pausar();
    assert_eq!(e.avanzar_tiempo(1_000_000), Err(ErrorEjecucion::EnPausa));
}

#[test]
fn cambiar_la_frecuencia_borra_el_credito() {
    let mut e = con_nops(10);
    e.fijar_frecuencia_hz(Some(1000)).unwrap();
    e.avanzar_tiempo(400_000).unwrap();
    e.fijar_frecuencia_hz(Some(1000)).unwrap();
    assert_eq!(e.avanzar_tiempo(400_000).unwrap().ciclos, 1);
}
