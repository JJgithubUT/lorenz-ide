//! Pruebas de la lectura de programas en texto.

use lorenz_cli::programa::{leer_palabras, ErrorPrograma};

#[test]
fn un_programa_vacio_no_tiene_palabras() {
    assert_eq!(leer_palabras("", 8).unwrap(), Vec::<u64>::new());
}

#[test]
fn lee_palabras_hexadecimales_separadas_por_espacios() {
    assert_eq!(leer_palabras("20 2A F0", 8).unwrap(), [0x20, 0x2A, 0xF0]);
}

#[test]
fn acepta_el_prefijo_0x_en_mayusculas_y_minusculas() {
    assert_eq!(leer_palabras("0x1f 0X1F", 8).unwrap(), [0x1F, 0x1F]);
}

#[test]
fn acepta_palabras_binarias() {
    assert_eq!(leer_palabras("0b00100000 0B1", 8).unwrap(), [0x20, 1]);
}

#[test]
fn acepta_comas_y_saltos_de_linea() {
    assert_eq!(leer_palabras("20, 05\nF0,\n", 8).unwrap(), [0x20, 5, 0xF0]);
}

#[test]
fn ignora_los_comentarios() {
    let texto = "# programa de prueba\n20 05 # LDI A, 5\nF0 # HLT\n";
    assert_eq!(leer_palabras(texto, 8).unwrap(), [0x20, 5, 0xF0]);
}

#[test]
fn una_linea_solo_de_comentario_no_aporta_nada() {
    assert!(leer_palabras("   # nada\n\n", 8).unwrap().is_empty());
}

#[test]
fn una_palabra_invalida_cita_la_linea_y_el_texto() {
    let e = leer_palabras("20\n05 zz\n", 8).unwrap_err();
    assert_eq!(e, ErrorPrograma::PalabraInvalida { linea: 2, texto: "zz".into() });
}

#[test]
fn un_prefijo_sin_digitos_es_invalido() {
    assert!(matches!(leer_palabras("0x", 8), Err(ErrorPrograma::PalabraInvalida { .. })));
    assert!(matches!(leer_palabras("0b", 8), Err(ErrorPrograma::PalabraInvalida { .. })));
}

#[test]
fn un_digito_binario_incorrecto_es_invalido() {
    assert!(matches!(leer_palabras("0b102", 8), Err(ErrorPrograma::PalabraInvalida { .. })));
}

#[test]
fn una_palabra_que_no_cabe_se_rechaza() {
    let e = leer_palabras("20\n1FF\n", 8).unwrap_err();
    assert_eq!(e, ErrorPrograma::PalabraDemasiadoGrande { linea: 2, valor: 0x1FF, bits: 8 });
}

#[test]
fn el_ancho_de_palabra_decide_el_limite() {
    assert_eq!(leer_palabras("1FF", 12).unwrap(), [0x1FF]);
    assert!(leer_palabras("1000", 12).is_err());
}

#[test]
fn con_64_bits_cabe_cualquier_valor() {
    assert_eq!(leer_palabras("FFFFFFFFFFFFFFFF", 64).unwrap(), [u64::MAX]);
}

#[test]
fn un_valor_mas_grande_que_64_bits_es_invalido() {
    assert!(matches!(leer_palabras("1FFFFFFFFFFFFFFFF", 64), Err(ErrorPrograma::PalabraInvalida { .. })));
}

#[test]
fn los_mensajes_de_error_hablan_de_la_linea() {
    let e = leer_palabras("zz", 8).unwrap_err();
    assert!(e.to_string().contains("linea 1"));
}
