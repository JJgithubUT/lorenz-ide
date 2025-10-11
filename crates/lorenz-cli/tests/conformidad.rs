//! La orden `conformidad` (RF-TST-016 a RF-TST-020).

use std::path::PathBuf;

use lorenz_cli::ejecutar_cli;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn maquina() -> String {
    raiz()
        .join("ejemplos/ejemplo-8/maquina.toml")
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
fn el_ejemplo_cumple_y_el_informe_lista_cada_instruccion() {
    let (codigo, salida, errores) = correr(&["conformidad", &maquina()]);
    assert_eq!(codigo, 0, "{errores}\n{salida}");
    assert!(salida.contains("conformidad de ejemplo-8"));
    assert!(salida.contains("ADD"));
    assert!(salida.ends_with("0 fallos\n"));
}

#[test]
fn se_puede_probar_una_sola_instruccion() {
    let (codigo, salida, _) = correr(&["conformidad", &maquina(), "--instruccion", "add"]);
    assert_eq!(codigo, 0);
    assert!(salida.contains("ADD"));
    assert!(!salida.contains("MOV"));
}

#[test]
fn una_instruccion_inexistente_es_un_error_de_uso() {
    let (codigo, _, errores) = correr(&["conformidad", &maquina(), "--instruccion", "VOLAR"]);
    assert_eq!(codigo, 2);
    assert!(errores.contains("VOLAR"));
}

#[test]
fn el_informe_json_se_puede_leer_de_nuevo() {
    let (codigo, salida, _) = correr(&["conformidad", &maquina(), "--json", "--casos", "4"]);
    assert_eq!(codigo, 0);
    let i = lorenz_tst::conformidad::InformeConformidad::desde_json(&salida).unwrap();
    assert_eq!(i.maquina, "ejemplo-8");
    assert_eq!(i.total_fallos(), 0);
}

#[test]
fn una_maquina_invalida_es_un_error_de_uso() {
    let (codigo, _, errores) = correr(&["conformidad", "no-existe.toml"]);
    assert_eq!(codigo, 2);
    assert!(errores.contains("no-existe.toml"));
}

#[test]
fn falta_la_maquina() {
    let (codigo, _, errores) = correr(&["conformidad"]);
    assert_eq!(codigo, 2);
    assert!(errores.contains("maquina"));
}

#[test]
fn la_ayuda_documenta_la_orden() {
    let (_, salida, _) = correr(&["ayuda"]);
    assert!(salida.contains("lorenz conformidad"));
}
