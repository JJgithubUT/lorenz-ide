//! Pruebas del analizador lexico de la semantica (RF-ARQ-022).

use lorenz_arch::semantica::lexico::{tokenizar, Token};

fn tokens(texto: &str) -> Vec<Token> {
    tokenizar(texto).unwrap().into_iter().map(|l| l.token).collect()
}

#[test]
fn un_texto_vacio_solo_produce_fin() {
    assert_eq!(tokens(""), vec![Token::Fin]);
}

#[test]
fn el_espacio_en_blanco_se_ignora() {
    assert_eq!(tokens("  \t\n "), vec![Token::Fin]);
}

#[test]
fn lee_un_numero_decimal() {
    assert_eq!(tokens("42"), vec![Token::Numero(42), Token::Fin]);
}

#[test]
fn lee_un_numero_hexadecimal() {
    assert_eq!(tokens("0xFF"), vec![Token::Numero(255), Token::Fin]);
    assert_eq!(tokens("0Xff"), vec![Token::Numero(255), Token::Fin]);
}

#[test]
fn lee_un_numero_binario() {
    assert_eq!(tokens("0b1010"), vec![Token::Numero(10), Token::Fin]);
}

#[test]
fn admite_guion_bajo_como_separador() {
    assert_eq!(tokens("0b1111_0000"), vec![Token::Numero(240), Token::Fin]);
    assert_eq!(tokens("1_000"), vec![Token::Numero(1000), Token::Fin]);
}

#[test]
fn el_cero_solo_es_un_numero() {
    assert_eq!(tokens("0"), vec![Token::Numero(0), Token::Fin]);
}

#[test]
fn rechaza_un_hexadecimal_sin_digitos() {
    assert!(tokenizar("0x").is_err());
}

#[test]
fn rechaza_un_digito_invalido_en_binario() {
    assert!(tokenizar("0b102").is_err());
}

#[test]
fn rechaza_un_numero_que_no_cabe_en_64_bits() {
    assert!(tokenizar("0x1_0000_0000_0000_0000").is_err());
}

#[test]
fn rechaza_letras_pegadas_a_un_numero() {
    assert!(tokenizar("12ab").is_err());
}

#[test]
fn lee_identificadores() {
    assert_eq!(
        tokens("rd rs_2 _x PC"),
        vec![
            Token::Ident("rd".into()),
            Token::Ident("rs_2".into()),
            Token::Ident("_x".into()),
            Token::Ident("PC".into()),
            Token::Fin
        ]
    );
}

#[test]
fn reconoce_las_palabras_reservadas() {
    assert_eq!(tokens("si"), vec![Token::Si, Token::Fin]);
    assert_eq!(tokens("detener"), vec![Token::Detener, Token::Fin]);
}

#[test]
fn una_palabra_reservada_dentro_de_un_identificador_no_lo_es() {
    assert_eq!(tokens("silla"), vec![Token::Ident("silla".into()), Token::Fin]);
    assert_eq!(tokens("detenerse"), vec![Token::Ident("detenerse".into()), Token::Fin]);
}

#[test]
fn las_palabras_reservadas_distinguen_mayusculas() {
    assert_eq!(tokens("Si"), vec![Token::Ident("Si".into()), Token::Fin]);
}

#[test]
fn mem_in_y_out_son_identificadores() {
    assert_eq!(
        tokens("mem in out"),
        vec![
            Token::Ident("mem".into()),
            Token::Ident("in".into()),
            Token::Ident("out".into()),
            Token::Fin
        ]
    );
}
