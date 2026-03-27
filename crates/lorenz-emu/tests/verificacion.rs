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

#[test]
fn el_orden_de_las_escrituras_no_cambia_la_suma() {
    let mut a = memoria();
    let mut b = memoria();
    a.escribir(0x100, 1).unwrap();
    a.escribir(0x101, 2).unwrap();
    a.escribir(0x100, 3).unwrap();
    b.escribir(0x101, 2).unwrap();
    b.escribir(0x100, 3).unwrap();
    assert_eq!(a.suma_de_verificacion(), b.suma_de_verificacion());
}

#[test]
fn las_escrituras_privilegiadas_y_por_bloque_tambien_cuentan() {
    let mut a = memoria();
    let mut b = memoria();
    a.cargar_bloque(0x100, &[5, 6, 7]).unwrap();
    b.escribir_privilegiado(0x100, 5).unwrap();
    b.escribir_privilegiado(0x101, 6).unwrap();
    b.escribir_privilegiado(0x102, 7).unwrap();
    assert_eq!(a.suma_de_verificacion(), b.suma_de_verificacion());
    assert_ne!(a.suma_de_verificacion(), memoria().suma_de_verificacion());
}

#[test]
fn escribir_un_valor_truncado_cuenta_con_el_valor_guardado() {
    let mut a = memoria();
    let mut b = memoria();
    a.escribir(0x100, 0x1FF).unwrap();
    b.escribir(0x100, 0xFF).unwrap();
    assert_eq!(a.suma_de_verificacion(), b.suma_de_verificacion());
}

#[test]
fn una_copia_conserva_la_suma_y_despues_cada_una_sigue_su_camino() {
    let mut a = memoria();
    a.escribir(0x100, 9).unwrap();
    let mut b = a.clone();
    assert_eq!(a.suma_de_verificacion(), b.suma_de_verificacion());
    b.escribir(0x100, 0).unwrap();
    assert_ne!(a.suma_de_verificacion(), b.suma_de_verificacion());
    assert_eq!(b.suma_de_verificacion(), memoria().suma_de_verificacion());
}
