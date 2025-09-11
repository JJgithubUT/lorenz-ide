//! Suma de verificacion de la memoria.

mod comun;

use comun::con_tres_regiones;
use lorenz_emu::Memoria;

fn memoria() -> Memoria {
    Memoria::nueva(&con_tres_regiones()).unwrap()
}

#[test]
fn dos_memorias_iguales_dan_la_misma_suma() {
    assert_eq!(
        memoria().suma_de_verificacion(),
        memoria().suma_de_verificacion()
    );
}

#[test]
fn cambiar_una_celda_cambia_la_suma() {
    let mut m = memoria();
    let antes = m.suma_de_verificacion();
    m.escribir(0x100, 1).unwrap();
    assert_ne!(m.suma_de_verificacion(), antes);
}

#[test]
fn volver_al_contenido_anterior_devuelve_la_suma() {
    let mut m = memoria();
    let antes = m.suma_de_verificacion();
    m.escribir(0x100, 1).unwrap();
    m.escribir(0x100, 0).unwrap();
    assert_eq!(m.suma_de_verificacion(), antes);
}

#[test]
fn el_mismo_valor_en_otra_celda_da_otra_suma() {
    let mut a = memoria();
    let mut b = memoria();
    a.escribir(0x100, 7).unwrap();
    b.escribir(0x101, 7).unwrap();
    assert_ne!(a.suma_de_verificacion(), b.suma_de_verificacion());
}
