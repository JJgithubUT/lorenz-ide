//! La orden `depurar` (RF-DEP-001, RF-DEP-018).

use std::io::Cursor;
use std::path::PathBuf;

use lorenz_cli::{ejecutar_cli_con_entrada, SALIDA_ERROR_USO, SALIDA_OK};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn maquina() -> String {
    raiz()
        .join("ejemplos/ejemplo-8/maquina.toml")
        .to_string_lossy()
        .into_owned()
}

fn suma() -> String {
    raiz()
        .join("ejemplos/ejemplo-8/programas/suma.s")
        .to_string_lossy()
        .into_owned()
}

fn temporal(nombre: &str, contenido: &str) -> String {
    let dir = std::env::temp_dir().join(format!("lorenz-dep-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ruta = dir.join(nombre);
    std::fs::write(&ruta, contenido).unwrap();
    ruta.to_string_lossy().into_owned()
}

fn correr(args: &[&str], entrada: &str) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut s, mut e) = (Vec::new(), Vec::new());
    let c = ejecutar_cli_con_entrada(&args, &mut Cursor::new(entrada.as_bytes()), &mut s, &mut e);
    (
        c,
        String::from_utf8(s).unwrap(),
        String::from_utf8(e).unwrap(),
    )
}

#[test]
fn sesion_interactiva_por_la_entrada() {
    let (c, salida, _) = correr(
        &["depurar", &maquina(), &suma()],
        "paso 2\nregistros\nfin\n",
    );
    assert_eq!(c, SALIDA_OK);
    assert!(salida.contains("(lorenz) "), "{salida}");
    assert!(salida.contains("2 instruccion"), "{salida}");
    assert!(salida.contains("A = c8"), "{salida}");
}

#[test]
fn la_entrada_que_se_acaba_termina_bien() {
    let (c, _, _) = correr(&["depurar", &maquina(), &suma()], "paso\n");
    assert_eq!(c, SALIDA_OK);
}

#[test]
fn guion_con_comentarios() {
    let g = temporal("g1.dbg", "# corre todo\ncontinuar\nregistros\nfin\n");
    let (c, salida, _) = correr(&["depurar", &maquina(), &suma(), "--guion", &g], "");
    assert_eq!(c, SALIDA_OK);
    assert!(salida.contains("continuar\n"), "{salida}");
    assert!(salida.contains("la maquina se detuvo"), "{salida}");
    assert!(salida.contains("A = 2c"), "{salida}");
}

#[test]
fn guion_inexistente() {
    let (c, _, err) = correr(
        &["depurar", &maquina(), &suma(), "--guion", "/no/existe.dbg"],
        "",
    );
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(err.contains("no se pudo leer"), "{err}");
}

#[test]
fn fuente_con_errores() {
    let f = temporal("mal.s", "BAILAR A\n");
    let (c, _, err) = correr(&["depurar", &maquina(), &f], "");
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(err.contains("mal.s"), "{err}");
}

#[test]
fn faltan_argumentos() {
    let (c, _, err) = correr(&["depurar", &maquina()], "");
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(err.contains("programa"), "{err}");
}

#[test]
fn programa_hexadecimal() {
    let h = temporal("p.hex", "00\n");
    let (c, salida, _) = correr(&["depurar", &maquina(), &h], "desensamblar 0 1\nfin\n");
    assert_eq!(c, SALIDA_OK);
    assert!(salida.contains("NOP"), "{salida}");
}
