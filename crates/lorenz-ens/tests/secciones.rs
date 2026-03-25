//! Secciones con nombre ubicadas en regiones declaradas (RF-ENS-010).

mod comun;
use comun::*;

#[test]
fn una_seccion_empieza_al_principio_de_su_region() {
    let m = ejemplo_8();
    let o = ensamblar(&m, ".seccion ram\nNOP").unwrap();
    assert_eq!(o.segmentos[0].direccion, 0x2000);
}

#[test]
fn una_seccion_continua_donde_la_dejo() {
    let m = ejemplo_8();
    let fuente =
        ".seccion rom\nNOP\n.seccion ram\n.db 1, 2\n.seccion rom\nHLT\n.seccion ram\n.db 3";
    let o = ensamblar(&m, fuente).unwrap();
    let p = o.palabras();
    assert_eq!(p[0], (0x0000, 0x00));
    assert_eq!(p[1], (0x0001, 0xF0));
    assert_eq!(p[2], (0x2000, 1));
    assert_eq!(p[3], (0x2001, 2));
    assert_eq!(p[4], (0x2002, 3));
}

#[test]
fn las_etiquetas_de_cada_seccion_caen_en_su_region() {
    let m = ejemplo_8();
    let o = ensamblar(
        &m,
        ".seccion rom\ncodigo: HLT\n.seccion ram\ndatos: .db 7\n.seccion rom\nJZ datos",
    )
    .unwrap();
    assert_eq!(o.simbolo("codigo"), Some(0));
    assert_eq!(o.simbolo("datos"), Some(0x2000));
    let p = o.palabras();
    assert_eq!(p[2], (0x0002, 0x00)); // el JZ empieza tras HLT: 0x01..0x03
}

#[test]
fn una_seccion_inexistente_lista_las_regiones() {
    let m = ejemplo_8();
    let e = errores(&m, ".seccion nada");
    assert!(e[0].to_string().contains("rom, ram, video"));
}

#[test]
fn seccion_exige_un_nombre() {
    let m = ejemplo_8();
    assert!(ensamblar(&m, ".seccion").is_err());
    assert!(ensamblar(&m, ".seccion rom, ram").is_err());
}

#[test]
fn una_seccion_sobre_un_dispositivo_se_rechaza_al_emitir() {
    let m = ejemplo_8();
    let e = errores(&m, ".seccion video\nNOP");
    assert!(e[0].to_string().contains("video"));
}

#[test]
fn org_dentro_de_una_seccion_mueve_su_contador() {
    let m = ejemplo_8();
    let o = ensamblar(
        &m,
        ".seccion ram\n.org 0x2100\n.db 1\n.seccion rom\nNOP\n.seccion ram\n.db 2",
    )
    .unwrap();
    let p = o.palabras();
    assert!(p.contains(&(0x2100, 1)));
    assert!(p.contains(&(0x2101, 2)));
}
