//! La orden `aleatorio` (RF-TST-024, RF-TST-025, RF-TST-035).

use std::path::PathBuf;

use lorenz_cli::ejecutar_cli;

fn maquina() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../ejemplos/ejemplo-8/maquina.toml")
        .to_string_lossy()
        .into_owned()
}

fn correr(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut salida, mut errores) = (Vec::new(), Vec::new());
    let codigo = ejecutar_cli(&args, &mut salida, &mut errores);
    (
        codigo,
        String::from_utf8(salida).unwrap(),
        String::from_utf8(errores).unwrap(),
    )
}

#[test]
fn el_emulador_y_la_referencia_no_divergen() {
    let (codigo, salida, errores) = correr(&["aleatorio", &maquina(), "--programas", "40"]);
    assert_eq!(codigo, 0, "{errores}\n{salida}");
    assert!(salida.contains("40 programas"));
    assert!(salida.contains("sin divergencias"));
}

#[test]
fn la_semilla_aparece_en_el_informe_y_lo_hace_reproducible() {
    let a = correr(&[
        "aleatorio",
        &maquina(),
        "--semilla",
        "99",
        "--programas",
        "10",
        "--json",
    ]);
    let b = correr(&[
        "aleatorio",
        &maquina(),
        "--semilla",
        "99",
        "--programas",
        "10",
        "--json",
    ]);
    assert_eq!(a.1, b.1);
    let v: serde_json::Value = serde_json::from_str(&a.1).unwrap();
    assert_eq!(v["semilla"], 99);
    assert_eq!(v["programas"], 10);
}

#[test]
fn el_informe_se_puede_guardar_en_un_archivo() {
    let ruta = std::env::temp_dir().join(format!("lorenz-aleatorio-{}.json", std::process::id()));
    let ruta = ruta.to_string_lossy().into_owned();
    let (codigo, _, errores) = correr(&[
        "aleatorio",
        &maquina(),
        "--programas",
        "3",
        "--salida",
        &ruta,
    ]);
    assert_eq!(codigo, 0, "{errores}");
    let i = lorenz_tst::aleatorio::InformeAleatorio::desde_json(
        &std::fs::read_to_string(&ruta).unwrap(),
    )
    .unwrap();
    assert_eq!(i.programas, 3);
}

#[test]
fn una_maquina_invalida_o_un_numero_malo_son_errores_de_uso() {
    assert_eq!(correr(&["aleatorio", "no-existe.toml"]).0, 2);
    assert_eq!(correr(&["aleatorio"]).0, 2);
    let (codigo, _, errores) = correr(&["aleatorio", &maquina(), "--semilla", "uno"]);
    assert_eq!(codigo, 2);
    assert!(errores.contains("--semilla"));
}

#[test]
fn la_ayuda_documenta_la_orden() {
    let (_, salida, _) = correr(&["ayuda"]);
    assert!(salida.contains("lorenz aleatorio"));
    assert!(salida.contains("--semilla"));
}
