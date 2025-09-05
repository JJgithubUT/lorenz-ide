//! Pausa y reanudacion (RF-EMU-013).

mod comun;

use comun::ejemplo_8;
use lorenz_emu::{Emulador, ErrorEjecucion};

fn con_programa(palabras: &[u64]) -> Emulador {
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    e.cargar_programa(0, palabras).unwrap();
    e
}

#[test]
fn una_maquina_nueva_no_esta_en_pausa() {
    assert!(!con_programa(&[0xF0]).en_pausa());
}

#[test]
fn pausar_y_reanudar_cambian_el_indicador() {
    let mut e = con_programa(&[0xF0]);
    e.pausar();
    assert!(e.en_pausa());
    e.reanudar();
    assert!(!e.en_pausa());
}

#[test]
fn ejecutar_en_pausa_se_niega() {
    let mut e = con_programa(&[0xF0]);
    e.pausar();
    assert_eq!(e.ejecutar(10), Err(ErrorEjecucion::EnPausa));
    assert!(!e.estado().detenida());
}

#[test]
fn avanzar_en_pausa_se_niega() {
    let mut e = con_programa(&[0xF0]);
    e.pausar();
    assert_eq!(e.avanzar(1), Err(ErrorEjecucion::EnPausa));
}

#[test]
fn un_paso_suelto_funciona_en_pausa() {
    let mut e = con_programa(&[0x20, 7, 0xF0]);
    e.pausar();
    let p = e.paso().unwrap().unwrap();
    assert_eq!(p.mnemonico, "LDI");
    assert!(e.en_pausa());
}

#[test]
fn al_reanudar_sigue_donde_se_quedo() {
    let mut e = con_programa(&[0x20, 7, 0xF0]);
    e.avanzar(1).unwrap();
    e.pausar();
    e.reanudar();
    e.ejecutar(10).unwrap();
    assert_eq!(e.estado().registro("A").unwrap(), 7);
    assert!(e.estado().detenida());
}

#[test]
fn reiniciar_quita_la_pausa() {
    let mut e = con_programa(&[0xF0]);
    e.pausar();
    e.reiniciar();
    assert!(!e.en_pausa());
}

#[test]
fn el_mensaje_de_error_pide_reanudar() {
    assert!(ErrorEjecucion::EnPausa.to_string().contains("reanudala"));
}
