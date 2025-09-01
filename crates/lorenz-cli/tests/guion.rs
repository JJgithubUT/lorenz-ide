//! Pruebas de la lectura de guiones de entrada.

use lorenz_cli::guion::{leer_guion, Accion, ErrorGuion, Evento};

fn evento(instruccion: u64, accion: Accion) -> Evento {
    Evento {
        instruccion,
        accion,
    }
}

#[test]
fn un_guion_vacio_no_tiene_eventos() {
    assert_eq!(leer_guion("").unwrap(), []);
    assert_eq!(leer_guion("  \n# nada\n\n").unwrap(), []);
}

#[test]
fn una_tecla_en_decimal_y_en_hexadecimal() {
    let g = leer_guion("@0 teclado 65\n@1 teclado 0x42\n@2 teclado 0b1000011").unwrap();
    assert_eq!(
        g,
        [
            evento(0, Accion::Tecla(65)),
            evento(1, Accion::Tecla(0x42)),
            evento(2, Accion::Tecla(0b100_0011)),
        ]
    );
}

#[test]
fn una_interrupcion() {
    assert_eq!(
        leer_guion("@25 interrupcion 3").unwrap(),
        [evento(25, Accion::Interrupcion(3))]
    );
}

#[test]
fn texto_por_la_serie() {
    assert_eq!(
        leer_guion("@4 serie \"hola\"").unwrap(),
        [evento(4, Accion::Serie(b"hola".to_vec()))]
    );
}

#[test]
fn el_texto_admite_escapes() {
    let g = leer_guion(r#"@0 serie "a\nb\t\"c\"\\\0""#).unwrap();
    assert_eq!(g, [evento(0, Accion::Serie(b"a\nb\t\"c\"\\\0".to_vec()))]);
}

#[test]
fn el_texto_puede_tener_espacios_y_almohadillas() {
    let g = leer_guion("@0 serie \"a # b  c\"").unwrap();
    assert_eq!(g, [evento(0, Accion::Serie(b"a # b  c".to_vec()))]);
}

#[test]
fn el_texto_conserva_los_caracteres_no_ascii_como_utf8() {
    let g = leer_guion("@0 serie \"ñ\"").unwrap();
    assert_eq!(g, [evento(0, Accion::Serie("ñ".as_bytes().to_vec()))]);
}

#[test]
fn los_comentarios_al_final_de_la_linea_se_ignoran() {
    let g = leer_guion("@1 teclado 0x41   # la letra A\n").unwrap();
    assert_eq!(g, [evento(1, Accion::Tecla(0x41))]);
}

#[test]
fn un_comentario_despues_de_un_texto_se_ignora() {
    let g = leer_guion("@1 serie \"x\" # nota").unwrap();
    assert_eq!(g, [evento(1, Accion::Serie(b"x".to_vec()))]);
}

#[test]
fn los_eventos_salen_en_el_orden_escrito() {
    let g = leer_guion("@9 teclado 1\n@2 teclado 2").unwrap();
    assert_eq!(g[0].instruccion, 9);
    assert_eq!(g[1].instruccion, 2);
}

#[test]
fn varios_eventos_en_el_mismo_momento() {
    let g = leer_guion("@5 teclado 1\n@5 interrupcion 0").unwrap();
    assert_eq!(g.len(), 2);
    assert!(g.iter().all(|e| e.instruccion == 5));
}

#[test]
fn sin_arroba_es_un_error_con_su_linea() {
    assert_eq!(
        leer_guion("@0 teclado 1\n3 teclado 2"),
        Err(ErrorGuion::SinMomento {
            linea: 2,
            texto: "3".into()
        })
    );
}

#[test]
fn sin_accion_es_un_error() {
    assert_eq!(
        leer_guion("@3"),
        Err(ErrorGuion::SinAccion {
            linea: 1,
            momento: "@3".into()
        })
    );
}

#[test]
fn una_accion_desconocida_se_nombra() {
    assert_eq!(
        leer_guion("@0 raton 1"),
        Err(ErrorGuion::AccionDesconocida {
            linea: 1,
            accion: "raton".into()
        })
    );
}

#[test]
fn un_momento_invalido_es_un_error() {
    assert_eq!(
        leer_guion("@abc teclado 1"),
        Err(ErrorGuion::NumeroInvalido {
            linea: 1,
            texto: "abc".into()
        })
    );
}

#[test]
fn una_tecla_sin_valor_es_un_error() {
    assert_eq!(
        leer_guion("@0 teclado"),
        Err(ErrorGuion::SinArgumento {
            linea: 1,
            accion: "teclado".into()
        })
    );
}

#[test]
fn una_interrupcion_sin_linea_es_un_error() {
    assert_eq!(
        leer_guion("@0 interrupcion"),
        Err(ErrorGuion::SinArgumento {
            linea: 1,
            accion: "interrupcion".into()
        })
    );
}

#[test]
fn una_serie_sin_texto_es_un_error() {
    assert_eq!(
        leer_guion("@0 serie"),
        Err(ErrorGuion::SinArgumento {
            linea: 1,
            accion: "serie".into()
        })
    );
}

#[test]
fn una_tecla_con_valor_invalido_es_un_error() {
    assert_eq!(
        leer_guion("@0 teclado zz"),
        Err(ErrorGuion::NumeroInvalido {
            linea: 1,
            texto: "zz".into()
        })
    );
}

#[test]
fn una_linea_de_interrupcion_demasiado_grande_es_un_error() {
    assert!(matches!(
        leer_guion("@0 interrupcion 99999999999"),
        Err(ErrorGuion::NumeroInvalido { linea: 1, .. })
    ));
}

#[test]
fn un_texto_sin_comillas_es_un_error() {
    assert_eq!(
        leer_guion("@0 serie hola"),
        Err(ErrorGuion::TextoSinComillas { linea: 1 })
    );
}

#[test]
fn un_texto_sin_cerrar_es_un_error() {
    assert_eq!(
        leer_guion("@0 serie \"hola"),
        Err(ErrorGuion::TextoSinCerrar { linea: 1 })
    );
}

#[test]
fn una_barra_al_final_del_texto_es_un_error() {
    assert_eq!(
        leer_guion("@0 serie \"hola\\"),
        Err(ErrorGuion::TextoSinCerrar { linea: 1 })
    );
}

#[test]
fn un_escape_desconocido_es_un_error() {
    assert_eq!(
        leer_guion("@0 serie \"a\\qb\""),
        Err(ErrorGuion::EscapeInvalido {
            linea: 1,
            caracter: 'q'
        })
    );
}

#[test]
fn texto_de_sobra_al_final_es_un_error() {
    assert_eq!(
        leer_guion("@0 teclado 1 2"),
        Err(ErrorGuion::Sobra {
            linea: 1,
            texto: "2".into()
        })
    );
    assert_eq!(
        leer_guion("@0 serie \"x\" y"),
        Err(ErrorGuion::Sobra {
            linea: 1,
            texto: "y".into()
        })
    );
}

#[test]
fn el_error_cita_la_linea_correcta_tras_lineas_en_blanco_y_comentarios() {
    let e = leer_guion("# uno\n\n@0 teclado 1\n\n@x teclado 1").unwrap_err();
    assert!(matches!(e, ErrorGuion::NumeroInvalido { linea: 5, .. }));
}

#[test]
fn los_mensajes_estan_en_espanol_y_citan_la_linea() {
    let e = leer_guion("@0 raton 1").unwrap_err();
    let m = e.to_string();
    assert!(m.contains("linea 1"));
    assert!(m.contains("raton"));
}
