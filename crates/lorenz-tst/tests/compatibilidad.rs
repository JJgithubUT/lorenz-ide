//! Binarios anteriores frente a una ISA que cambio (RF-TST-029).

use std::path::PathBuf;

use lorenz_arch::modelo::Maquina;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};
use lorenz_tst::compatibilidad::{comprobar_binario, Motivo};

fn texto_base() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ejemplos/ejemplo-8/maquina.toml"),
    )
    .unwrap()
}

fn base() -> Maquina {
    lorenz_arch::carga::cargar(&texto_base()).unwrap()
}

/// La arquitectura base con un pedazo de su descripcion cambiado.
fn cambiada(de: &str, a: &str) -> Maquina {
    let t = texto_base();
    assert!(t.contains(de), "el texto a cambiar no esta: {de}");
    lorenz_arch::carga::cargar(&t.replacen(de, a, 1)).unwrap_or_else(|e| panic!("{e}"))
}

fn binario(m: &Maquina, fuente: &str) -> Vec<(u64, Vec<u64>)> {
    let o = Ensamblador::nuevo(m)
        .unwrap()
        .ensamblar(&Fuente::nueva("p.s", fuente))
        .unwrap();
    o.segmentos
        .into_iter()
        .map(|s| (s.direccion, s.palabras))
        .collect()
}

const SUMA: &str = "LDI A, 200\nLDI B, 100\nADD A, B\nHLT\n";

#[test]
fn la_misma_arquitectura_es_compatible() {
    let m = base();
    let i = comprobar_binario(&m, &m, &binario(&m, SUMA)).unwrap();
    assert!(i.es_compatible());
    assert_eq!(i.instrucciones, 4);
    assert!(i
        .a_texto()
        .contains("4 instrucciones revisadas, 0 incompatibles"));
}

#[test]
fn una_instruccion_que_cambia_de_codigo_deja_de_reconocerse() {
    let m = base();
    let nueva = cambiada("codigo    = \"1111 ....\"", "codigo    = \"1110 ....\"");
    let i = comprobar_binario(&m, &nueva, &binario(&m, SUMA)).unwrap();
    assert!(!i.es_compatible());
    assert_eq!(i.incompatibles.len(), 1);
    let x = &i.incompatibles[0];
    assert_eq!(x.antes, "HLT");
    assert_eq!(x.direccion, 5);
    assert_eq!(x.motivo, Motivo::NoSeReconoce);
    assert!(i
        .a_texto()
        .contains("0x0005 [f0] `HLT`: la arquitectura nueva no la reconoce"));
}

#[test]
fn cambiar_el_codigo_de_una_instruccion_por_el_de_otra_cambia_su_significado() {
    let m = base();
    // MOV y ADD intercambian codigos.
    let t = texto_base()
        .replacen("\"0001 rd:2 rs:2\"", "\"@@@@ rd:2 rs:2\"", 1)
        .replacen("\"0011 rd:2 rs:2\"", "\"0001 rd:2 rs:2\"", 1)
        .replacen("\"@@@@ rd:2 rs:2\"", "\"0011 rd:2 rs:2\"", 1);
    let nueva = lorenz_arch::carga::cargar(&t).unwrap();
    let i = comprobar_binario(&m, &nueva, &binario(&m, SUMA)).unwrap();
    assert_eq!(i.incompatibles.len(), 1, "{}", i.a_texto());
    assert_eq!(i.incompatibles[0].antes, "ADD A, B");
    assert!(
        matches!(&i.incompatibles[0].motivo, Motivo::OtraInstruccion { ahora } if ahora == "MOV A, B")
    );
}

#[test]
fn si_la_instruccion_sigue_pero_cambia_lo_que_hace_se_avisa() {
    let m = base();
    let nueva = cambiada(
        "semantica = \"rd = rd + rs\"",
        "semantica = \"rd = rd - rs\"",
    );
    let i = comprobar_binario(&m, &nueva, &binario(&m, SUMA)).unwrap();
    assert_eq!(i.incompatibles.len(), 1);
    assert!(
        matches!(&i.incompatibles[0].motivo, Motivo::CambiaElComportamiento { que } if que.contains("rd = rd + rs") && que.contains("rd = rd - rs")),
        "{:?}",
        i.incompatibles
    );
}

#[test]
fn cambiar_los_ciclos_tambien_cuenta() {
    let m = base();
    let nueva = cambiada(
        "ciclos    = 2\ndescripcion = \"Suma",
        "ciclos    = 3\ndescripcion = \"Suma",
    );
    let i = comprobar_binario(&m, &nueva, &binario(&m, SUMA)).unwrap();
    assert_eq!(i.incompatibles.len(), 1);
    assert!(
        matches!(&i.incompatibles[0].motivo, Motivo::CambiaElComportamiento { que } if que.contains("ciclos"))
    );
}

#[test]
fn las_palabras_que_ya_eran_datos_no_cuentan() {
    let m = base();
    let nueva = cambiada("codigo    = \"1111 ....\"", "codigo    = \"1110 ....\"");
    // LDI A, 200 y DI: ninguna cambio, aunque HLT ya no se reconozca.
    let tramos = vec![(0u64, vec![0x20, 200, 0x72])];
    let i = comprobar_binario(&m, &nueva, &tramos).unwrap();
    assert!(i.es_compatible(), "{}", i.a_texto());
}

#[test]
fn otro_ancho_de_palabra_invalida_cualquier_binario() {
    let m = base();
    let nueva = lorenz_arch::carga::cargar(
        &texto_base().replace("bits_palabra   = 8", "bits_palabra   = 16"),
    );
    // Una arquitectura con otro ancho de palabra puede ni cargar; si carga, el informe es global.
    if let Ok(nueva) = nueva {
        let i = comprobar_binario(&m, &nueva, &binario(&m, SUMA)).unwrap();
        assert!(i.incompatibilidad_global.is_some());
        assert!(i.a_texto().contains("INCOMPATIBLE"));
    }
}

#[test]
fn el_informe_viaja_como_json() {
    let m = base();
    let nueva = cambiada("codigo    = \"1111 ....\"", "codigo    = \"1110 ....\"");
    let i = comprobar_binario(&m, &nueva, &binario(&m, SUMA)).unwrap();
    let v: serde_json::Value = serde_json::from_str(&i.a_json()).unwrap();
    assert_eq!(v["incompatibles"][0]["motivo"]["tipo"], "no_se_reconoce");
}
