//! El archivo objeto (RF-ENS-016).

mod comun;
use comun::*;
use lorenz_ens::ensamblador::Objeto;
use lorenz_ens::objeto::FORMATO_OBJETO;

const PROGRAMA: &str = "K = 5\ninicio: LDI A, K\nADD A, B\n.org 0x2000\ndatos: .db 1, 2, 3\n";

#[test]
fn el_objeto_lleva_codigo_simbolos_y_ubicacion() {
    let m = ejemplo_8();
    let o = ensamblar(&m, PROGRAMA).unwrap();
    let t = o.a_toml(&m.nombre);
    assert!(t.contains(&format!("formato = \"{FORMATO_OBJETO}\"")));
    assert!(t.contains("maquina = \"ejemplo-8\""));
    assert!(t.contains("[[segmento]]"));
    assert!(t.contains("[[simbolo]]"));
    assert!(t.contains("[[ubicacion]]"));
}

#[test]
fn guardar_y_leer_devuelve_el_mismo_objeto() {
    let m = ejemplo_8();
    let o = ensamblar(&m, PROGRAMA).unwrap();
    let (nombre, leido) = Objeto::desde_toml(&o.a_toml(&m.nombre)).unwrap();
    assert_eq!(nombre, "ejemplo-8");
    assert_eq!(leido, o);
}

#[test]
fn un_objeto_vacio_tambien_se_guarda() {
    let m = ejemplo_8();
    let o = ensamblar(&m, "").unwrap();
    let (_, leido) = Objeto::desde_toml(&o.a_toml(&m.nombre)).unwrap();
    assert_eq!(leido, o);
}

#[test]
fn un_formato_desconocido_se_rechaza_con_el_que_se_esperaba() {
    let e = Objeto::desde_toml("formato = \"otro\"\nmaquina = \"x\"\n").unwrap_err();
    assert!(e.contains("otro") && e.contains(FORMATO_OBJETO));
}

#[test]
fn texto_que_no_es_un_objeto_se_rechaza() {
    assert!(Objeto::desde_toml("esto no es toml [").is_err());
    assert!(Objeto::desde_toml("x = 1").is_err());
}

#[test]
fn las_palabras_de_un_segmento_se_conservan_en_orden() {
    let m = ejemplo_8();
    let o = ensamblar(&m, ".db 9, 8, 7, 255, 0").unwrap();
    let (_, leido) = Objeto::desde_toml(&o.a_toml(&m.nombre)).unwrap();
    assert_eq!(leido.segmentos[0].palabras, vec![9, 8, 7, 255, 0]);
}
