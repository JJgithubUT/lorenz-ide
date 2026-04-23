//! Las ordenes `kernel`, `depurar-kernel` y `comparar-gpu` (RF-GPU-013 a RF-GPU-016, 018, 019).

use std::io::Cursor;
use std::path::PathBuf;

use lorenz_cli::{
    ejecutar_cli, ejecutar_cli_con_entrada, SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO,
    SALIDA_LIMITE, SALIDA_OK,
};

fn ruta(r: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(r)
        .to_string_lossy()
        .into_owned()
}

fn correr(args: &[&str]) -> (i32, String, String) {
    correr_con(args, "")
}

fn correr_con(args: &[&str], entrada: &str) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut s, mut e) = (Vec::new(), Vec::new());
    let c = ejecutar_cli_con_entrada(
        &args,
        &mut Cursor::new(entrada.as_bytes().to_vec()),
        &mut s,
        &mut e,
    );
    (
        c,
        String::from_utf8(s).unwrap(),
        String::from_utf8(e).unwrap(),
    )
}

fn temporal(nombre: &str, texto: &str) -> String {
    let d = std::env::temp_dir().join(format!("lorenz-kernel-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let r = d.join(nombre);
    let tmp = d.join(format!("{nombre}.{:?}.tmp", std::thread::current().id()));
    std::fs::write(&tmp, texto).unwrap();
    std::fs::rename(&tmp, &r).unwrap();
    r.to_string_lossy().into_owned()
}

const M: &str = "ejemplos/gpu-16/maquina.toml";
fn k(n: &str) -> String {
    ruta(&format!("ejemplos/gpu-16/kernels/{n}.kgpu"))
}

#[test]
fn la_suma_de_vectores_da_el_resultado_y_las_metricas() {
    let (c, s, e) = correr(&["kernel", &ruta(M), &k("suma-vectores"), "--volcar", "32:16"]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(
        s.contains("16 hilos en 2 grupos (warp de 8, 2 unidades)"),
        "{s}"
    );
    assert!(
        s.contains("global[32..48]: 11 22 33 44 55 66 77 88 99 110 121 132 143 154 165 176"),
        "{s}"
    );
    assert!(s.contains("uso de carriles: 100.0%"));
}

#[test]
fn la_reduccion_usa_memoria_compartida_y_barreras() {
    let (c, s, _) = correr(&["kernel", &ruta(M), &k("reduccion"), "--volcar", "8:1"]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("global[8..9]: 31"), "{s}");
    assert!(s.contains("4 barreras"), "{s}");
}

#[test]
fn la_traza_muestra_la_mascara_de_la_divergencia() {
    let (c, s, _) = correr(&[
        "kernel",
        &ruta(M),
        &k("divergencia"),
        "--traza",
        "--volcar",
        "0:8",
    ]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("[.#.#.#.#] divergente mul   r2, r0, 100"), "{s}");
    assert!(s.contains("[#.#.#.#.] divergente li    r2, 7"), "{s}");
    assert!(s.contains("global[0..8]: 7 100 7 300 7 500 7 700"), "{s}");
}

#[test]
fn se_puede_cambiar_el_lanzamiento() {
    let (c, s, _) = correr(&[
        "kernel",
        &ruta(M),
        &k("suma-vectores"),
        "--grupos",
        "4",
        "--hilos",
        "4",
        "--volcar",
        "32:2",
    ]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("16 hilos en 4 grupos"));
}

#[test]
fn la_cpu_y_la_gpu_dan_el_mismo_resultado_y_se_comparan_los_tiempos() {
    let (c, s, e) = correr(&[
        "comparar-gpu",
        &ruta(M),
        &ruta("ejemplos/gpu-16/programas/suma-vectores.s"),
        &k("suma-vectores"),
        "--resultado",
        "0x4020:32:16",
    ]);
    assert_eq!(c, SALIDA_OK, "{e}{s}");
    assert!(s.contains("CPU: 220 ciclos"), "{s}");
    assert!(s.contains("GPU: 63 ciclos"), "{s}");
    assert!(s.contains("la GPU tarda"), "{s}");
    assert!(s.contains("resultados: iguales en 16 palabras"), "{s}");
}

#[test]
fn comparar_avisa_si_los_resultados_difieren() {
    let mal = temporal(
        "mal.kgpu",
        ".global 48\n.lanzar 1 1\n li r0, 1\n st [r0+32], r0\n exit\n",
    );
    let (c, s, _) = correr(&[
        "comparar-gpu",
        &ruta(M),
        &ruta("ejemplos/gpu-16/programas/suma-vectores.s"),
        &mal,
        "--resultado",
        "0x4020:32:4",
    ]);
    assert_eq!(c, SALIDA_ERROR_EJECUCION);
    assert!(s.contains("palabras distintas"), "{s}");
    assert!(s.contains("posicion 0: la CPU da 11 y la GPU 0"), "{s}");
}

#[test]
fn el_depurador_avanza_hilo_por_hilo() {
    let (c, s, e) = correr_con(
        &["depurar-kernel", &ruta(M), &k("divergencia")],
        "paso 4\nhilo 0.0.3\ngrupos\nmemoria 0 2\ncontinuar\nmemoria 0 4\ninfo\nfin\n",
    );
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(
        s.contains("hilo 0.0.3: pc=4, vivo"),
        "el hilo impar ya multiplico: {s}"
    );
    assert!(s.contains("r2  = 300"), "{s}");
    assert!(s.contains("unidad 0: grupo 0 (1 warps)"));
    assert!(s.contains("el kernel termino"));
    assert!(s.contains("global[0..4]: [7, 100, 7, 300]"), "{s}");
}

#[test]
fn el_depurador_acepta_un_guion() {
    let g = temporal("sesion.txt", "# empieza\npaso\nhilo 9.9.9\nayuda\nfin\n");
    let (c, s, _) = correr(&["depurar-kernel", &ruta(M), &k("divergencia"), "--guion", &g]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("ese hilo no esta en curso"), "{s}");
    assert!(s.contains("continuar"));
}

#[test]
fn errores_de_uso_y_de_ejecucion() {
    let sin_lanzar = temporal("sin.kgpu", "exit\n");
    let (c, _, e) = correr(&["kernel", &ruta(M), &sin_lanzar]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("cuantos hilos lanzar"), "{e}");
    let (c, _, e) = correr(&[
        "kernel",
        &ruta("ejemplos/risc-16/maquina.toml"),
        &k("reduccion"),
    ]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("falta [gpu]"));
    let roto = temporal("roto.kgpu", ".lanzar 1 1\nli r0\n");
    let (c, _, e) = correr(&["kernel", &ruta(M), &roto]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("linea 2: `li` lleva 2 operandos"), "{e}");
    let fuera = temporal(
        "fuera.kgpu",
        ".global 4\n.lanzar 1 1\n li r0, 9\n ld r1, [r0+0]\n exit\n",
    );
    let (c, _, e) = correr(&["kernel", &ruta(M), &fuera]);
    assert_eq!(c, SALIDA_ERROR_EJECUCION);
    assert!(e.contains("fuera de la memoria global"), "{e}");
    let eterno = temporal("eterno.kgpu", ".lanzar 1 1\nb: bra b\n");
    let (c, _, e) = correr(&["kernel", &ruta(M), &eterno, "--limite", "100"]);
    assert_eq!(c, SALIDA_LIMITE);
    assert!(e.contains("100 instrucciones de warp"));
    let (c, _, e) = correr(&[
        "comparar-gpu",
        &ruta(M),
        "a.s",
        &k("reduccion"),
        "--resultado",
        "1:2",
    ]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("DIR_CPU:DIR_GPU:CANTIDAD"), "{e}");
    let (c, _, _) = ejecutar_cli_vacio(&["kernel", &ruta(M)]);
    assert_eq!(c, SALIDA_ERROR_USO);
}

fn ejecutar_cli_vacio(args: &[&str]) -> (i32, Vec<u8>, Vec<u8>) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut s, mut e) = (Vec::new(), Vec::new());
    let c = ejecutar_cli(&args, &mut s, &mut e);
    (c, s, e)
}
