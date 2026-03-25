//! Dependencias de datos de las instrucciones (RF-CIC-005).

mod comun;
use comun::*;

use lorenz_cic::dependencias;
use lorenz_emu::{Decodificador, Memoria};

fn deps(programa: &[u8]) -> lorenz_cic::Dependencias {
    let m = lorenz_arch::cargar(&texto_maquina()).unwrap();
    let mut mem = Memoria::nueva(&m).unwrap();
    let palabras: Vec<u64> = programa.iter().map(|b| u64::from(*b)).collect();
    mem.cargar_bloque(0, &palabras).unwrap();
    let d = Decodificador::nuevo(&m).decodificar(&mem, 0).unwrap();
    dependencias(&m, &m.instrucciones[d.indice], &d.operandos)
}

#[test]
fn una_suma_lee_sus_dos_registros_y_escribe_uno_y_las_banderas() {
    let d = deps(&[ADD_AB]);
    assert_eq!(d.lee, vec!["A", "B"]);
    assert_eq!(d.escribe, vec!["A", "bandera C", "bandera Z"]);
    assert!(!d.es_salto);
}

#[test]
fn una_bandera_no_se_confunde_con_un_registro_del_mismo_nombre() {
    // ADD C,A escribe el registro C y la bandera C: son cosas distintas.
    let d = deps(&[ADD_CA]);
    assert!(d.escribe.contains(&"C".to_string()));
    assert!(d.escribe.contains(&"bandera C".to_string()));
}

#[test]
fn una_carga_inmediata_no_lee_nada() {
    let d = deps(&[LDI_A, 5]);
    assert!(d.lee.is_empty());
    assert_eq!(d.escribe, vec!["A"]);
}

#[test]
fn un_salto_condicional_lee_la_bandera_y_es_directo() {
    let d = deps(&[0x60, 0x10, 0x00]);
    assert_eq!(d.lee, vec!["bandera Z"]);
    assert!(d.es_salto && !d.incondicional && !d.indirecto);
}
