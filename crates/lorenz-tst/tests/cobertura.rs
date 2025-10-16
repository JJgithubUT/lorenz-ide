//! Cobertura de instrucciones y ramas (RF-TST-021 a RF-TST-023).

use std::path::PathBuf;

use lorenz_arch::modelo::Maquina;
use lorenz_emu::Paso;
use lorenz_tst::cobertura::{es_ramificacion, Cobertura};

fn maquina() -> Maquina {
    let ruta =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ejemplos/ejemplo-8/maquina.toml");
    lorenz_arch::carga::cargar_archivo(ruta).unwrap()
}

fn paso(mnemonico: &str, direccion: u64, salto: bool) -> Paso {
    Paso {
        direccion,
        mnemonico: mnemonico.into(),
        longitud: 1,
        ciclos: 1,
        salto,
        detenida: false,
    }
}

#[test]
fn solo_jz_es_un_salto_condicional_en_el_ejemplo() {
    let m = maquina();
    let ramas: Vec<&str> = m
        .instrucciones
        .iter()
        .filter(|i| es_ramificacion(&m, i))
        .map(|i| i.mnemonico.as_str())
        .collect();
    assert_eq!(ramas, ["JZ"]);
}

#[test]
fn cuenta_las_ejecuciones_y_nombra_las_instrucciones_sin_ejercitar() {
    let m = maquina();
    let mut c = Cobertura::nueva(&m);
    c.registrar("p.s", &paso("LDI", 0, false));
    c.registrar("p.s", &paso("LDI", 2, false));
    c.registrar("p.s", &paso("HLT", 4, false));
    let i = c.informe();
    assert_eq!(i.instrucciones_ejercitadas, 2);
    let ldi = i
        .instrucciones
        .iter()
        .find(|x| x.mnemonico == "LDI")
        .unwrap();
    assert_eq!(ldi.ejecuciones, 2);
    let sin = i.no_ejercitadas();
    assert!(sin.contains(&"ADD") && sin.contains(&"JZ"));
    assert!(!sin.contains(&"LDI"));
    assert!((i.porcentaje_de_instrucciones() - 200.0 / 11.0).abs() < 1e-9);
}

#[test]
fn una_rama_esta_completa_cuando_se_vio_tomada_y_sin_tomar() {
    let m = maquina();
    let mut c = Cobertura::nueva(&m);
    c.registrar("p.s", &paso("JZ", 8, false));
    let i = c.informe();
    assert_eq!(i.ramas_cubiertas(), (1, 2));
    assert!(i.a_texto().contains("p.s:0x0008 JZ: nunca se toma"));
    c.registrar("p.s", &paso("JZ", 8, true));
    let i = c.informe();
    assert_eq!(i.ramas_cubiertas(), (2, 2));
    assert!(!i.a_texto().contains("nunca"));
}

#[test]
fn el_mismo_salto_en_dos_programas_son_dos_ramas() {
    let m = maquina();
    let mut c = Cobertura::nueva(&m);
    c.registrar("a.s", &paso("JZ", 8, true));
    c.registrar("b.s", &paso("JZ", 8, false));
    assert_eq!(c.informe().ramas.len(), 2);
}

#[test]
fn fusionar_suma_lo_visto_en_cada_ejecucion() {
    let m = maquina();
    let mut a = Cobertura::nueva(&m);
    let mut b = Cobertura::nueva(&m);
    a.registrar("p.s", &paso("JZ", 8, true));
    b.registrar("p.s", &paso("JZ", 8, false));
    b.registrar("p.s", &paso("ADD", 1, false));
    a.fusionar(&b);
    let i = a.informe();
    assert_eq!(i.ramas_cubiertas(), (2, 2));
    assert_eq!(
        i.instrucciones
            .iter()
            .find(|x| x.mnemonico == "ADD")
            .unwrap()
            .ejecuciones,
        1
    );
}

#[test]
fn sin_ejecutar_nada_el_informe_lo_dice() {
    let m = maquina();
    let i = Cobertura::nueva(&m).informe();
    assert_eq!(i.instrucciones_ejercitadas, 0);
    assert!(i.a_texto().contains("ningun salto condicional ejecutado"));
    assert_eq!(i.porcentaje_de_ramas(), 100.0);
}

#[test]
fn el_informe_viaja_como_json() {
    let m = maquina();
    let mut c = Cobertura::nueva(&m);
    c.registrar("p.s", &paso("JZ", 8, true));
    let i = c.informe();
    let t = serde_json::to_string(&i).unwrap();
    let otra: lorenz_tst::cobertura::InformeCobertura = serde_json::from_str(&t).unwrap();
    assert_eq!(i, otra);
}
