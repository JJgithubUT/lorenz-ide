//! Dispositivo de almacenamiento por bloques (RF-EMU-034).

mod comun;

use comun::con_disco;
use lorenz_emu::{Emulador, ErrorEjecucion};

const BLOQUE: u64 = 0x20;
const DATOS: u64 = 0x21;
const ESTADO: u64 = 0x22;

fn emulador() -> Emulador {
    Emulador::nuevo(con_disco()).unwrap()
}

#[test]
fn un_disco_nuevo_esta_lleno_de_ceros() {
    let mut e = emulador();
    assert_eq!(e.leer_bloque("disco", 2).unwrap(), [0, 0, 0, 0]);
}

#[test]
fn el_estado_inicial_es_listo_sin_error() {
    let mut e = emulador();
    assert_eq!(e.puertos_mut().leer(ESTADO).unwrap(), 1);
}

#[test]
fn se_escribe_un_bloque_por_los_puertos() {
    let mut e = emulador();
    let p = e.puertos_mut();
    p.escribir(BLOQUE, 1).unwrap();
    for v in [10, 20, 30, 40] {
        p.escribir(DATOS, v).unwrap();
    }
    assert_eq!(e.leer_bloque("disco", 1).unwrap(), [10, 20, 30, 40]);
    assert_eq!(e.leer_bloque("disco", 0).unwrap(), [0, 0, 0, 0]);
}

#[test]
fn se_lee_un_bloque_cargado_desde_fuera() {
    let mut e = emulador();
    e.cargar_bloque("disco", 3, &[7, 8, 9, 10]).unwrap();
    let p = e.puertos_mut();
    p.escribir(BLOQUE, 3).unwrap();
    let leidos: Vec<u64> = (0..4).map(|_| p.leer(DATOS).unwrap()).collect();
    assert_eq!(leidos, [7, 8, 9, 10]);
}

#[test]
fn seleccionar_un_bloque_reinicia_el_cursor() {
    let mut e = emulador();
    e.cargar_bloque("disco", 0, &[1, 2, 3, 4]).unwrap();
    let p = e.puertos_mut();
    p.leer(DATOS).unwrap();
    p.leer(DATOS).unwrap();
    p.escribir(BLOQUE, 0).unwrap();
    assert_eq!(p.leer(DATOS).unwrap(), 1);
}

#[test]
fn el_puerto_bloque_devuelve_el_bloque_seleccionado() {
    let mut e = emulador();
    e.puertos_mut().escribir(BLOQUE, 2).unwrap();
    assert_eq!(e.puertos_mut().leer(BLOQUE).unwrap(), 2);
}

#[test]
fn seleccionar_un_bloque_inexistente_marca_error_y_no_escribe() {
    let mut e = emulador();
    let p = e.puertos_mut();
    p.escribir(BLOQUE, 4).unwrap();
    assert_eq!(p.leer(ESTADO).unwrap(), 3);
    p.escribir(DATOS, 99).unwrap();
    for b in 0..4 {
        assert_eq!(e.leer_bloque("disco", b).unwrap(), [0, 0, 0, 0]);
    }
}

#[test]
fn pasarse_del_final_del_bloque_marca_error() {
    let mut e = emulador();
    let p = e.puertos_mut();
    for _ in 0..4 {
        p.leer(DATOS).unwrap();
    }
    assert_eq!(p.leer(ESTADO).unwrap(), 1);
    assert_eq!(p.leer(DATOS).unwrap(), 0);
    assert_eq!(p.leer(ESTADO).unwrap(), 3);
}

#[test]
fn escribir_pasado_el_final_no_toca_el_bloque_siguiente() {
    let mut e = emulador();
    let p = e.puertos_mut();
    for v in 1..=5 {
        p.escribir(DATOS, v).unwrap();
    }
    assert_eq!(e.leer_bloque("disco", 0).unwrap(), [1, 2, 3, 4]);
    assert_eq!(e.leer_bloque("disco", 1).unwrap(), [0, 0, 0, 0]);
}

#[test]
fn un_error_se_borra_al_elegir_un_bloque_valido() {
    let mut e = emulador();
    let p = e.puertos_mut();
    p.escribir(BLOQUE, 9).unwrap();
    p.escribir(BLOQUE, 1).unwrap();
    assert_eq!(p.leer(ESTADO).unwrap(), 1);
}

#[test]
fn escribir_en_el_puerto_de_estado_no_hace_nada() {
    let mut e = emulador();
    e.puertos_mut().escribir(ESTADO, 0xFF).unwrap();
    assert_eq!(e.puertos_mut().leer(ESTADO).unwrap(), 1);
}

#[test]
fn los_valores_se_truncan_al_ancho_de_palabra() {
    let mut e = emulador();
    e.cargar_bloque("disco", 0, &[0x1FF]).unwrap();
    assert_eq!(e.leer_bloque("disco", 0).unwrap(), [0xFF, 0, 0, 0]);
    e.puertos_mut().escribir(DATOS, 0x234).unwrap();
    assert_eq!(e.leer_bloque("disco", 0).unwrap()[0], 0x34);
}

#[test]
fn cargar_menos_palabras_rellena_con_ceros() {
    let mut e = emulador();
    e.cargar_bloque("disco", 1, &[5, 6, 7, 8]).unwrap();
    e.cargar_bloque("disco", 1, &[1]).unwrap();
    assert_eq!(e.leer_bloque("disco", 1).unwrap(), [1, 0, 0, 0]);
}

#[test]
fn cargar_mas_palabras_que_un_bloque_se_rechaza() {
    let mut e = emulador();
    assert!(matches!(
        e.cargar_bloque("disco", 0, &[1, 2, 3, 4, 5]),
        Err(ErrorEjecucion::BloqueFueraDeRango { .. })
    ));
}

#[test]
fn un_bloque_fuera_del_disco_se_rechaza() {
    let mut e = emulador();
    assert!(matches!(
        e.leer_bloque("disco", 4),
        Err(ErrorEjecucion::BloqueFueraDeRango { bloque: 4, .. })
    ));
    assert!(matches!(
        e.cargar_bloque("disco", 4, &[]),
        Err(ErrorEjecucion::BloqueFueraDeRango { .. })
    ));
}

#[test]
fn un_dispositivo_inexistente_se_rechaza() {
    let mut e = emulador();
    assert!(matches!(
        e.leer_bloque("nube", 0),
        Err(ErrorEjecucion::DispositivoInexistente { .. })
    ));
}

#[test]
fn reiniciar_conserva_el_contenido_pero_no_la_posicion() {
    let mut e = emulador();
    e.cargar_bloque("disco", 2, &[4, 3, 2, 1]).unwrap();
    e.puertos_mut().escribir(BLOQUE, 2).unwrap();
    e.puertos_mut().leer(DATOS).unwrap();
    e.reiniciar();
    assert_eq!(e.leer_bloque("disco", 2).unwrap(), [4, 3, 2, 1]);
    assert_eq!(e.puertos_mut().leer(BLOQUE).unwrap(), 0);
}

#[test]
fn la_instantanea_conserva_el_disco() {
    let mut e = emulador();
    e.cargar_bloque("disco", 1, &[9, 9, 9, 9]).unwrap();
    let foto = e.instantanea();
    e.cargar_bloque("disco", 1, &[0]).unwrap();
    e.restaurar(&foto).unwrap();
    assert_eq!(e.leer_bloque("disco", 1).unwrap(), [9, 9, 9, 9]);
}
