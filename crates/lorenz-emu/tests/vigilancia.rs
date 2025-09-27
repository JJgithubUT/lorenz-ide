//! Vigilancia de accesos a memoria para puntos de observacion (RF-DEP-014, RF-DEP-015).

mod comun;

use comun::{con_tres_regiones, ejemplo_8};
use lorenz_emu::{AccesoVigilado, Emulador, Memoria, TipoAcceso};

fn memoria() -> Memoria {
    Memoria::nueva(&con_tres_regiones()).unwrap()
}

#[test]
fn sin_vigilar_no_se_anota_nada() {
    let mut m = memoria();
    m.escribir(0x100, 1).unwrap();
    assert!(m.tomar_accesos().is_empty());
}

#[test]
fn una_escritura_vigilada_se_anota_con_la_instruccion() {
    let mut m = memoria();
    m.vigilar(0x100, TipoAcceso::Escritura);
    m.fijar_instruccion_actual(0x42);
    m.escribir(0x100, 9).unwrap();
    assert_eq!(
        m.tomar_accesos(),
        vec![AccesoVigilado {
            direccion: 0x100,
            tipo: TipoAcceso::Escritura,
            pc: 0x42
        }]
    );
}

#[test]
fn escribir_el_mismo_valor_tambien_cuenta() {
    let mut m = memoria();
    m.vigilar(0x100, TipoAcceso::Escritura);
    m.escribir(0x100, 0).unwrap();
    assert_eq!(m.tomar_accesos().len(), 1);
}

#[test]
fn una_escritura_en_otra_celda_no_se_anota() {
    let mut m = memoria();
    m.vigilar(0x100, TipoAcceso::Escritura);
    m.escribir(0x101, 1).unwrap();
    assert!(m.tomar_accesos().is_empty());
}

#[test]
fn una_lectura_vigilada_se_anota_pero_una_escritura_no() {
    let mut m = memoria();
    m.vigilar(0x100, TipoAcceso::Lectura);
    m.escribir(0x100, 3).unwrap();
    assert!(m.tomar_accesos().is_empty());
    m.leer_vigilada(0x100).unwrap();
    let a = m.tomar_accesos();
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].tipo, TipoAcceso::Lectura);
}

#[test]
fn lo_que_hace_el_depurador_no_cuenta() {
    let mut m = memoria();
    m.vigilar(0x100, TipoAcceso::Escritura);
    m.vigilar(0x100, TipoAcceso::Lectura);
    m.escribir_privilegiado(0x100, 5).unwrap();
    m.leer_privilegiado(0x100).unwrap();
    assert!(m.tomar_accesos().is_empty());
}

#[test]
fn tomar_los_accesos_los_olvida() {
    let mut m = memoria();
    m.vigilar(0x100, TipoAcceso::Escritura);
    m.escribir(0x100, 1).unwrap();
    assert_eq!(m.tomar_accesos().len(), 1);
    assert!(m.tomar_accesos().is_empty());
}

#[test]
fn dejar_de_vigilar_funciona_y_avisa_si_no_se_vigilaba() {
    let mut m = memoria();
    m.vigilar(0x100, TipoAcceso::Escritura);
    assert_eq!(m.vigiladas(TipoAcceso::Escritura), vec![0x100]);
    assert!(m.dejar_de_vigilar(0x100, TipoAcceso::Escritura));
    assert!(!m.dejar_de_vigilar(0x100, TipoAcceso::Escritura));
    m.escribir(0x100, 1).unwrap();
    assert!(m.tomar_accesos().is_empty());
}

#[test]
fn un_programa_que_escribe_dispara_la_vigilancia_con_su_direccion() {
    // ST [0x2000], A ; HLT
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    e.cargar_programa(0, &[0x50, 0x00, 0x20, 0xF0]).unwrap();
    e.memoria_mut().vigilar(0x2000, TipoAcceso::Escritura);
    e.paso().unwrap();
    let a = e.memoria_mut().tomar_accesos();
    assert_eq!(a.len(), 1);
    assert_eq!((a[0].direccion, a[0].pc), (0x2000, 0));
}

#[test]
fn un_programa_que_lee_dispara_la_vigilancia_de_lectura() {
    // LD A, [0x2000] ; HLT
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    e.cargar_programa(0, &[0x40, 0x00, 0x20, 0xF0]).unwrap();
    e.memoria_mut().vigilar(0x2000, TipoAcceso::Lectura);
    e.paso().unwrap();
    let a = e.memoria_mut().tomar_accesos();
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].tipo, TipoAcceso::Lectura);
}
