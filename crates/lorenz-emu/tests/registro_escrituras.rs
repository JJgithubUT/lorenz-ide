//! Registro de las escrituras de las instrucciones, con el valor anterior (RF-DEP-024).

mod comun;

use comun::{con_tres_regiones, ejemplo_8};
use lorenz_emu::{Emulador, EscrituraRegistrada, Memoria};

fn memoria() -> Memoria {
    Memoria::nueva(&con_tres_regiones()).unwrap()
}

#[test]
fn apagado_no_anota() {
    let mut m = memoria();
    m.escribir(0x100, 1).unwrap();
    assert!(m.tomar_escrituras().is_empty());
}

#[test]
fn anota_el_valor_anterior_y_el_nuevo() {
    let mut m = memoria();
    m.registrar_escrituras(true);
    m.fijar_instruccion_actual(0x20);
    m.escribir(0x100, 5).unwrap();
    m.escribir(0x100, 7).unwrap();
    assert_eq!(
        m.tomar_escrituras(),
        vec![
            EscrituraRegistrada {
                direccion: 0x100,
                antes: 0,
                despues: 5,
                pc: 0x20
            },
            EscrituraRegistrada {
                direccion: 0x100,
                antes: 5,
                despues: 7,
                pc: 0x20
            },
        ]
    );
    assert!(m.tomar_escrituras().is_empty());
}

#[test]
fn lo_que_escribe_el_depurador_no_cuenta() {
    let mut m = memoria();
    m.registrar_escrituras(true);
    m.escribir_privilegiado(0x100, 5).unwrap();
    m.cargar_bloque(0x100, &[1, 2]).unwrap();
    assert!(m.tomar_escrituras().is_empty());
}

#[test]
fn una_escritura_rechazada_no_se_anota() {
    let mut m = memoria();
    m.registrar_escrituras(true);
    assert!(m.escribir(0x0, 1).is_err());
    assert!(m.tomar_escrituras().is_empty());
}

#[test]
fn apagarlo_olvida_lo_pendiente() {
    let mut m = memoria();
    m.registrar_escrituras(true);
    m.escribir(0x100, 5).unwrap();
    m.registrar_escrituras(false);
    assert!(m.tomar_escrituras().is_empty());
}

#[test]
fn el_emulador_anota_la_escritura_de_una_instruccion() {
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    e.memoria_mut().registrar_escrituras(true);
    // ST [0x2000], A ; HLT
    e.cargar_programa(0, &[0x50, 0x00, 0x20, 0xF0]).unwrap();
    e.paso().unwrap();
    let w = e.memoria_mut().tomar_escrituras();
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].direccion, 0x2000);
    assert_eq!(w[0].pc, 0);
}
