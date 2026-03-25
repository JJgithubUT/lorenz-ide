//! Aviso de banderas indeterminadas antes de un salto condicional (RF-ENS-028).

mod comun;
use comun::*;

fn avisos(texto: &str) -> Vec<String> {
    let m = ejemplo_8();
    ensamblar(&m, texto)
        .unwrap()
        .avisos
        .iter()
        .map(|a| a.to_string())
        .collect()
}

#[test]
fn saltar_sin_fijar_la_bandera_avisa() {
    let a = avisos("LDI A, 1\nJZ fin\nfin: HLT\n");
    assert_eq!(a.len(), 1, "{a:?}");
    assert!(a[0].starts_with("prueba.s:2: aviso:"), "{a:?}");
    assert!(a[0].contains("bandera Z"), "{a:?}");
    assert!(a[0].contains("ninguna instruccion"), "{a:?}");
}

#[test]
fn fijarla_antes_evita_el_aviso() {
    assert!(avisos("LDI A, 1\nLDI B, 1\nADD A, B\nJZ fin\nfin: HLT\n").is_empty());
}

#[test]
fn tras_una_etiqueta_no_se_sabe_y_se_calla() {
    assert!(avisos("LDI A, 1\nbucle: JZ bucle\nHLT\n").is_empty());
}

#[test]
fn las_instrucciones_que_no_tocan_banderas_no_las_fijan() {
    let a = avisos("LDI A, 1\nMOV B, A\nST [0x10], B\nJZ fin\nfin: HLT\n");
    assert_eq!(a.len(), 1, "{a:?}");
}

#[test]
fn el_ensamblado_sigue_siendo_correcto_con_aviso() {
    let m = ejemplo_8();
    assert!(ensamblar(&m, "JZ 0\nHLT\n").is_ok());
}
