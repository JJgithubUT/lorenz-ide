//! La orden `sugerir` (RF-CIC-034).

use std::path::PathBuf;

use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_USO, SALIDA_OK};

fn ruta(r: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(r)
        .to_string_lossy()
        .into_owned()
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

fn maquina() -> String {
    ruta("ejemplos/ejemplo-8-ciclos/maquina.toml")
}

fn temporal(nombre: &str, texto: &str) -> String {
    let d = std::env::temp_dir().join(format!("lorenz-sugerir-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let r = d.join(nombre);
    std::fs::write(&r, texto).unwrap();
    r.to_string_lossy().into_owned()
}

#[test]
fn un_bucle_sugiere_fusionar_su_cuerpo() {
    let (c, s, e) = correr(&[
        "sugerir",
        &maquina(),
        &ruta("ejemplos/ejemplo-8-ciclos/programas/bucle.s"),
        "--limite",
        "200",
    ]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("1 programa analizado, 200 instrucciones"), "{s}");
    assert!(s.contains("ADD+JZ"), "{s}");
    assert!(s.contains("estimacion"), "{s}");
}

#[test]
fn varios_programas_se_analizan_juntos() {
    let a = temporal("a.s", "LDI A, 1\nLDI B, 2\nADD A, B\nHLT\n");
    let b = temporal("b.s", "LDI A, 3\nLDI B, 4\nADD A, B\nHLT\n");
    let (c, s, e) = correr(&["sugerir", &maquina(), &a, &b]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("2 programas analizados"), "{s}");
    assert!(s.contains("LDI+ADD") || s.contains("LDI+LDI"), "{s}");
}

#[test]
fn un_programa_sin_secuencias_utiles_lo_dice() {
    let p = temporal("hlt.s", "HLT\n");
    let (c, s, _) = correr(&["sugerir", &maquina(), &p]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("no hay secuencias"), "{s}");
}

#[test]
fn los_argumentos_incorrectos_son_error_de_uso() {
    let p = temporal("x.s", "HLT\n");
    assert_eq!(correr(&["sugerir", &maquina()]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["sugerir"]).0, SALIDA_ERROR_USO);
    assert_eq!(
        correr(&["sugerir", &maquina(), &p, "--minima", "1"]).0,
        SALIDA_ERROR_USO
    );
    assert_eq!(
        correr(&["sugerir", &maquina(), &p, "--rayo"]).0,
        SALIDA_ERROR_USO
    );
    assert_eq!(
        correr(&["sugerir", "no-existe.toml", &p]).0,
        SALIDA_ERROR_USO
    );
    assert_eq!(
        correr(&["sugerir", &maquina(), "no-existe.s"]).0,
        SALIDA_ERROR_USO
    );
}
