//! Pruebas de los puertos de entrada y salida.

mod comun;

use comun::ejemplo_8;
use lorenz_emu::{ErrorEjecucion, Puertos};

fn puertos() -> Puertos {
    Puertos::nuevo(&ejemplo_8())
}

#[test]
fn solo_existen_los_puertos_declarados() {
    assert_eq!(puertos().numeros(), [1, 2]);
}

#[test]
fn un_puerto_no_declarado_da_error_al_leer_y_escribir() {
    let mut p = puertos();
    assert!(matches!(p.leer(9), Err(ErrorEjecucion::PuertoNoDeclarado { puerto: 9 })));
    assert!(matches!(p.escribir(9, 1), Err(ErrorEjecucion::PuertoNoDeclarado { puerto: 9 })));
    assert!(matches!(p.encolar_entrada(9, 1), Err(ErrorEjecucion::PuertoNoDeclarado { .. })));
}

#[test]
fn leer_sin_datos_devuelve_cero() {
    assert_eq!(puertos().leer(1).unwrap(), 0);
}

#[test]
fn la_entrada_sale_en_el_orden_en_que_entro() {
    let mut p = puertos();
    for v in [3, 1, 2] {
        p.encolar_entrada(1, v).unwrap();
    }
    assert_eq!(p.pendientes(1).unwrap(), 3);
    assert_eq!([p.leer(1).unwrap(), p.leer(1).unwrap(), p.leer(1).unwrap()], [3, 1, 2]);
    assert_eq!(p.pendientes(1).unwrap(), 0);
}

#[test]
fn cada_puerto_tiene_su_propia_cola() {
    let mut p = puertos();
    p.encolar_entrada(1, 7).unwrap();
    assert_eq!(p.leer(2).unwrap(), 0);
    assert_eq!(p.leer(1).unwrap(), 7);
}

#[test]
fn la_entrada_se_trunca_al_ancho_de_palabra() {
    let mut p = puertos();
    p.encolar_entrada(1, 0x1AB).unwrap();
    assert_eq!(p.leer(1).unwrap(), 0xAB);
}

#[test]
fn la_salida_se_registra_en_orden_y_truncada() {
    let mut p = puertos();
    p.escribir(2, 5).unwrap();
    p.escribir(2, 0x100).unwrap();
    assert_eq!(p.salida(2).unwrap(), [5, 0]);
    assert!(p.salida(1).unwrap().is_empty());
}

#[test]
fn reiniciar_vacia_entradas_y_salidas() {
    let mut p = puertos();
    p.encolar_entrada(1, 1).unwrap();
    p.escribir(2, 1).unwrap();
    p.reiniciar();
    assert_eq!(p.pendientes(1).unwrap(), 0);
    assert!(p.salida(2).unwrap().is_empty());
}
