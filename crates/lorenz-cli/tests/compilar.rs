//! La orden `compilar` y la ejecucion de fuentes SGT (RF-SGT-032, RF-SGT-038, RF-SGT-042).

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_cli::ejecutar_cli;

static N: AtomicUsize = AtomicUsize::new(0);

fn ejemplo(nombre: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../ejemplos")
        .join(nombre)
        .join("maquina.toml")
        .to_string_lossy()
        .into_owned()
}

fn carpeta() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-compilar-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
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

const SUMA: &str = "var s: u16;\nfuncion principal() {\n    var i: u16 = 1;\n    mientras i <= 10 {\n        s += i;\n        i += 1;\n    }\n}\n";

fn fuente(texto: &str) -> String {
    let ruta = carpeta().join("p.sgt");
    std::fs::write(&ruta, texto).unwrap();
    ruta.to_string_lossy().into_owned()
}

#[test]
fn compila_a_hexadecimal() {
    let (c, s, e) = correr(&["compilar", &ejemplo("risc-16"), &fuente(SUMA)]);
    assert_eq!((c, e.as_str()), (0, ""), "{e}");
    assert!(s.starts_with("# origen"), "{s}");
}

#[test]
fn escribe_ensamblador_comentado_y_lineas() {
    let dir = carpeta();
    let (asm, lineas, hex) = (dir.join("p.s"), dir.join("p.lineas"), dir.join("p.hex"));
    let f = fuente(SUMA);
    let (c, s, e) = correr(&[
        "compilar",
        &ejemplo("risc-16"),
        &f,
        "-o",
        hex.to_str().unwrap(),
        "--asm",
        asm.to_str().unwrap(),
        "--lineas",
        lineas.to_str().unwrap(),
    ]);
    assert_eq!((c, e.as_str()), (0, ""), "{e}");
    assert!(s.contains("palabras para risc-16"), "{s}");
    let asm = std::fs::read_to_string(asm).unwrap();
    assert!(
        asm.contains("; 5:         s += i;") || asm.contains("; 5: s += i;"),
        "{asm}"
    );
    let lineas = std::fs::read_to_string(lineas).unwrap();
    assert!(
        lineas.lines().count() > 3 && lineas.contains(" 5\n"),
        "{lineas}"
    );
    assert!(hex.exists());
}

#[test]
fn error_con_archivo_linea_y_columna() {
    let f = fuente("funcion principal() {\n    x = 1;\n}\n");
    let (c, _, e) = correr(&["compilar", &ejemplo("risc-16"), &f]);
    assert_eq!(c, 1);
    assert!(e.contains(":2:5: no existe `x`"), "{e}");
}

#[test]
fn error_en_terminos_de_la_maquina() {
    let f = fuente("funcion principal() {\n    var a: u8 = 1;\n}\n");
    let (c, _, e) = correr(&["compilar", &ejemplo("risc-16"), &f]);
    assert_eq!(c, 1);
    assert!(e.contains("`risc-16`") && e.contains("16 bits"), "{e}");
}

#[test]
fn ejecutar_acepta_fuentes_sgt() {
    let f = fuente(SUMA);
    let dir = carpeta().join("v.hex");
    let (c, _, e) = correr(&[
        "ejecutar",
        &ejemplo("risc-16"),
        &f,
        "--volcar",
        &format!("0x4000:1:{}", dir.display()),
    ]);
    assert_eq!((c, e.as_str()), (0, ""), "{e}");
    let v = std::fs::read_to_string(dir).unwrap();
    assert!(v.to_lowercase().contains("0037"), "55 = 0x37: {v}");
}

#[test]
fn falta_el_fuente() {
    let (c, _, e) = correr(&["compilar", &ejemplo("risc-16")]);
    assert_eq!(c, 2);
    assert!(e.contains("fuente"), "{e}");
}
