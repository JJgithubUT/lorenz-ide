//! La orden `enlazar` (RF-ENS-019, RF-ENS-020).

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_cli::ejecutar_cli;

static N: AtomicUsize = AtomicUsize::new(0);

fn maquina() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../ejemplos/ejemplo-8/maquina.toml")
        .to_string_lossy()
        .into_owned()
}

fn carpeta() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-enlazar-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn escribir(dir: &std::path::Path, nombre: &str, texto: &str) -> String {
    let r = dir.join(nombre);
    std::fs::write(&r, texto).unwrap();
    r.to_string_lossy().into_owned()
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
fn ensamblar_enlazar_y_ejecutar_dos_archivos() {
    let d = carpeta();
    let a = escribir(&d, "a.s", ".externo valor\n.org 0x10\nLDI A, valor\nHLT\n");
    let b = escribir(&d, "b.s", "valor = 42\n");
    let m = maquina();
    let oa = d.join("a.obj.toml").to_string_lossy().into_owned();
    let (c, s, _) = correr(&["ensamblar", &m, &a, "--objeto", &oa]);
    assert_eq!(c, 0);
    assert!(s.contains("por enlazar"), "{s}");
    let hex = d.join("todo.hex").to_string_lossy().into_owned();
    let (c, s, e) = correr(&["enlazar", &m, &oa, &b, "-o", &hex]);
    assert_eq!(c, 0, "{e}");
    assert!(s.contains("2 objeto(s) enlazados"), "{s}");
    let (c, s, _) = correr(&["ejecutar", &m, &hex, "--direccion", "0x10"]);
    assert_eq!(c, 0);
    assert!(
        s.contains("A") && s.contains("0x2a") || s.contains("42"),
        "{s}"
    );
}

#[test]
fn los_fuentes_se_pueden_enlazar_directamente() {
    let d = carpeta();
    let a = escribir(&d, "a.s", ".externo f\n.org 0x10\nLDI A, f\nHLT\n");
    let b = escribir(&d, "b.s", "f = 7\n");
    let (c, s, e) = correr(&["enlazar", &maquina(), &a, &b]);
    assert_eq!(c, 0, "{e}");
    assert!(s.contains("0x"), "{s}");
}

#[test]
fn un_simbolo_sin_resolver_sale_con_codigo_1_y_su_lugar() {
    let d = carpeta();
    let a = escribir(&d, "a.s", ".externo falta\nLDI A, falta\n");
    let (c, _, e) = correr(&["enlazar", &maquina(), &a]);
    assert_eq!(c, 1);
    assert!(e.contains("falta") && e.contains("a.s:2"), "{e}");
}

#[test]
fn ensamblar_sin_objeto_un_programa_con_externos_explica_que_hacer() {
    let d = carpeta();
    let a = escribir(&d, "a.s", ".externo f\nLDI A, f\n");
    let (c, _, e) = correr(&["ensamblar", &maquina(), &a]);
    assert_eq!(c, 2);
    assert!(
        e.contains("lorenz enlazar") && e.contains("--objeto"),
        "{e}"
    );
}

#[test]
fn ejecutar_un_fuente_con_externos_pide_enlazarlo() {
    let d = carpeta();
    let a = escribir(&d, "a.s", ".externo f\nLDI A, f\nHLT\n");
    let (c, _, e) = correr(&["ejecutar", &maquina(), &a]);
    assert_ne!(c, 0);
    assert!(e.contains("enlazar"), "{e}");
}

#[test]
fn sin_objetos_es_un_error_de_uso() {
    let (c, _, e) = correr(&["enlazar", &maquina()]);
    assert_eq!(c, 2);
    assert!(e.contains("objeto"), "{e}");
}

#[test]
fn un_objeto_de_otra_maquina_se_rechaza() {
    let d = carpeta();
    let o = escribir(
        &d,
        "x.toml",
        "formato = \"lorenz-objeto-0.1\"\nmaquina = \"otra\"\n",
    );
    let (c, _, e) = correr(&["enlazar", &maquina(), &o]);
    assert_eq!(c, 2);
    assert!(e.contains("otra"), "{e}");
}
