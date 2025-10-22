//! Simbolos externos y reubicaciones (RF-ENS-019).

mod comun;
use comun::*;

#[test]
fn una_instruccion_con_un_externo_deja_un_hueco_y_una_reubicacion() {
    let m = ejemplo_8();
    let o = ensamblar(&m, ".externo K\nNOP\nLDI A, K\nNOP").unwrap();
    assert!(o.externos.contains_key("K"));
    assert_eq!(o.reubicaciones.len(), 1);
    let r = &o.reubicaciones[0];
    assert_eq!(r.fuente, "LDI A, K");
    assert_eq!(r.simbolos, vec!["K".to_string()]);
    assert_eq!(r.linea, 3);
    // El hueco ocupa lo que ocupara la instruccion y el resto del programa no se mueve.
    let sin = bytes(&m, "NOP\nLDI A, 7\nNOP");
    assert_eq!(o.tamano(), sin.len());
    assert_eq!(r.direccion, o.mapa[0].direccion + 1);
}

#[test]
fn un_externo_sin_usar_no_deja_reubicaciones() {
    let m = ejemplo_8();
    let o = ensamblar(&m, ".externo K\nNOP").unwrap();
    assert!(o.reubicaciones.is_empty());
    assert_eq!(o.externos.len(), 1);
}

#[test]
fn varios_externos_en_una_directiva_y_en_una_expresion() {
    let m = ejemplo_8();
    let o = ensamblar(&m, ".externo A1, B1\nLDI A, A1 + B1").unwrap();
    assert_eq!(o.externos.len(), 2);
    assert_eq!(o.reubicaciones[0].simbolos, vec!["A1", "B1"]);
}

#[test]
fn definir_y_declarar_externo_el_mismo_simbolo_es_un_error() {
    let m = ejemplo_8();
    let e = errores(&m, ".externo K\nK = 3\nNOP");
    assert!(e[0].to_string().contains("externo"), "{}", e[0]);
}

#[test]
fn un_externo_en_un_dato_se_rechaza_con_el_motivo() {
    let m = ejemplo_8();
    let e = errores(&m, ".externo K\n.db K");
    assert!(e[0].to_string().contains("instruccion"), "{}", e[0]);
}

#[test]
fn externo_sin_argumentos_es_un_error() {
    let m = ejemplo_8();
    assert!(!errores(&m, ".externo").is_empty());
}

#[test]
fn un_simbolo_no_declarado_sigue_siendo_un_error() {
    let m = ejemplo_8();
    assert!(!errores(&m, "LDI A, K").is_empty());
}
