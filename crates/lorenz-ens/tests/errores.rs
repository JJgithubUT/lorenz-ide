//! Errores con posicion y causa en terminos de la ISA (RF-ENS-011 a RF-ENS-015).

mod comun;
use comun::*;
use lorenz_ens::error::Causa;

#[test]
fn mnemonico_desconocido_con_columna() {
    let m = ejemplo_8();
    let e = errores(&m, "  FOO A, B");
    assert_eq!(
        (e[0].archivo.as_str(), e[0].linea, e[0].columna),
        ("prueba.s", 1, 3)
    );
    assert!(matches!(&e[0].causa, Causa::MnemonicoDesconocido(x) if x == "FOO"));
}

#[test]
fn registro_invalido_nombra_los_permitidos() {
    let m = ejemplo_8();
    let e = errores(&m, "MOV A, Z");
    assert_eq!(e[0].columna, 8);
    match &e[0].causa {
        Causa::RegistroInvalido {
            operando,
            escrito,
            permitidos,
        } => {
            assert_eq!(operando, "rs");
            assert_eq!(escrito, "Z");
            assert_eq!(permitidos, &["A", "B", "C", "D"]);
        }
        otra => panic!("{otra:?}"),
    }
}

#[test]
fn registro_repetido_senala_el_correcto() {
    let m = ejemplo_8();
    let e = errores(&m, "MOV Q, Q");
    assert_eq!(e[0].columna, 5);
}

#[test]
fn operandos_que_no_encajan_listan_las_formas_validas() {
    let m = ejemplo_8();
    let e = errores(&m, "ADD A");
    match &e[0].causa {
        Causa::OperandosNoEncajan { mnemonico, formas } => {
            assert_eq!(mnemonico, "ADD");
            assert_eq!(formas, &["ADD {rd}, {rs}"]);
        }
        otra => panic!("{otra:?}"),
    }
    assert!(e[0].to_string().contains("formas validas: ADD {rd}, {rs}"));
}

#[test]
fn operandos_de_mas_tambien_se_rechazan() {
    let m = ejemplo_8();
    assert!(ensamblar(&m, "HLT A").is_err());
    assert!(ensamblar(&m, "ADD A, B, C").is_err());
}

#[test]
fn inmediato_fuera_de_rango_indica_los_limites_y_la_columna() {
    let m = ejemplo_8();
    let e = errores(&m, "LDI A, 300");
    assert_eq!(e[0].columna, 8);
    assert!(e[0].to_string().contains("de 0 a 255"));
    assert!(errores(&m, "LDI A, -1")[0]
        .to_string()
        .contains("de 0 a 255"));
}

#[test]
fn un_error_de_expresion_no_impide_ver_los_siguientes() {
    let m = ejemplo_8();
    let e = errores(&m, "LDI A, 1 +\nFOO\nHLT\nBAR");
    assert_eq!(e.iter().map(|x| x.linea).collect::<Vec<_>>(), vec![1, 2, 4]);
}

#[test]
fn varios_errores_de_la_segunda_pasada_se_acumulan() {
    let m = ejemplo_8();
    let e = errores(&m, "LDI A, 256\nLDI B, nada\nLD A, [0x10000]\nHLT");
    assert_eq!(e.len(), 3);
}

#[test]
fn si_la_primera_pasada_falla_no_se_inventan_errores_de_direcciones() {
    let m = ejemplo_8();
    // `FOO` no se puede medir; sin esta regla `fin` tendria una direccion falsa y el salto
    // reportaria un error que no existe.
    let e = errores(&m, "FOO\nJZ fin\nfin: HLT");
    assert_eq!(e.len(), 1);
}

#[test]
fn un_error_de_sintaxis_de_linea_lleva_columna() {
    let m = ejemplo_8();
    let e = errores(&m, "NOP\n  @");
    assert_eq!((e[0].linea, e[0].columna), (2, 3));
}

#[test]
fn codigo_en_una_region_inexistente_se_rechaza() {
    let m = con_saltos_relativos("little");
    // La maquina cubre todo 0..=0xFFFF; usamos ejemplo_8 con un hueco: 0xF100 no tiene region.
    let _ = m;
    let m = ejemplo_8();
    let e = errores(&m, ".org 0xF100\nNOP");
    assert!(matches!(e[0].causa, Causa::SinRegion { direccion: 0xF100 }));
}

#[test]
fn codigo_sobre_un_dispositivo_se_rechaza() {
    let m = ejemplo_8();
    let e = errores(&m, ".org 0xF000\nNOP");
    match &e[0].causa {
        Causa::RegionNoEscribible { region, .. } => assert_eq!(region, "video"),
        otra => panic!("{otra:?}"),
    }
}

#[test]
fn una_rom_de_solo_lectura_si_admite_codigo() {
    let m = ejemplo_8();
    // La ROM es donde vive el programa: el ensamblador produce su imagen.
    assert!(ensamblar(&m, ".org 0x1FFF\nNOP").is_ok());
}

#[test]
fn un_elemento_que_cruza_regiones_se_rechaza() {
    let m = ejemplo_8();
    // LDI ocupa dos palabras: 0x1FFF esta en la ROM y 0x2000 en la RAM.
    let e = errores(&m, ".org 0x1FFF\nLDI A, 1");
    assert!(e[0].to_string().contains("cruza"));
}

#[test]
fn dos_fragmentos_que_escriben_la_misma_direccion_se_rechazan() {
    let m = ejemplo_8();
    let e = errores(&m, "NOP\nNOP\n.org 0\nHLT");
    assert!(matches!(e[0].causa, Causa::Solape { direccion: 0 }));
}

#[test]
fn comillas_sin_cerrar_en_una_cadena() {
    let m = ejemplo_8();
    assert!(ensamblar(&m, ".db \"abc").is_err());
    assert!(ensamblar(&m, r#".db "a\q""#).is_err());
}

#[test]
fn el_texto_del_error_incluye_archivo_linea_y_columna() {
    let m = ejemplo_8();
    let e = errores(&m, "NOP\nBAR");
    assert!(e[0].to_string().starts_with("prueba.s:2:1: "));
}
