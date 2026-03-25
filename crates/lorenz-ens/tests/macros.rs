//! Macros con parametros (RF-ENS-009).

mod comun;
use comun::*;
use lorenz_ens::error::Causa;

#[test]
fn una_macro_se_expande_en_cada_llamada() {
    let m = ejemplo_8();
    let fuente = ".macro CARGA r, v\n  LDI \\r, \\v\n.endm\nCARGA A, 5\nCARGA B, 6\n";
    assert_eq!(bytes(&m, fuente), vec![0x20, 5, 0x24, 6]);
}

#[test]
fn una_macro_puede_tener_varias_lineas() {
    let m = ejemplo_8();
    let fuente = ".macro DOBLE r\n  ADD \\r, \\r\n  ADD \\r, \\r\n.endm\nDOBLE C";
    assert_eq!(bytes(&m, fuente), vec![0x3A, 0x3A]);
}

#[test]
fn una_macro_sin_parametros() {
    let m = ejemplo_8();
    assert_eq!(
        bytes(&m, ".macro PARAR\n  NOP\n  HLT\n.endm\nPARAR"),
        vec![0, 0xF0]
    );
}

#[test]
fn los_argumentos_pueden_ser_expresiones() {
    let m = ejemplo_8();
    let fuente = ".macro CARGA r, v\n  LDI \\r, \\v\n.endm\nCARGA A, (2 + 3) * 4\n";
    assert_eq!(bytes(&m, fuente), vec![0x20, 20]);
}

#[test]
fn las_etiquetas_locales_de_cada_expansion_no_chocan() {
    let m = ejemplo_8();
    let fuente = ".macro SALTA\nlocal\\@: JZ local\\@\n.endm\nSALTA\nSALTA\n";
    let o = ensamblar(&m, fuente).unwrap();
    assert_eq!(o.tamano(), 6);
    assert!(o.simbolo("local1").is_some() && o.simbolo("local2").is_some());
}

#[test]
fn una_macro_puede_invocar_a_otra() {
    let m = ejemplo_8();
    let fuente =
        ".macro UNO r\n  LDI \\r, 1\n.endm\n.macro DOS a, b\n  UNO \\a\n  UNO \\b\n.endm\nDOS A, B";
    assert_eq!(bytes(&m, fuente), vec![0x20, 1, 0x24, 1]);
}

#[test]
fn una_macro_que_se_llama_a_si_misma_se_corta() {
    let m = ejemplo_8();
    let e = errores(&m, ".macro BUCLE\n  BUCLE\n.endm\nBUCLE");
    assert!(e[0].to_string().contains("se expande a si misma"));
}

#[test]
fn los_nombres_de_macro_no_distinguen_mayusculas() {
    let m = ejemplo_8();
    assert_eq!(
        bytes(&m, ".macro Parar\n HLT\n.endm\nPARAR\nparar"),
        vec![0xF0, 0xF0]
    );
}

#[test]
fn un_parametro_largo_no_se_rompe_por_uno_corto() {
    let m = ejemplo_8();
    let fuente = ".macro M a, ab\n  LDI \\a, \\ab\n.endm\nM A, 7";
    assert_eq!(bytes(&m, fuente), vec![0x20, 7]);
}

#[test]
fn argumentos_de_mas_o_de_menos_son_un_error() {
    let m = ejemplo_8();
    let def = ".macro M a, b\n  MOV \\a, \\b\n.endm\n";
    let e = errores(&m, &format!("{def}M A"));
    assert!(e[0]
        .to_string()
        .contains("recibe 2 argumento(s) y se le pasaron 1"));
    assert!(ensamblar(&m, &format!("{def}M A, B, C")).is_err());
}

#[test]
fn una_macro_no_puede_llamarse_como_una_instruccion() {
    let m = ejemplo_8();
    let e = errores(&m, ".macro NOP\n HLT\n.endm");
    assert!(e[0].to_string().contains("ya es una instruccion"));
}

#[test]
fn una_macro_duplicada_indica_donde_se_definio() {
    let m = ejemplo_8();
    let e = errores(&m, ".macro X\n NOP\n.endm\n.macro X\n NOP\n.endm");
    assert!(e[0].to_string().contains("ya se definio en prueba.s:1"));
}

#[test]
fn una_macro_sin_cerrar_es_un_error() {
    let m = ejemplo_8();
    let e = errores(&m, ".macro X\n NOP\n");
    assert!(e[0].to_string().contains("falta `.endm`"));
}

#[test]
fn un_endm_suelto_es_un_error() {
    let m = ejemplo_8();
    assert!(ensamblar(&m, ".endm").is_err());
}

#[test]
fn un_parametro_con_nombre_invalido_se_rechaza() {
    let m = ejemplo_8();
    assert!(ensamblar(&m, ".macro X 1a\n NOP\n.endm").is_err());
}

#[test]
fn una_macro_se_puede_usar_antes_de_que_algo_la_redefina_pero_no_antes_de_definirse() {
    let m = ejemplo_8();
    // Antes de definirla, `X` es solo un mnemonico desconocido.
    let e = errores(&m, "X\n.macro X\n NOP\n.endm");
    assert!(matches!(e[0].causa, Causa::MnemonicoDesconocido(_)));
}

#[test]
fn una_etiqueta_antes_de_la_llamada_se_conserva() {
    let m = ejemplo_8();
    let o = ensamblar(&m, ".macro P\n HLT\n.endm\nfin: P\nJZ fin").unwrap();
    assert_eq!(o.simbolo("fin"), Some(0));
}

#[test]
fn los_errores_dentro_de_una_macro_apuntan_a_la_linea_que_la_llama() {
    let m = ejemplo_8();
    let e = errores(&m, ".macro M\n FOO\n.endm\nNOP\nM");
    assert_eq!(e[0].linea, 5);
}
