//! La orden `compatibilidad` (RF-TST-029).

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

fn programa(nombre: &str) -> String {
    raiz()
        .join("ejemplos/ejemplo-8/programas")
        .join(nombre)
        .to_string_lossy()
        .into_owned()
}

fn nueva_version(nombre: &str, de: &str, a: &str) -> String {
    let t = std::fs::read_to_string(maquina()).unwrap();
    assert!(t.contains(de));
    let dir = std::env::temp_dir().join(format!("lorenz-compat-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ruta = dir.join(nombre);
    std::fs::write(&ruta, t.replacen(de, a, 1)).unwrap();
    ruta.to_string_lossy().into_owned()
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
fn la_misma_arquitectura_es_compatible_con_un_fuente_y_con_un_hexadecimal() {
    for p in ["suma.s", "suma.hex"] {
        let (c, salida, errores) =
            correr(&["compatibilidad", &maquina(), &maquina(), &programa(p)]);
        assert_eq!(c, 0, "{errores}\n{salida}");
        assert!(salida.contains("0 incompatibles"), "{salida}");
    }
}

#[test]
fn una_isa_que_cambia_un_codigo_rompe_los_binarios_que_lo_usan() {
    let nueva = nueva_version(
        "nueva.toml",
        "codigo    = \"1111 ....\"",
        "codigo    = \"1110 ....\"",
    );
    let (c, salida, _) = correr(&["compatibilidad", &maquina(), &nueva, &programa("suma.hex")]);
    assert_eq!(c, 1, "{salida}");
    assert!(
        salida.contains("`HLT`: la arquitectura nueva no la reconoce"),
        "{salida}"
    );
}

#[test]
fn sale_en_json() {
    let nueva = nueva_version(
        "nueva-json.toml",
        "codigo    = \"1111 ....\"",
        "codigo    = \"1110 ....\"",
    );
    let (c, salida, _) = correr(&[
        "compatibilidad",
        &maquina(),
        &nueva,
        &programa("suma.hex"),
        "--json",
    ]);
    assert_eq!(c, 1);
    let v: serde_json::Value = serde_json::from_str(&salida).unwrap();
    assert_eq!(v["incompatibles"].as_array().unwrap().len(), 1);
}

#[test]
fn los_errores_de_uso_se_distinguen() {
    assert_eq!(correr(&["compatibilidad"]).0, 2);
    assert_eq!(correr(&["compatibilidad", &maquina()]).0, 2);
    assert_eq!(correr(&["compatibilidad", &maquina(), &maquina()]).0, 2);
    let (c, _, errores) = correr(&[
        "compatibilidad",
        "no.toml",
        &maquina(),
        &programa("suma.hex"),
    ]);
    assert_eq!(c, 2);
    assert!(errores.contains("no.toml"));
    let (c, _, errores) = correr(&["compatibilidad", &maquina(), &maquina(), "no-existe.hex"]);
    assert_eq!(c, 2);
    assert!(errores.contains("no-existe.hex"));
}

#[test]
fn la_ayuda_documenta_la_orden() {
    assert!(correr(&["ayuda"]).1.contains("lorenz compatibilidad"));
}
