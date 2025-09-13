//! Pruebas del seguimiento de escrituras y de lecturas sin inicializar.

mod comun;

use comun::{con_tres_regiones, ejemplo_8};
use lorenz_emu::{Emulador, ErrorEjecucion, Escritura, LecturaSinInicializar, Memoria};

fn memoria() -> Memoria {
    Memoria::nueva(&con_tres_regiones()).unwrap()
}

#[test]
fn el_seguimiento_empieza_apagado() {
    let m = memoria();
    assert!(!m.seguimiento_activo());
    assert_eq!(m.ultima_escritura(0x100).unwrap(), None);
}

#[test]
fn con_el_seguimiento_apagado_todo_cuenta_como_inicializado() {
    assert!(memoria().esta_inicializada(0x100).unwrap());
}

#[test]
fn al_activarlo_lo_que_vale_cero_no_esta_inicializado() {
    let mut m = memoria();
    m.activar_seguimiento();
    assert!(!m.esta_inicializada(0x100).unwrap());
}

#[test]
fn al_activarlo_lo_que_ya_vale_algo_se_da_por_cargado() {
    let mut m = memoria();
    m.escribir_privilegiado(0x100, 7).unwrap();
    m.activar_seguimiento();
    assert!(m.esta_inicializada(0x100).unwrap());
    assert_eq!(m.ultima_escritura(0x100).unwrap(), Some(Escritura::Carga));
}

#[test]
fn escribir_marca_la_celda_con_la_instruccion_actual() {
    let mut m = memoria();
    m.activar_seguimiento();
    m.fijar_instruccion_actual(0x42);
    m.escribir(0x100, 9).unwrap();
    assert!(m.esta_inicializada(0x100).unwrap());
    assert_eq!(
        m.ultima_escritura(0x100).unwrap(),
        Some(Escritura::Instruccion { pc: 0x42 })
    );
}

#[test]
fn escribir_un_cero_tambien_inicializa() {
    let mut m = memoria();
    m.activar_seguimiento();
    m.escribir(0x100, 0).unwrap();
    assert!(m.esta_inicializada(0x100).unwrap());
}

#[test]
fn la_ultima_escritura_gana() {
    let mut m = memoria();
    m.activar_seguimiento();
    m.fijar_instruccion_actual(1);
    m.escribir(0x100, 1).unwrap();
    m.fijar_instruccion_actual(2);
    m.escribir(0x100, 2).unwrap();
    assert_eq!(
        m.ultima_escritura(0x100).unwrap(),
        Some(Escritura::Instruccion { pc: 2 })
    );
}

#[test]
fn una_escritura_privilegiada_cuenta_como_carga() {
    let mut m = memoria();
    m.activar_seguimiento();
    m.fijar_instruccion_actual(5);
    m.escribir_privilegiado(0x100, 3).unwrap();
    assert_eq!(m.ultima_escritura(0x100).unwrap(), Some(Escritura::Carga));
}

#[test]
fn cargar_un_bloque_marca_todas_sus_celdas() {
    let mut m = memoria();
    m.activar_seguimiento();
    m.cargar_bloque(0x100, &[1, 2, 3]).unwrap();
    for d in 0x100..0x103 {
        assert_eq!(m.ultima_escritura(d).unwrap(), Some(Escritura::Carga));
    }
    assert_eq!(m.ultima_escritura(0x103).unwrap(), None);
}

#[test]
fn una_escritura_rechazada_no_marca_nada() {
    let mut m = memoria();
    m.activar_seguimiento();
    assert!(m.escribir(0x00, 1).is_err());
    assert_eq!(m.ultima_escritura(0x00).unwrap(), None);
}

#[test]
fn preguntar_por_una_direccion_fuera_de_las_regiones_da_error() {
    let m = memoria();
    assert_eq!(
        m.ultima_escritura(0x9000),
        Err(ErrorEjecucion::FueraDeRegion { direccion: 0x9000 })
    );
    assert_eq!(
        m.esta_inicializada(0x9000),
        Err(ErrorEjecucion::FueraDeRegion { direccion: 0x9000 })
    );
}

#[test]
fn leer_vigilada_anota_una_celda_sin_escribir() {
    let mut m = memoria();
    m.activar_seguimiento();
    m.fijar_instruccion_actual(0x20);
    assert_eq!(m.leer_vigilada(0x100).unwrap(), 0);
    assert_eq!(
        m.lecturas_sin_inicializar(),
        [LecturaSinInicializar {
            direccion: 0x100,
            pc: 0x20
        }]
    );
}

