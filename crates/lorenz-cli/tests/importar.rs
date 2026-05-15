//! Importar arquitecturas desde tablas de instrucciones (RF-ECO-015).

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_cli::importar::importar_texto;
use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_USO, SALIDA_OK};

static N: AtomicUsize = AtomicUsize::new(0);

fn temporal() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-imp-{}-{}",
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

fn importada(nombre: &str, dir: &std::path::Path) -> String {
    let destino = dir.join(format!("{nombre}.toml"));
    let tabla = format!("../../importadores/{nombre}.tabla");
    let (c, _, e) = correr(&["importar", &tabla, "-o", destino.to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK, "{e}");
    destino.to_str().unwrap().to_string()
}

#[test]
fn jdh8_importada_ejecuta_un_programa() {
    let t = temporal();
    let m = importada("jdh-8", &t);
    let p = t.join("p.s");
    std::fs::write(&p, "LDI A, 5\nLDI B, 7\nADD A, B\nST [0x8000], A\nHLT\n").unwrap();
    let (c, s, e) = correr(&["ejecutar", &m, p.to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("A=0xc"), "{s}");
}

#[test]
fn astro8_importada_ejecuta_un_programa() {
    let t = temporal();
    let m = importada("astro-8", &t);
    let p = t.join("p.s");
    std::fs::write(
        &p,
        "LDI A, 5\nLDI B, 9\nADD A, B\nLDI C, 1\nSUB A, C\nHLT\n",
    )
    .unwrap();
    let (c, s, e) = correr(&["ejecutar", &m, p.to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("A=0xd"), "{s}");
}

#[test]
fn sin_salida_escribe_la_maquina_por_pantalla() {
    let (c, s, _) = correr(&["importar", "../../importadores/jdh-8.tabla"]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("nombre = \"jdh-8\"") && s.contains("[[instruccion]]"));
}

#[test]
fn los_rangos_salen_del_ancho_del_campo() {
    let t = "maquina: m palabra=8 direccion=8\nregistros: A B\npc: PC 8\nregion: ram 0 0xFF rw\n\
             instruccion: ADDS {rd}, {imms} ; 0001 rd:1 imms:3 ; rd = rd + imms ; 1\n\
             instruccion: HLT ; 1111 .... ; detener ; 1\n";
    let toml = importar_texto(t).unwrap();
    assert!(toml.contains("rango = [-4, 3]"), "{toml}");
}

#[test]
fn las_tablas_malas_se_explican() {
    let casos = [
        ("", "falta la linea `maquina:`"),
        (
            "maquina: m palabra=8 direccion=8\n",
            "ninguna `instruccion:`",
        ),
        ("maquina: m palabra=8 direccion=8\nvolar: x\n", "linea 2"),
        ("sin dos puntos\n", "linea 1"),
        (
            "maquina: m palabra=8 direccion=8\ninstruccion: X {zzz} ; 0000 ....",
            "SINTAXIS",
        ),
        (
            "maquina: m palabra=8 direccion=8\ninstruccion: X {zzz} ; 0000 .... ; ; 1\n",
            "no se que operando",
        ),
        (
            "maquina: m palabra=8 direccion=8\ninstruccion: X ; 0000 .... ; ; mucho\n",
            "ciclos",
        ),
    ];
    for (tabla, esperado) in casos {
        let e = importar_texto(tabla).unwrap_err();
        assert!(e.contains(esperado), "{tabla:?} -> {e}");
    }
}

#[test]
fn una_maquina_incoherente_no_se_escribe() {
    let t = temporal();
    let tabla = t.join("mala.tabla");
    // La palabra de la instruccion mide 5 bits en una maquina de 8.
    std::fs::write(&tabla, "maquina: m palabra=8 direccion=8\nregion: ram 0 0xFF rw\ninstruccion: HLT ; 1111 . ; detener ; 1\n").unwrap();
    let destino = t.join("m.toml");
    let (c, _, e) = correr(&[
        "importar",
        tabla.to_str().unwrap(),
        "-o",
        destino.to_str().unwrap(),
    ]);
    assert_eq!(c, SALIDA_ERROR_USO, "{e}");
    assert!(!destino.exists());
}

#[test]
fn errores_de_uso() {
    assert_eq!(correr(&["importar"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["importar", "no-existe.tabla"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["importar", "a", "b"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["importar", "a", "-o"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["importar", "a", "--raro"]).0, SALIDA_ERROR_USO);
}
