//! Lorenz funciona por completo sin ninguna funcion de inteligencia artificial (RF-ECO-043).
//!
//! Ninguna orden puede necesitar un servicio externo, asi que el programa no debe depender de un
//! cliente de red ni de un motor de modelos. Se comprueba sobre el archivo de bloqueo.

use std::path::Path;

const PROHIBIDAS: &[&str] = &[
    "reqwest",
    "hyper",
    "ureq",
    "curl",
    "isahc",
    "surf",
    "tokio",
    "async-std",
    "openssl",
    "native-tls",
    "rustls",
    "anthropic",
    "openai",
    "async-openai",
    "llm",
    "candle-core",
    "tch",
    "ort",
    "burn",
    "tract-core",
    "rig-core",
    "langchain-rust",
];

fn dependencias() -> Vec<String> {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    std::fs::read_to_string(raiz)
        .expect("Cargo.lock debe estar en la raiz del repositorio")
        .lines()
        .filter_map(|l| l.strip_prefix("name = \""))
        .map(|l| l.trim_end_matches('"').to_string())
        .collect()
}

#[test]
fn no_hay_clientes_de_red_ni_motores_de_ia_entre_las_dependencias() {
    let encontradas: Vec<String> = dependencias()
        .into_iter()
        .filter(|d| PROHIBIDAS.contains(&d.as_str()))
        .collect();
    assert!(
        encontradas.is_empty(),
        "dependencias prohibidas: {encontradas:?}"
    );
}

#[test]
fn el_archivo_de_bloqueo_se_lee_y_trae_las_dependencias_conocidas() {
    let d = dependencias();
    assert!(d.iter().any(|x| x == "toml"));
    assert!(d.iter().any(|x| x == "lorenz-cli"));
}
