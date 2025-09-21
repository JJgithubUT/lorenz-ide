//! Mapa, listado y tabla de simbolos (RF-ENS-017, RF-ENS-018, RF-ENS-029).

mod comun;
use comun::*;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};
use lorenz_ens::salidas::{leer_etiquetas, listado, mapa, tabla_de_simbolos};

const PROGRAMA: &str = "; suma\nK = 5\ninicio: LDI A, K\n\n  ADD A, B ; total\nfin: HLT\n";

#[test]
fn el_mapa_relaciona_cada_tramo_con_su_linea() {
    let m = ejemplo_8();
    let o = ensamblar(&m, PROGRAMA).unwrap();
    let t = mapa(&o);
    assert!(t.contains("0x0000         2  prueba.s:3"), "{t}");
    assert!(t.contains("0x0002         1  prueba.s:5"), "{t}");
    assert!(t.contains("0x0003         1  prueba.s:6"), "{t}");
}

#[test]
fn el_mapa_de_datos_y_espacio_cubre_varias_palabras() {
    let m = ejemplo_8();
    let o = ensamblar(&m, ".db 1, 2, 3\n.espacio 4").unwrap();
    let t = mapa(&o);
    assert!(t.contains("3  prueba.s:1"));
    assert!(t.contains("4  prueba.s:2"));
}

#[test]
fn el_listado_conserva_todo_el_fuente() {
    let m = ejemplo_8();
    let f = Fuente::nueva("prueba.s", PROGRAMA);
    let o = Ensamblador::nuevo(&m).unwrap().ensamblar(&f).unwrap();
    let l = listado(&o, &[f], m.bits_palabra);
    assert_eq!(l.lines().count(), PROGRAMA.lines().count());
    assert!(l.contains("; suma"));
    assert!(l.contains("inicio: LDI A, K"));
}

#[test]
fn el_listado_pone_direccion_y_bytes_en_las_lineas_que_emiten() {
    let m = ejemplo_8();
    let f = Fuente::nueva("prueba.s", PROGRAMA);
    let o = Ensamblador::nuevo(&m).unwrap().ensamblar(&f).unwrap();
    let l = listado(&o, &[f], m.bits_palabra);
    let linea = l.lines().find(|x| x.contains("LDI A, K")).unwrap();
    assert!(linea.contains("0000  20 05"), "{linea}");
    let linea = l.lines().find(|x| x.contains("HLT")).unwrap();
    assert!(linea.contains("0003  F0"), "{linea}");
    // Una linea sin codigo no lleva bytes.
    let comentario = l.lines().find(|x| x.contains("; suma")).unwrap();
    assert!(!comentario.contains("20"));
}

#[test]
fn el_listado_parte_las_filas_largas() {
    let m = ejemplo_8();
    let f = Fuente::nueva("prueba.s", ".db 1, 2, 3, 4, 5, 6, 7, 8\n");
    let o = Ensamblador::nuevo(&m).unwrap().ensamblar(&f).unwrap();
    let l = listado(&o, &[f], m.bits_palabra);
    assert_eq!(l.lines().count(), 2);
    assert!(l.lines().nth(1).unwrap().contains("0006  07 08"));
}

#[test]
fn la_tabla_de_simbolos_separa_etiquetas_y_constantes() {
    let m = ejemplo_8();
    let o = ensamblar(&m, PROGRAMA).unwrap();
    let t = tabla_de_simbolos(&o);
    assert!(
        t.contains("[etiquetas]\nfin = 0x0003\ninicio = 0x0000\n"),
        "{t}"
    );
    assert!(t.contains("[constantes]\nK = 5\n"), "{t}");
}

#[test]
fn la_tabla_es_toml_valido_y_se_lee_de_vuelta() {
    let m = ejemplo_8();
    let o = ensamblar(&m, PROGRAMA).unwrap();
    let t = tabla_de_simbolos(&o);
    let etiquetas = leer_etiquetas(&t).unwrap();
    assert_eq!(etiquetas.get(&0), Some(&"inicio".to_string()));
    assert_eq!(etiquetas.get(&3), Some(&"fin".to_string()));
    assert_eq!(etiquetas.len(), 2);
}

#[test]
fn una_tabla_mal_formada_se_rechaza() {
    assert!(leer_etiquetas("esto no es toml [").is_err());
    assert!(leer_etiquetas("[etiquetas]\nx = \"texto\"\n").is_err());
    assert!(leer_etiquetas("[etiquetas]\nx = -1\n").is_err());
    assert!(leer_etiquetas("").unwrap().is_empty());
}

#[test]
fn dos_etiquetas_en_la_misma_direccion_dejan_la_primera_alfabetica() {
    let e = leer_etiquetas("[etiquetas]\nb = 4\na = 4\n").unwrap();
    assert_eq!(e.get(&4), Some(&"a".to_string()));
}
