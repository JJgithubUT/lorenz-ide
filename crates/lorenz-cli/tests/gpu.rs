//! La orden `gpu` (RF-GPU-004, RF-GPU-017, RF-GPU-022).

use std::path::PathBuf;

use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_OK};

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

fn temporal(nombre: &str, texto: &str) -> String {
    let d = std::env::temp_dir().join(format!("lorenz-gpu-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let r = d.join(nombre);
    // Escritura atomica: otra prueba puede estar leyendo este mismo archivo ahora.
    let tmp = d.join(format!("{nombre}.{:?}.tmp", std::thread::current().id()));
    std::fs::write(&tmp, texto).unwrap();
    std::fs::rename(&tmp, &r).unwrap();
    r.to_string_lossy().into_owned()
}

const M: &str = "ejemplos/gpu-16/maquina.toml";

#[test]
fn un_guion_dibuja_y_da_metricas() {
    let (c, s, e) = correr(&[
        "gpu",
        &ruta(M),
        &ruta("ejemplos/gpu-16/programas/figuras.gpu"),
    ]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("ciclos de la GPU:"));
    assert!(s.contains("comandos: 12 (1 triangulos, 1 lotes)"));
    assert!(!s.contains("ciclos de la CPU"), "un guion no tiene CPU");
}

#[test]
fn un_programa_de_la_cpu_dibuja_por_el_puerto() {
    let (c, s, e) = correr(&[
        "gpu",
        &ruta(M),
        &ruta("ejemplos/gpu-16/programas/triangulo.s"),
        "--pixel",
        "3,3",
        "--pixel",
        "30,20",
    ]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("ciclos de la CPU: 24"));
    assert!(s.contains("(3, 3): #0000FF"), "{s}");
    assert!(s.contains("(30, 20): #000000"), "{s}");
}

#[test]
fn vuelca_png_y_ppm() {
    let png = temporal("f.png", "");
    let (c, s, _) = correr(&[
        "gpu",
        &ruta(M),
        &ruta("ejemplos/gpu-16/programas/figuras.gpu"),
        "--imagen",
        &png,
        "--ampliar",
        "2",
    ]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("(64x48)"));
    let b = std::fs::read(&png).unwrap();
    assert_eq!(&b[1..4], b"PNG");
    let ppm = temporal("f.ppm", "");
    correr(&[
        "gpu",
        &ruta(M),
        &ruta("ejemplos/gpu-16/programas/figuras.gpu"),
        "--imagen",
        &ppm,
    ]);
    assert!(std::fs::read(&ppm)
        .unwrap()
        .starts_with(b"P6\n32 24\n255\n"));
}

#[test]
fn el_texto_dibuja_la_superficie() {
    let (_, s, _) = correr(&[
        "gpu",
        &ruta(M),
        &ruta("ejemplos/gpu-16/programas/figuras.gpu"),
        "--texto",
    ]);
    assert!(s.lines().filter(|l| l.len() == 32).count() >= 24);
}

#[test]
fn sin_seccion_gpu_es_un_error_de_uso() {
    let (c, _, e) = correr(&[
        "gpu",
        &ruta("ejemplos/risc-16/maquina.toml"),
        &ruta("ejemplos/gpu-16/programas/figuras.gpu"),
    ]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("falta [gpu]"), "{e}");
}

#[test]
fn un_guion_con_error_senala_la_linea() {
    let g = temporal("malo.gpu", "limpiar\ndibujar 1\n");
    let (c, _, e) = correr(&["gpu", &ruta(M), &g]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("linea 2: la orden `dibujar` no existe"), "{e}");
}

#[test]
fn el_limite_de_instrucciones_se_respeta() {
    let p = temporal("bucle.s", "bucle: J bucle\n");
    let (c, _, e) = correr(&["gpu", &ruta(M), &p, "--limite", "50"]);
    assert_eq!(c, SALIDA_LIMITE);
    assert!(e.contains("limite de 50"));
}

#[test]
fn opciones_invalidas() {
    let (c, _, e) = correr(&["gpu", &ruta(M), "x.gpu", "--pixel", "3"]);
    assert_ne!(c, SALIDA_OK);
    assert!(e.contains("X,Y"), "{e}");
    let (c, _, _) = correr(&["gpu", &ruta(M)]);
    assert_ne!(c, SALIDA_OK);
}
