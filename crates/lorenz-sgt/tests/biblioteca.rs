//! Biblioteca minima de memoria y de textos (RF-SGT-041).
mod comun;
use comun::*;

const MAQUINAS: [&str; 2] = ["risc-16", "sgt-16"];

fn dif(fuente: &str) {
    for m in MAQUINAS {
        igual_al_oraculo(m, fuente);
    }
}

fn valores(fuente: &str) -> Vec<std::collections::HashMap<String, i128>> {
    MAQUINAS.iter().map(|m| ejecutado(m, fuente).0).collect()
}

#[test]
fn un_texto_inicia_un_arreglo_y_termina_en_cero() {
    let fuente = "var t: [u16; 8] = \"hola\"; var a: u16; var b: u16; var c: u16; var d: u16;
        funcion principal() { a = t[0]; b = t[3]; c = t[4]; d = t[7]; }";
    dif(fuente);
    for g in valores(fuente) {
        assert_eq!((g["a"], g["b"], g["c"], g["d"]), (104, 97, 0, 0));
    }
}

#[test]
fn longitud_de_un_texto() {
    let fuente = "var t: [u16; 10] = \"lorenz\"; var n: u16; var v: u16;
        funcion principal() { n = cad_longitud(&t[0]); v = cad_longitud(&t[6]); }";
    dif(fuente);
    for g in valores(fuente) {
        assert_eq!((g["n"], g["v"]), (6, 0));
    }
}

#[test]
fn copiar_y_comparar_textos() {
    let fuente = "var a: [u16; 8] = \"abc\"; var b: [u16; 8]; var c: [u16; 8] = \"abd\";
        var largo: u16; var igual: i16; var menor: i16; var mayor: i16;
        funcion principal() {
          largo = cad_copiar(&b[0], &a[0]);
          igual = cad_comparar(&a[0], &b[0]);
          menor = cad_comparar(&a[0], &c[0]);
          mayor = cad_comparar(&c[0], &a[0]);
        }";
    dif(fuente);
    for g in valores(fuente) {
        assert_eq!(g["largo"], 3);
        assert_eq!(g["igual"], 0);
        assert_eq!(g["menor"], 0xFFFF);
        assert_eq!(g["mayor"], 1);
    }
}

#[test]
fn un_texto_mas_corto_es_menor_que_su_continuacion() {
    dif(
        "var a: [u16; 6] = \"ab\"; var b: [u16; 6] = \"abc\"; var r: i16; var s: i16;
        funcion principal() { r = cad_comparar(&a[0], &b[0]); s = cad_comparar(&b[0], &a[0]); }",
    );
}

#[test]
fn llenar_copiar_y_comparar_memoria() {
    let fuente = "var m: [u16; 8]; var n: [u16; 8]; var suma: u16; var igual: i16; var dif: i16;
        funcion principal() {
          mem_llenar(&m[0], 7, 8);
          mem_copiar(&n[0], &m[0], 8);
          igual = mem_comparar(&m[0], &n[0], 8);
          n[5] = 9;
          dif = mem_comparar(&m[0], &n[0], 8);
          var i: u16 = 0;
          mientras i < 8 { suma += n[i]; i += 1; }
        }";
    dif(fuente);
    for g in valores(fuente) {
        assert_eq!(g["suma"], 7 * 7 + 9);
        assert_eq!(g["igual"], 0);
        assert_eq!(g["dif"], 0xFFFF);
    }
}

#[test]
fn copiar_cero_elementos_no_toca_nada() {
    dif("var a: [u16; 2] = \"x\"; var b: [u16; 2]; var r: u16;
        funcion principal() { mem_copiar(&b[0], &a[0], 0); r = b[0]; }");
}

#[test]
fn lo_que_no_se_usa_no_entra_en_el_programa() {
    for m in MAQUINAS {
        let sin = compilar_en(m, "funcion principal() { }").unwrap();
        let con = compilar_en(
            m,
            "var t: [u16; 4] = \"ab\"; var n: u16; funcion principal() { n = cad_longitud(&t[0]); }",
        )
        .unwrap();
        assert!(sin
            .modulo
            .funciones
            .iter()
            .all(|f| f.base != "cad_longitud"));
        assert!(con
            .modulo
            .funciones
            .iter()
            .any(|f| f.base == "cad_longitud"));
        assert!(con.modulo.funciones.iter().all(|f| f.base != "mem_copiar"));
    }
}

#[test]
fn el_texto_no_cabe() {
    let e = compilar_en(
        "risc-16",
        "var t: [u16; 3] = \"hola\"; funcion principal() { }",
    )
    .unwrap_err();
    assert!(
        e.contains("4 caracteres") && e.contains("3 elementos"),
        "{e}"
    );
    assert!(e.contains("cero final"), "{e}");
}

#[test]
fn el_texto_justo_deja_lugar_al_cero() {
    let e = compilar_en(
        "risc-16",
        "var t: [u16; 4] = \"hola\"; funcion principal() { }",
    )
    .unwrap_err();
    assert!(e.contains("4 caracteres"), "{e}");
    compilar_en(
        "risc-16",
        "var t: [u16; 5] = \"hola\"; funcion principal() { }",
    )
    .unwrap();
}

#[test]
fn el_caracter_debe_caber_en_el_elemento() {
    // En una maquina de 16 bits, un elemento u16 admite cualquier caracter hasta U+FFFF.
    let e = compilar_en(
        "risc-16",
        "var t: [u16; 2] = \"\u{1F600}\"; funcion principal() { }",
    )
    .unwrap_err();
    assert!(e.contains("no cabe"), "{e}");
}

#[test]
fn un_texto_solo_inicia_arreglos_de_enteros() {
    let e = compilar_en("risc-16", "var t: u16 = \"a\"; funcion principal() { }").unwrap_err();
    assert!(e.contains("textos"), "{e}");
}

#[test]
fn no_se_puede_redefinir_una_funcion_de_la_biblioteca() {
    let e = compilar_en(
        "risc-16",
        "funcion cad_longitud(t: *u16) -> u16 { devolver 0; } funcion principal() { }",
    )
    .unwrap_err();
    assert!(e.contains("biblioteca de SGT"), "{e}");
}
