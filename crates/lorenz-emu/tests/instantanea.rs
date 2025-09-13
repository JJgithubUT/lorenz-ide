//! Pruebas de las instantaneas: guardar y restaurar el estado completo.

mod comun;

use comun::ejemplo_8;
use lorenz_emu::{Emulador, ErrorEjecucion};

fn con_programa(palabras: &[u64]) -> Emulador {
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    e.cargar_programa(0, palabras).unwrap();
    e
}

#[test]
fn restaurar_devuelve_los_registros_y_el_pc() {
    let mut e = con_programa(&[0x20, 5, 0x24, 7, 0xF0]);
    e.paso().unwrap();
    let foto = e.instantanea();
    e.ejecutar(10).unwrap();
    assert_eq!(e.estado().registro("B").unwrap(), 7);
    e.restaurar(&foto).unwrap();
    assert_eq!(e.estado().registro("A").unwrap(), 5);
    assert_eq!(e.estado().registro("B").unwrap(), 0);
    assert_eq!(e.estado().pc(), 2);
    assert!(!e.estado().detenida());
}

#[test]
fn restaurar_devuelve_la_memoria() {
    let mut e = con_programa(&[0x20, 9, 0x50, 0x00, 0x20, 0xF0]);
    let foto = e.instantanea();
    e.ejecutar(10).unwrap();
    assert_eq!(e.memoria().leer(0x2000).unwrap(), 9);
    e.restaurar(&foto).unwrap();
    assert_eq!(e.memoria().leer(0x2000).unwrap(), 0);
}

#[test]
fn restaurar_devuelve_contadores_y_banderas() {
    let mut e = con_programa(&[0x20, 200, 0x24, 100, 0x31, 0xF0]);
    e.avanzar(3).unwrap();
    let foto = e.instantanea();
    let (instr, ciclos) = (e.estado().instrucciones(), e.estado().ciclos());
    e.ejecutar(10).unwrap();
    e.restaurar(&foto).unwrap();
    assert_eq!(
        (e.estado().instrucciones(), e.estado().ciclos()),
        (instr, ciclos)
    );
    assert!(e.estado().bandera("C").unwrap());
}

#[test]
fn ejecutar_desde_una_instantanea_da_el_mismo_resultado_que_seguir() {
    let programa = [0x20, 200, 0x24, 100, 0x31, 0x50, 0x00, 0x20, 0xF0];
    let mut seguido = con_programa(&programa);
    seguido.avanzar(2).unwrap();
    let foto = seguido.instantanea();
    seguido.ejecutar(20).unwrap();

    let mut repetido = con_programa(&programa);
    repetido.restaurar(&foto).unwrap();
    repetido.ejecutar(20).unwrap();

    assert_eq!(seguido.estado().ciclos(), repetido.estado().ciclos());
    assert_eq!(
        seguido.estado().registro("A").unwrap(),
        repetido.estado().registro("A").unwrap()
    );
    assert_eq!(
        seguido.memoria().leer(0x2000).unwrap(),
        repetido.memoria().leer(0x2000).unwrap()
    );
}

#[test]
fn la_instantanea_no_cambia_aunque_la_maquina_siga() {
    let mut e = con_programa(&[0x20, 5, 0xF0]);
    let foto = e.instantanea();
    e.ejecutar(10).unwrap();
    assert_eq!(foto.estado().registro("A").unwrap(), 0);
    assert_eq!(foto.estado().pc(), 0);
}

#[test]
fn la_instantanea_expone_la_memoria_del_instante() {
    let mut e = con_programa(&[0xF0]);
    e.memoria_mut().escribir(0x2000, 0x55).unwrap();
    let foto = e.instantanea();
    e.memoria_mut().escribir(0x2000, 0x66).unwrap();
    assert_eq!(foto.memoria().leer(0x2000).unwrap(), 0x55);
}

#[test]
fn restaurar_devuelve_los_puertos_y_las_interrupciones_pendientes() {
    let mut e = con_programa(&[0xF0]);
    e.pulsar_tecla("teclado", 0x41).unwrap();
    let foto = e.instantanea();
    e.puertos_mut().leer(1).unwrap();
    e.reiniciar();
    assert!(e.interrupciones_pendientes().is_empty());
    e.restaurar(&foto).unwrap();
    assert_eq!(e.interrupciones_pendientes(), [1]);
    assert_eq!(e.puertos_mut().leer(1).unwrap(), 0x41);
}

#[test]
fn restaurar_devuelve_el_registro_de_interrupciones_atendidas() {
    let mut e = con_programa(&[0x00, 0xF0]);
    e.cargar_programa(0x0100, &[0xF0]).unwrap();
    e.cargar_programa(0xEFF0, &[0x00, 0x01]).unwrap();
    e.estado_mut().habilitar_interrupciones(true);
    e.levantar_interrupcion(0).unwrap();
    e.paso().unwrap();
    let foto = e.instantanea();
    e.reiniciar();
    assert!(e.interrupciones_atendidas().is_empty());
    e.restaurar(&foto).unwrap();
    assert_eq!(e.interrupciones_atendidas().len(), 1);
}

#[test]
fn restaurar_sobre_otra_maquina_es_un_error_y_no_cambia_nada() {
    let mut a = con_programa(&[0xF0]);
    let foto = a.instantanea();
    let mut m = ejemplo_8();
    m.nombre = "otra".into();
    let mut b = Emulador::nuevo(m).unwrap();
    b.estado_mut().fijar_pc(0x40);
    assert_eq!(
        b.restaurar(&foto),
        Err(ErrorEjecucion::InstantaneaIncompatible {
            instantanea: "ejemplo-8".into(),
            maquina: "otra".into()
        })
    );
    assert_eq!(b.estado().pc(), 0x40);
    a.ejecutar(1).unwrap();
}

#[test]
fn se_puede_restaurar_la_misma_instantanea_varias_veces() {
    let mut e = con_programa(&[0x20, 5, 0xF0]);
    let foto = e.instantanea();
    for _ in 0..3 {
        e.ejecutar(10).unwrap();
        assert_eq!(e.estado().registro("A").unwrap(), 5);
        e.restaurar(&foto).unwrap();
        assert_eq!(e.estado().registro("A").unwrap(), 0);
    }
}
