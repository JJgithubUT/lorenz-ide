//! La orden `nuevo` (RF-ECO-042): un proyecto nace de una carpeta plantilla.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_cli::ejecutar_cli;

static N: AtomicUsize = AtomicUsize::new(0);

fn plantilla() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../ejemplos/so-ejemplo")
        .to_string_lossy()
        .into_owned()
}

fn temporal() -> PathBuf {
    std::env::temp_dir().join(format!(
        "lorenz-nuevo-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ))
}

fn correr(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut s, mut e) = (Vec::new(), Vec::new());
    let c = ejecutar_cli(&args, &mut s, &mut e);
    (
        c,
        String::from_utf8(s).unwrap(),
        String::from_utf8(e).unwrap(),
    )
}

#[test]
fn copia_la_plantilla_y_renombra_la_maquina() {
    let d = temporal();
    let destino = d.to_string_lossy().into_owned();
    let (c, s, e) = correr(&["nuevo", &plantilla(), &destino, "--nombre", "mi-so"]);
    assert_eq!(c, 0, "{e}");
    assert!(s.contains("proyecto nuevo"), "{s}");
    let m = std::fs::read_to_string(d.join("maquina.toml")).unwrap();
    assert!(m.contains("nombre         = \"mi-so\""), "{m}");
    assert!(!m.contains("lorenz-os\""));
    assert!(d.join("so.sgt").is_file());
    let (c, _, e) = correr(&[
        "compilar",
        &d.join("maquina.toml").to_string_lossy(),
        &d.join("so.sgt").to_string_lossy(),
    ]);
    assert_eq!(c, 0, "{e}");
}

#[test]
fn no_pisa_una_carpeta_con_contenido() {
    let d = temporal();
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("a.txt"), "x").unwrap();
    let (c, _, e) = correr(&["nuevo", &plantilla(), &d.to_string_lossy()]);
    assert_ne!(c, 0);
    assert!(e.contains("ya existe"), "{e}");
    assert_eq!(std::fs::read_to_string(d.join("a.txt")).unwrap(), "x");
}

#[test]
fn una_carpeta_sin_maquina_no_es_plantilla() {
    let d = temporal();
    std::fs::create_dir_all(&d).unwrap();
    let (c, _, e) = correr(&["nuevo", &d.to_string_lossy(), &temporal().to_string_lossy()]);
    assert_ne!(c, 0);
    assert!(e.contains("no es una plantilla"), "{e}");
}

#[test]
fn faltan_argumentos() {
    let (c, _, _) = correr(&["nuevo", "solo-una"]);
    assert_ne!(c, 0);
}
