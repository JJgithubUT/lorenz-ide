//! Ejercicios guiados y desafios (RF-ECO-014, RF-ECO-016).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_cli::desafios::SALIDA_NO_RESUELVE;
use lorenz_cli::ejercicios::SALIDA_NO_RESUELTO;
use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_USO, SALIDA_OK};

static N: AtomicUsize = AtomicUsize::new(0);

fn temporal() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-ejer-{}-{}",
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

const RAIZ: &str = "../../ejercicios";

fn ejercicios() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(RAIZ)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.join("ejercicio.toml").is_file())
        .collect();
    v.sort();
    v
}

#[test]
fn hay_doce_ejercicios_y_todos_se_listan() {
    assert_eq!(ejercicios().len(), 12);
    let (c, s, _) = correr(&["ejercicio", "listar", RAIZ]);
    assert_eq!(c, SALIDA_OK);
    assert_eq!(s.lines().count(), 12);
    assert!(s.contains("fibonacci"));
}

#[test]
fn cada_referencia_resuelve_su_ejercicio() {
    for d in ejercicios() {
        let r = d.join("referencia.s");
        let (c, s, e) = correr(&[
            "ejercicio",
            "verificar",
            d.to_str().unwrap(),
            "--solucion",
            r.to_str().unwrap(),
        ]);
        assert_eq!(c, SALIDA_OK, "{}: {s}{e}", d.display());
        assert!(s.contains("Resuelto"), "{}: {s}", d.display());
    }
}

#[test]
fn cada_esqueleto_todavia_no_resuelve() {
    for d in ejercicios() {
        let (c, s, _) = correr(&["ejercicio", "verificar", d.to_str().unwrap()]);
        assert_eq!(c, SALIDA_NO_RESUELTO, "{}: {s}", d.display());
        assert!(s.contains("Todavia no"));
    }
}

#[test]
fn ver_muestra_enunciado_y_pistas_de_a_una() {
    let d = format!("{RAIZ}/3-maximo");
    let (c, s, _) = correr(&["ejercicio", "ver", &d]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("El mayor de dos"));
    let (c, s, _) = correr(&["ejercicio", "ver", &d, "--pista", "1"]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("SLT"));
    let (c, _, e) = correr(&["ejercicio", "ver", &d, "--pista", "9"]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(!e.is_empty());
}

#[test]
fn los_errores_de_uso_se_avisan() {
    assert_eq!(correr(&["ejercicio"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["ejercicio", "volar", "x"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["ejercicio", "ver"]).0, SALIDA_ERROR_USO);
    assert_eq!(
        correr(&["ejercicio", "ver", "no-existe"]).0,
        SALIDA_ERROR_USO
    );
}

#[test]
fn un_ejercicio_con_datos_invalidos_se_rechaza() {
    let t = temporal();
    for (i, texto) in [
        "[ejercicio]\nid = \"a b\"\ntitulo = \"t\"\nnivel = 1\nenunciado = \"e\"\n",
        "[ejercicio]\nid = \"a\"\ntitulo = \"t\"\nnivel = 9\nenunciado = \"e\"\n",
        "[ejercicio]\nid = \"a\"\ntitulo = \"\"\nnivel = 1\nenunciado = \"e\"\n",
        "[ejercicio]\nid = \"a\"\ntitulo = \"t\"\nnivel = 1\nenunciado = \"e\"\nsobra = 1\n",
    ]
    .iter()
    .enumerate()
    {
        let d = t.join(format!("e{i}"));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("ejercicio.toml"), texto).unwrap();
        let (c, _, e) = correr(&["ejercicio", "ver", d.to_str().unwrap()]);
        assert_eq!(c, SALIDA_ERROR_USO, "{texto}");
        assert!(!e.is_empty());
    }
}

const DESAFIO: &str = "../../desafios/multiplicar-barato";

#[test]
fn el_desafio_ordena_por_la_metrica() {
    let a = format!("{DESAFIO}/ingenua.s");
    let b = format!("{DESAFIO}/ajustada.s");
    let (c, s, _) = correr(&["desafio", "comparar", DESAFIO, &a, &b]);
    assert_eq!(c, SALIDA_OK);
    let l: Vec<&str> = s.lines().collect();
    assert!(
        l[2].contains("ajustada.s") && l[2].trim_start().starts_with('1'),
        "{s}"
    );
    assert!(
        l[3].contains("ingenua.s") && l[3].trim_start().starts_with('2'),
        "{s}"
    );
}

#[test]
fn dos_soluciones_iguales_comparten_puesto() {
    let a = format!("{DESAFIO}/ingenua.s");
    let t = temporal();
    let copia = t.join("copia.s");
    std::fs::copy(&a, &copia).unwrap();
    let (c, s, _) = correr(&["desafio", "comparar", DESAFIO, &a, copia.to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK);
    let puestos: Vec<&str> = s
        .lines()
        .skip(2)
        .map(|l| l.split_whitespace().next().unwrap())
        .collect();
    assert_eq!(
        puestos[0].trim_end_matches('*'),
        puestos[1].trim_end_matches('*'),
        "{s}"
    );
}

#[test]
fn una_solucion_incorrecta_no_entra_al_ranking() {
    let t = temporal();
    let mala = t.join("mala.s");
    std::fs::write(&mala, "HLT\n").unwrap();
    let (c, s, _) = correr(&["desafio", "evaluar", DESAFIO, mala.to_str().unwrap()]);
    assert_eq!(c, SALIDA_NO_RESUELVE, "{s}");
}

#[test]
fn desafio_ver_y_errores() {
    let (c, s, _) = correr(&["desafio", "ver", DESAFIO]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("menos es mejor"));
    assert_eq!(correr(&["desafio"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["desafio", "ver", "no-existe"]).0, SALIDA_ERROR_USO);
    assert_eq!(
        correr(&["desafio", "comparar", DESAFIO]).0,
        SALIDA_ERROR_USO
    );
    let _ = Path::new(RAIZ);
}
