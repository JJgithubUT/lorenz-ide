//! Plantillas de proyecto incluidas (RF-IDE-007, RF-IDE-008).

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_USO, SALIDA_OK};

static N: AtomicUsize = AtomicUsize::new(0);

fn carpeta() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-plantillas-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&d);
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

#[test]
fn lista_las_tres_plantillas() {
    let (c, s, _) = correr(&["plantillas"]);
    assert_eq!(c, SALIDA_OK);
    for n in ["programa-simple", "so-minimo", "arquitectura-nueva"] {
        assert!(s.contains(n), "{s}");
    }
    assert_eq!(correr(&["plantillas", "x"]).0, SALIDA_ERROR_USO);
}

#[test]
fn cada_plantilla_crea_un_proyecto_que_se_comprueba_y_corre() {
    for p in ["programa-simple", "so-minimo", "arquitectura-nueva"] {
        let d = carpeta().join("mi-proyecto");
        let ruta = d.to_str().unwrap();
        let (c, s, e) = correr(&["nuevo", p, ruta]);
        assert_eq!(c, SALIDA_OK, "{p}: {e}");
        assert!(s.contains(&format!("plantilla {p}")), "{s}");
        let (c, s, e) = correr(&["proyecto", ruta]);
        assert_eq!(c, SALIDA_OK, "{p}: {e}");
        assert!(s.starts_with("proyecto mi-proyecto"), "{p}: {s}");
        let (c, _, e) = correr(&["tarea", ruta, "correr"]);
        assert_eq!(c, SALIDA_OK, "{p}: {e}");
        // El nombre del proyecto llega tambien a la maquina.
        let m = std::fs::read_to_string(d.join("maquina.toml")).unwrap();
        assert!(
            m.contains("nombre         = \"mi-proyecto\"")
                || m.contains("nombre = \"mi-proyecto\"")
                || m.contains("mi-proyecto"),
            "{p}"
        );
        assert!(!m.contains("__NOMBRE__"));
    }
}

#[test]
fn el_programa_simple_pasa_sus_pruebas() {
    let d = carpeta().join("p");
    let ruta = d.to_str().unwrap();
    correr(&["nuevo", "programa-simple", ruta]);
    let (c, s, e) = correr(&["tarea", ruta, "verificar"]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("1 pasan"), "{s}");
}

#[test]
fn nombre_elige_el_del_proyecto() {
    let d = carpeta().join("carpeta");
    let ruta = d.to_str().unwrap();
    correr(&["nuevo", "arquitectura-nueva", ruta, "--nombre", "mi-cpu"]);
    let p = std::fs::read_to_string(d.join("proyecto.toml")).unwrap();
    assert!(p.contains("nombre = \"mi-cpu\""), "{p}");
    let (_, s, _) = correr(&["proyecto", ruta]);
    assert!(s.starts_with("proyecto mi-cpu"), "{s}");
}

#[test]
fn el_nombre_de_la_carpeta_se_limpia() {
    let d = carpeta().join("Mi Proyecto (1)");
    correr(&["nuevo", "programa-simple", d.to_str().unwrap()]);
    let p = std::fs::read_to_string(d.join("proyecto.toml")).unwrap();
    assert!(p.contains("nombre = \"Mi-Proyecto--1\""), "{p}");
}

#[test]
fn rechaza_un_destino_con_contenido_y_una_plantilla_desconocida() {
    let d = carpeta();
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("algo"), "x").unwrap();
    let (c, _, e) = correr(&["nuevo", "programa-simple", d.to_str().unwrap()]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("ya existe"), "{e}");
    let (c, _, e) = correr(&["nuevo", "no-existe", carpeta().to_str().unwrap()]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(
        e.contains("programa-simple") && e.contains("so-minimo"),
        "{e}"
    );
}

#[test]
fn una_carpeta_con_el_nombre_de_una_plantilla_tiene_prioridad() {
    let base = carpeta();
    let local = base.join("programa-simple");
    std::fs::create_dir_all(&local).unwrap();
    std::fs::write(
        local.join("maquina.toml"),
        std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap(),
    )
    .unwrap();
    let destino = base.join("destino");
    let args = ["nuevo", local.to_str().unwrap(), destino.to_str().unwrap()];
    let (c, s, e) = correr(&args);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(!s.contains("plantilla programa-simple"), "{s}");
    assert!(!destino.join("proyecto.toml").exists());
}
