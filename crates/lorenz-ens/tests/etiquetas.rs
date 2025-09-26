//! Etiquetas, saltos y segunda pasada (RF-ENS-003, RF-ENS-012).

mod comun;
use comun::*;
use lorenz_ens::error::Causa;

#[test]
fn una_etiqueta_hacia_atras_vale_su_direccion() {
    let m = ejemplo_8();
    let o = ensamblar(&m, "NOP\nNOP\nfin: HLT\nJZ fin").unwrap();
    assert_eq!(o.simbolo("fin"), Some(2));
    let palabras: Vec<u64> = o.palabras().into_iter().map(|(_, p)| p).collect();
    assert_eq!(palabras, vec![0x00, 0x00, 0xF0, 0x60, 0x02, 0x00]);
}

#[test]
fn una_etiqueta_hacia_adelante_se_resuelve_en_la_segunda_pasada() {
    let m = ejemplo_8();
    // JZ ocupa 3 palabras, asi que `fin` cae en la direccion 3.
    let b = bytes(&m, "JZ fin\nfin: HLT");
    assert_eq!(b, vec![0x60, 0x03, 0x00, 0xF0]);
}

#[test]
fn una_etiqueta_sola_en_su_linea_apunta_a_la_siguiente_instruccion() {
    let m = ejemplo_8();
    let o = ensamblar(&m, "NOP\nmedio:\nHLT").unwrap();
    assert_eq!(o.simbolo("medio"), Some(1));
}

#[test]
fn varias_etiquetas_pueden_compartir_direccion() {
    let m = ejemplo_8();
    let o = ensamblar(&m, "a: b: HLT").unwrap();
    assert_eq!(o.simbolo("a"), o.simbolo("b"));
}

#[test]
fn una_etiqueta_duplicada_indica_la_linea_previa() {
    let m = ejemplo_8();
    let e = errores(&m, "x: NOP\nx: HLT");
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].linea, 2);
    assert!(matches!(
        e[0].causa,
        Causa::EtiquetaDuplicada { previa: 1, .. }
    ));
}

#[test]
fn un_simbolo_indefinido_se_reporta_en_su_columna() {
    let m = ejemplo_8();
    let e = errores(&m, "JZ nada");
    assert_eq!(e[0].columna, 4);
    assert!(e[0].to_string().contains("`nada`"));
}

#[test]
fn el_salto_relativo_se_mide_desde_la_instruccion_siguiente() {
    let m = con_saltos_relativos("little");
    // JR ocupa 2 palabras; el destino en 0 da desplazamiento -2.
    assert_eq!(bytes(&m, "inicio: JR inicio"), vec![0x70, 0xFE]);
    // Hacia adelante: JR (2) + NOP (1) y destino en 3: desplazamiento 1.
    assert_eq!(
        bytes(&m, "JR fin\nNOP\nfin: NOP"),
        vec![0x70, 0x01, 0x00, 0x00]
    );
}

#[test]
fn un_salto_demasiado_largo_indica_el_alcance_maximo() {
    let m = con_saltos_relativos("little");
    let e = errores(&m, "JR lejos\n.espacio 200\nlejos: NOP");
    match &e[0].causa {
        Causa::SaltoFueraDeAlcance {
            minimo,
            maximo,
            desplazamiento,
        } => {
            assert_eq!((*minimo, *maximo), (-128, 127));
            assert_eq!(*desplazamiento, 200);
        }
        otra => panic!("{otra:?}"),
    }
    assert!(e[0].to_string().contains("-128 a 127"));
}

#[test]
fn el_limite_exacto_del_salto_si_cabe() {
    let m = con_saltos_relativos("little");
    // Destino a 127 palabras de la instruccion siguiente.
    assert!(ensamblar(&m, "JR d\n.espacio 127\nd: NOP").is_ok());
    assert!(ensamblar(&m, "JR d\n.espacio 128\nd: NOP").is_err());
}

#[test]
fn una_direccion_repartida_respeta_el_endianness() {
    let le = con_saltos_relativos("little");
    let be = con_saltos_relativos("big");
    assert_eq!(bytes(&le, "JP 0x1234"), vec![0x80, 0x34, 0x12]);
    assert_eq!(bytes(&be, "JP 0x1234"), vec![0x80, 0x12, 0x34]);
}

#[test]
fn una_direccion_fuera_del_espacio_se_rechaza() {
    let m = con_saltos_relativos("little");
    let e = errores(&m, "JP 0x10000");
    assert!(matches!(e[0].causa, Causa::DireccionFueraDeRango { .. }));
}

#[test]
fn puertos_e_inmediatos_con_signo() {
    let m = con_saltos_relativos("little");
    assert_eq!(bytes(&m, "OUTP 0x30"), vec![0x90, 0x30]);
    assert!(ensamblar(&m, "OUTP 256").is_err());
    assert_eq!(bytes(&m, "ADDI -1"), vec![0xA0, 0xFF]);
    assert_eq!(bytes(&m, "ADDI -128"), vec![0xA0, 0x80]);
    assert!(ensamblar(&m, "ADDI 128").is_err());
    assert!(ensamblar(&m, "ADDI -129").is_err());
}