#[test]
fn leer_vigilada_no_anota_una_celda_escrita() {
    let mut m = memoria();
    m.activar_seguimiento();
    m.escribir(0x100, 0).unwrap();
    m.leer_vigilada(0x100).unwrap();
    assert!(m.lecturas_sin_inicializar().is_empty());
}

#[test]
fn leer_vigilada_sin_seguimiento_no_anota_nada() {
    let mut m = memoria();
    m.leer_vigilada(0x100).unwrap();
    assert!(m.lecturas_sin_inicializar().is_empty());
}

#[test]
fn leer_vigilada_respeta_los_permisos() {
    let mut m = memoria();
    m.activar_seguimiento();
    assert!(matches!(
        m.leer_vigilada(0x300),
        Err(ErrorEjecucion::LecturaDeSoloEscritura { .. })
    ));
    assert!(m.lecturas_sin_inicializar().is_empty());
}

#[test]
fn se_pueden_olvidar_las_lecturas_anotadas() {
    let mut m = memoria();
    m.activar_seguimiento();
    m.leer_vigilada(0x100).unwrap();
    m.olvidar_lecturas_sin_inicializar();
    assert!(m.lecturas_sin_inicializar().is_empty());
}

#[test]
fn activar_dos_veces_no_borra_lo_ya_anotado() {
    let mut m = memoria();
    m.activar_seguimiento();
    m.fijar_instruccion_actual(3);
    m.escribir(0x100, 1).unwrap();
    m.activar_seguimiento();
    assert_eq!(
        m.ultima_escritura(0x100).unwrap(),
        Some(Escritura::Instruccion { pc: 3 })
    );
}

#[test]
fn las_lecturas_anotadas_tienen_un_tope() {
    let mut m = memoria();
    m.activar_seguimiento();
    for _ in 0..(lorenz_emu::MAX_LECTURAS_SIN_INICIALIZAR + 10) {
        m.leer_vigilada(0x100).unwrap();
    }
    assert_eq!(
        m.lecturas_sin_inicializar().len(),
        lorenz_emu::MAX_LECTURAS_SIN_INICIALIZAR
    );
}

// --- Con programas reales ---

fn emulador_con_seguimiento(programa: &[u64]) -> Emulador {
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    e.memoria_mut().activar_seguimiento();
    e.cargar_programa(0, programa).unwrap();
    e
}

#[test]
fn un_st_deja_su_direccion_como_ultima_escritura() {
    // LDI A,5 ; ST A,[0x2000] ; HLT
    let mut e = emulador_con_seguimiento(&[0x20, 5, 0x50, 0x00, 0x20, 0xF0]);
    e.ejecutar(10).unwrap();
    assert_eq!(
        e.memoria().ultima_escritura(0x2000).unwrap(),
        Some(Escritura::Instruccion { pc: 2 })
    );
}

#[test]
fn el_programa_cargado_figura_como_carga() {
    let e = emulador_con_seguimiento(&[0x20, 5, 0xF0]);
    assert_eq!(
        e.memoria().ultima_escritura(1).unwrap(),
        Some(Escritura::Carga)
    );
}

#[test]
fn un_ld_de_memoria_sin_escribir_se_anota_con_la_direccion_del_ld() {
    // LD A,[0x2005] ; HLT
    let mut e = emulador_con_seguimiento(&[0x40, 0x05, 0x20, 0xF0]);
    e.ejecutar(10).unwrap();
    assert_eq!(
        e.memoria().lecturas_sin_inicializar(),
        [LecturaSinInicializar {
            direccion: 0x2005,
            pc: 0
        }]
    );
}

#[test]
fn un_ld_de_memoria_escrita_antes_no_se_anota() {
    // LDI A,1 ; ST A,[0x2000] ; LD B,[0x2000] ; HLT
    let mut e = emulador_con_seguimiento(&[0x20, 1, 0x50, 0x00, 0x20, 0x44, 0x00, 0x20, 0xF0]);
    e.ejecutar(10).unwrap();
    assert!(e.memoria().lecturas_sin_inicializar().is_empty());
}

#[test]
fn la_instantanea_conserva_el_seguimiento() {
    let mut e = emulador_con_seguimiento(&[0x20, 1, 0x50, 0x00, 0x20, 0xF0]);
    e.ejecutar(10).unwrap();
    let foto = e.instantanea();
    let mut otro = Emulador::nuevo(ejemplo_8()).unwrap();
    otro.restaurar(&foto).unwrap();
    assert_eq!(
        otro.memoria().ultima_escritura(0x2000).unwrap(),
        Some(Escritura::Instruccion { pc: 2 })
    );
}
