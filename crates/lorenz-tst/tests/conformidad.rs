//! Suite de conformidad generada desde la arquitectura (RF-TST-016 a RF-TST-020).

use std::path::PathBuf;

use lorenz_arch::modelo::Maquina;
use lorenz_tst::conformidad::{ejecutar_conformidad, Opciones};

fn maquina(ejemplo: &str) -> Maquina {
    let ruta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../ejemplos")
        .join(ejemplo)
        .join("maquina.toml");
    lorenz_arch::carga::cargar_archivo(ruta).unwrap()
}

#[test]
fn el_emulador_cumple_su_especificacion_en_el_ejemplo_de_8_bits() {
    let m = maquina("ejemplo-8");
    let i = ejecutar_conformidad(&m, &Opciones::default()).unwrap();
    assert_eq!(i.total_fallos(), 0, "{}", i.a_texto());
    assert_eq!(i.instrucciones.len(), m.instrucciones.len());
    assert!(i.total_casos() > 300, "{}", i.a_texto());
}

#[test]
fn todos_los_ejemplos_del_repositorio_cumplen() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ejemplos");
    let mut vistos = 0;
    for e in std::fs::read_dir(dir).unwrap() {
        let ruta = e.unwrap().path().join("maquina.toml");
        if !ruta.exists() {
            continue;
        }
        let m = lorenz_arch::carga::cargar_archivo(&ruta).unwrap();
        match ejecutar_conformidad(&m, &Opciones::default()) {
            Ok(i) => {
                assert_eq!(i.total_fallos(), 0, "{}: {}", ruta.display(), i.a_texto());
                vistos += 1;
            }
            Err(e) => eprintln!("{}: {e}", ruta.display()),
        }
    }
    assert!(vistos >= 1);
}

fn rica() -> Maquina {
    let ruta = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/maquinas/rica-8.toml");
    lorenz_arch::carga::cargar_archivo(ruta).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn una_maquina_con_resta_pila_y_saltos_relativos_cumple() {
    let m = rica();
    let i = ejecutar_conformidad(&m, &Opciones::default()).unwrap();
    assert_eq!(i.total_fallos(), 0, "{}", i.a_texto());
}

#[test]
fn el_informe_cuenta_casos_por_instruccion() {
    let m = rica();
    let i = ejecutar_conformidad(&m, &Opciones::default()).unwrap();
    println!("{}", i.a_texto());
    let sub = i
        .instrucciones
        .iter()
        .find(|x| x.mnemonico == "SUB")
        .unwrap();
    assert!(sub.casos >= 40 && sub.omitidos == 0);
    let div = i
        .instrucciones
        .iter()
        .find(|x| x.mnemonico == "DIV")
        .unwrap();
    assert!(div.casos > 0);
    let entrada = i
        .instrucciones
        .iter()
        .find(|x| x.mnemonico == "IN")
        .unwrap();
    assert!(entrada.nota.is_some());
}
