//! El analizador lexico de SGT.

use lorenz_sgt::lexico::{analizar, Token};

fn tokens(texto: &str) -> Vec<Token> {
    analizar("t.sgt", texto)
        .unwrap()
        .into_iter()
        .map(|l| l.token)
        .collect()
}

#[test]
fn un_texto_vacio_solo_tiene_fin() {
    assert_eq!(tokens(""), vec![Token::Fin]);
    assert_eq!(tokens("  \n\t "), vec![Token::Fin]);
}

#[test]
fn reconoce_palabras_reservadas_e_identificadores() {
    assert_eq!(
        tokens("funcion suma var si sino mientras bucle romper continuar devolver"),
        vec![
            Token::Funcion,
            Token::Ident("suma".into()),
            Token::Var,
            Token::Si,
            Token::Sino,
            Token::Mientras,
            Token::Bucle,
            Token::Romper,
            Token::Continuar,
            Token::Devolver,
            Token::Fin
        ]
    );
}

#[test]
fn un_identificador_que_empieza_como_palabra_reservada_no_lo_es() {
    assert_eq!(
        tokens("simple variable"),
        vec![
            Token::Ident("simple".into()),
            Token::Ident("variable".into()),
            Token::Fin
        ]
    );
}

#[test]
fn numeros_en_decimal_hexadecimal_binario_y_con_separadores() {
    assert_eq!(
        tokens("10 0xFF 0b1010 1_000 0x1_0"),
        vec![
            Token::Entero(10),
            Token::Entero(255),
            Token::Entero(10),
            Token::Entero(1000),
            Token::Entero(16),
            Token::Fin
        ]
    );
}

#[test]
fn una_cifra_invalida_se_senala_con_su_lugar() {
    let e = analizar("t.sgt", "var x = 0b102;").unwrap_err();
    assert!(e.mensaje.contains("base 2"), "{e}");
    assert_eq!((e.lugar.linea, e.lugar.columna), (1, 9));
}

#[test]
fn un_numero_enorme_se_rechaza() {
    assert!(analizar("t.sgt", "99999999999999999999999").is_err());
}

#[test]
fn caracteres_y_textos() {
    assert_eq!(
        tokens(r#"'a' '\n' "hola\tmundo""#),
        vec![
            Token::Entero(97),
            Token::Entero(10),
            Token::Cadena("hola\tmundo".into()),
            Token::Fin
        ]
    );
}

#[test]
fn un_texto_sin_cerrar_es_un_error() {
    let e = analizar("t.sgt", "\"abc").unwrap_err();
    assert!(e.mensaje.contains("sin cerrar"));
    assert!(e.sugerencia.is_some());
}

#[test]
fn operadores_de_uno_dos_y_tres_caracteres() {
    assert_eq!(
        tokens("+ +| - -| << <<= >> >>= <= >= == != && || -> => += &="),
        vec![
            Token::Mas,
            Token::MasSat,
            Token::Menos,
            Token::MenosSat,
            Token::DesplazaIzq,
            Token::DesplazaIzqIgual,
            Token::DesplazaDer,
            Token::DesplazaDerIgual,
            Token::MenorIgual,
            Token::MayorIgual,
            Token::IgualIgual,
            Token::Distinto,
            Token::AmpAmp,
            Token::TuberiaTuberia,
            Token::Flecha,
            Token::FlechaDoble,
            Token::MasIgual,
            Token::AmpersandIgual,
            Token::Fin
        ]
    );
}

#[test]
fn los_comentarios_no_producen_lexemas() {
    assert_eq!(
        tokens("a // hasta el final\n b /* en\nvarias lineas */ c"),
        vec![
            Token::Ident("a".into()),
            Token::Ident("b".into()),
            Token::Ident("c".into()),
            Token::Fin
        ]
    );
}

#[test]
fn un_comentario_de_bloque_sin_cerrar_es_un_error() {
    let e = analizar("t.sgt", "a /* nunca").unwrap_err();
    assert!(e.mensaje.contains("sin cerrar"));
}

#[test]
fn cada_lexema_lleva_su_linea_y_columna() {
    let l = analizar("t.sgt", "var x;\n  y").unwrap();
    let lugares: Vec<(u32, u32)> = l.iter().map(|x| (x.lugar.linea, x.lugar.columna)).collect();
    assert_eq!(lugares, vec![(1, 1), (1, 5), (1, 6), (2, 3), (2, 4)]);
}

#[test]
fn un_caracter_extrano_se_senala() {
    let e = analizar("t.sgt", "a # b").unwrap_err();
    assert!(e.to_string().starts_with("t.sgt:1:3:"), "{e}");
}
