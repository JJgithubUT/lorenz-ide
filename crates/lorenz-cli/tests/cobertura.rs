//! `ejecutar --cobertura` (RF-TST-021, RF-TST-022, RF-TST-023).

use std::path::PathBuf;

use lorenz_cli::ejecutar_cli;

fn ejemplo(rel: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../ejemplos/ejemplo-8")
        .join(rel)
        .to_string_lossy()
        .into_owned()
}

fn correr(args: &[&str]) -> (i32, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut s, mut e) = (Vec::new(), Vec::new());
    let c = ejecutar_cli(&args, &mut s, &mut e);
    (c, String::from_utf8(s).unwrap())
}

#[test]
fn sin_la_opcion_no_hay_informe_de_cobertura() {
    let (c, s) = correr(&[
        "ejecutar",
        &ejemplo("maquina.toml"),
        &ejemplo("programas/suma.hex"),
    ]);
    assert_eq!(c, 0);
    assert!(!s.contains("cobertura"));
}

#[test]
fn con_la_opcion_se_informa_de_instrucciones_y_de_las_que_faltan() {
    let (c, s) = correr(&[
        "ejecutar",
        &ejemplo("maquina.toml"),
        &ejemplo("programas/suma.hex"),
        "--cobertura",
    ]);
    assert_eq!(c, 0);
    assert!(s.contains("cobertura de instrucciones:"), "{s}");
    assert!(s.contains("sin ejercitar:"), "{s}");
    assert!(
        !s.contains("HLT"),
        "suma termina con HLT, asi que esta cubierta: {s}"
    );
}

#[test]
fn una_rama_que_solo_se_toma_se_senala() {
    let (_, s) = correr(&[
        "ejecutar",
        &ejemplo("maquina.toml"),
        &ejemplo("programas/bucle.s"),
        "--limite",
        "100",
        "--cobertura",
    ]);
    assert!(s.contains("cobertura de ramas: 1/2"), "{s}");
    assert!(s.contains("JZ: nunca se deja de tomar"), "{s}");
}
