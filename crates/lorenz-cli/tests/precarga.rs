//! Las regiones con `contenido` se llenan antes de ejecutar (RF-ARQ-029).

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_cli::ejecutar_cli;

static N: AtomicUsize = AtomicUsize::new(0);

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

fn proyecto(contenido: Option<&str>) -> (PathBuf, String) {
    let d = std::env::temp_dir().join(format!(
        "lorenz-cli-precarga-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d).unwrap();
    let t = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ejemplos/ejemplo-8/maquina.toml"),
    )
    .unwrap();
    let t = match contenido {
        Some(c) => t.replacen(
            "nombre = \"rom\"",
            &format!("nombre = \"rom\"\ncontenido = \"{c}\""),
            1,
        ),
        None => t,
    };
    let m = d.join("maquina.toml");
    std::fs::write(&m, t).unwrap();
    std::fs::write(d.join("rom.hex"), "AA BB CC\n").unwrap();
    std::fs::write(d.join("hlt.hex"), "F0\n").unwrap();
    (d.clone(), m.to_string_lossy().into_owned())
}

#[test]
fn ejecutar_ve_el_contenido_de_la_rom() {
    let (d, m) = proyecto(Some("rom.hex"));
    let h = d.join("hlt.hex").to_string_lossy().into_owned();
    let (c, s, e) = correr(&[
        "ejecutar",
        &m,
        &h,
        "--direccion",
        "0x100",
        "--volcar",
        "0:3",
    ]);
    assert_eq!(c, 0, "{e}");
    assert!(s.contains("AA") || s.contains("aa"), "{s}");
}

#[test]
fn el_programa_del_usuario_puede_sobrescribir_el_contenido() {
    let (d, m) = proyecto(Some("rom.hex"));
    let h = d.join("hlt.hex").to_string_lossy().into_owned();
    let (c, s, _) = correr(&["ejecutar", &m, &h, "--volcar", "0:3"]);
    assert_eq!(c, 0);
    assert!(s.contains("F0") || s.contains("f0"), "{s}");
}

#[test]
fn un_contenido_inexistente_es_un_error_de_uso() {
    let (d, m) = proyecto(Some("falta.hex"));
    let h = d.join("hlt.hex").to_string_lossy().into_owned();
    let (c, _, e) = correr(&["ejecutar", &m, &h]);
    assert_eq!(c, 2);
    assert!(e.contains("falta.hex"), "{e}");
}

#[test]
fn sin_contenido_la_rom_queda_en_ceros() {
    let (d, m) = proyecto(None);
    let h = d.join("hlt.hex").to_string_lossy().into_owned();
    let (c, s, _) = correr(&[
        "ejecutar",
        &m,
        &h,
        "--direccion",
        "0x100",
        "--volcar",
        "0:3",
    ]);
    assert_eq!(c, 0);
    assert!(!s.to_lowercase().contains("aa"), "{s}");
}
