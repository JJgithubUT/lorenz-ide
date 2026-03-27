//! Constantes, expresiones, literales y directivas (RF-ENS-004 a RF-ENS-007).

mod comun;
use comun::*;
use lorenz_ens::error::Causa;

#[test]
fn una_constante_con_igual_se_usa_como_inmediato() {
    let m = ejemplo_8();
    assert_eq!(bytes(&m, "VALOR = 42\nLDI A, VALOR"), vec![0x20, 42]);
}

#[test]
fn una_constante_con_equ_se_usa_igual() {
    let m = ejemplo_8();
    assert_eq!(bytes(&m, ".equ VALOR, 7\nLDI B, VALOR"), vec![0x24, 7]);
}

#[test]
fn una_constante_puede_usar_otra() {
    let m = ejemplo_8();
    assert_eq!(bytes(&m, "A1 = 3\nB1 = A1 * 4\nLDI A, B1"), vec![0x20, 12]);
}

#[test]
fn una_constante_no_ocupa_memoria_ni_es_etiqueta() {
    let m = ejemplo_8();
    let o = ensamblar(&m, "K = 5\nHLT").unwrap();
    assert_eq!(o.tamano(), 1);
    assert!(!o.simbolos["K"].etiqueta);
}

#[test]
fn una_constante_no_puede_usar_un_simbolo_posterior() {
    let m = ejemplo_8();
    let e = errores(&m, "K = despues\ndespues: HLT");
    assert!(e[0].to_string().contains("despues"));
}

#[test]
fn las_expresiones_se_evaluan_en_los_operandos() {
    let m = ejemplo_8();
    assert_eq!(bytes(&m, "LDI A, (3 + 4) * 2"), vec![0x20, 14]);
    assert_eq!(bytes(&m, "LDI A, 1 << 4 | 1"), vec![0x20, 17]);
    assert_eq!(bytes(&m, "LD A, [0x2000 + 2 * 8]"), vec![0x40, 0x10, 0x20]);
}

#[test]
fn los_literales_en_cuatro_bases() {
    let m = ejemplo_8();
    assert_eq!(bytes(&m, "LDI A, 65"), vec![0x20, 65]);
    assert_eq!(bytes(&m, "LDI A, 0x41"), vec![0x20, 65]);
    assert_eq!(bytes(&m, "LDI A, 0b1000001"), vec![0x20, 65]);
    assert_eq!(bytes(&m, "LDI A, 'A'"), vec![0x20, 65]);
    assert_eq!(bytes(&m, "LDI A, $41"), vec![0x20, 65]);
}

#[test]
fn el_simbolo_dolar_es_la_direccion_actual() {
    let m = ejemplo_8();
    assert_eq!(bytes(&m, "NOP\nNOP\nLDI A, $"), vec![0, 0, 0x20, 2]);
}

#[test]
fn org_cambia_la_direccion_y_abre_un_segmento() {
    let m = ejemplo_8();
    let o = ensamblar(&m, "HLT\n.org 0x0100\nNOP\nHLT").unwrap();
    assert_eq!(o.segmentos.len(), 2);
    assert_eq!(o.segmentos[1].direccion, 0x100);
    assert_eq!(o.segmentos[1].palabras, vec![0x00, 0xF0]);
}

#[test]
fn org_acepta_expresiones_con_constantes() {
    let m = ejemplo_8();
    let o = ensamblar(&m, "BASE = 0x40\n.org BASE + 2\nHLT").unwrap();
    assert_eq!(o.segmentos[0].direccion, 0x42);
}

#[test]
fn org_fuera_del_espacio_se_rechaza() {
    let m = ejemplo_8();
    let e = errores(&m, ".org 0x10000");
    assert!(matches!(e[0].causa, Causa::OrigenInvalido { .. }));
}

#[test]
fn db_emite_una_palabra_por_valor() {
    let m = ejemplo_8();
    assert_eq!(bytes(&m, ".db 1, 2, 0xFF, 'A'"), vec![1, 2, 255, 65]);
}

#[test]
fn db_con_cadena_emite_un_caracter_por_palabra() {
    let m = ejemplo_8();
    assert_eq!(bytes(&m, r#".db "Hola", 0"#), vec![72, 111, 108, 97, 0]);
    assert_eq!(bytes(&m, r#".db "a\n\t\0""#), vec![97, 10, 9, 0]);
    assert_eq!(bytes(&m, r#".db "a;b""#), vec![97, 59, 98]);
}

#[test]
fn db_rechaza_un_valor_que_no_cabe() {
    let m = ejemplo_8();
    let e = errores(&m, ".db 256");
    assert!(matches!(
        e[0].causa,
        Causa::DatoFueraDeRango {
            valor: 256,
            bits: 8
        }
    ));
    assert!(ensamblar(&m, ".db -128").is_ok());
    assert!(ensamblar(&m, ".db -129").is_err());
}

#[test]
fn dw_emite_direcciones_segun_el_endianness() {
    let le = con_saltos_relativos("little");
    let be = con_saltos_relativos("big");
    assert_eq!(bytes(&le, ".dw 0x1234, 1"), vec![0x34, 0x12, 1, 0]);
    assert_eq!(bytes(&be, ".dw 0x1234, 1"), vec![0x12, 0x34, 0, 1]);
}

#[test]
fn dw_puede_apuntar_a_una_etiqueta() {
    let m = ejemplo_8();
    let b = bytes(&m, "tabla: .dw destino\ndestino: HLT");
    assert_eq!(b, vec![0x02, 0x00, 0xF0]);
}

#[test]
fn espacio_reserva_palabras_con_cero_o_con_relleno() {
    let m = ejemplo_8();
    assert_eq!(bytes(&m, ".espacio 3"), vec![0, 0, 0]);
    assert_eq!(bytes(&m, ".espacio 2, 0xAA"), vec![0xAA, 0xAA]);
    assert_eq!(bytes(&m, ".space 1\n.res 1"), vec![0, 0]);
}

#[test]
fn espacio_avanza_la_direccion_de_lo_que_sigue() {
    let m = ejemplo_8();
    let o = ensamblar(&m, ".espacio 4\nfin: HLT").unwrap();
    assert_eq!(o.simbolo("fin"), Some(4));
}

#[test]
fn directivas_con_argumentos_incorrectos() {
    let m = ejemplo_8();
    for t in [
        ".org",
        ".org 1, 2",
        ".equ X",
        ".db",
        ".dw",
        ".espacio",
        ".espacio 1, 2, 3",
    ] {
        let e = errores(&m, t);
        assert!(
            matches!(e[0].causa, Causa::ArgumentosDeDirectiva { .. }),
            "{t}: {:?}",
            e[0].causa
        );
    }
}

#[test]
fn una_directiva_desconocida_se_reporta() {
    let m = ejemplo_8();
    let e = errores(&m, ".nada 3");
    assert!(matches!(e[0].causa, Causa::DirectivaDesconocida(_)));
    assert_eq!((e[0].linea, e[0].columna), (1, 1));
}

#[test]
fn las_directivas_no_distinguen_mayusculas() {
    let m = ejemplo_8();
    assert_eq!(bytes(&m, ".DB 1\n.Org 0x10\n.DB 2").len(), 2);
}
