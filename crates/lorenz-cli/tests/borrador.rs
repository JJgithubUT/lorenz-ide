//! `lorenz borrador` y su API (RF-ECO-045): del texto a una maquina que ensambla y ejecuta.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use lorenz_cli::ide::{router, Ide};
use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_USO, SALIDA_OK};
use lorenz_web::{Json, Limites, Servidor};

static N: AtomicUsize = AtomicUsize::new(0);

fn temporal() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-borr-{}-{}",
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

#[test]
fn el_borrador_se_guarda_y_ejecuta_un_programa() {
    let d = temporal();
    let m = d.join("m.toml");
    let (c, _, e) = correr(&[
        "borrador",
        "Procesador",
        "de",
        "16",
        "bits",
        "con",
        "8",
        "registros,",
        "64",
        "KiB",
        "de",
        "memoria",
        "y",
        "suma",
        "-o",
        m.to_str().unwrap(),
    ]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(e.contains("Supuestos"), "{e}");
    let p = d.join("p.s");
    std::fs::write(
        &p,
        "        LDI R0, 5\n        LDI R1, 7\n        ADD R0, R1\n        HLT\n",
    )
    .unwrap();
    let (c, s, e) = correr(&["ejecutar", m.to_str().unwrap(), p.to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("R0=0xc"), "{s}");
}

#[test]
fn sin_descripcion_o_con_algo_imposible_es_error_de_uso() {
    assert_eq!(correr(&["borrador"]).0, SALIDA_ERROR_USO);
    let (c, _, e) = correr(&["borrador", "palabras", "de", "12", "bits"]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("12"), "{e}");
}

#[test]
fn la_api_devuelve_texto_y_supuestos() {
    let d = temporal();
    let ide = Arc::new(Ide::nuevo(&d));
    let s = Servidor::iniciar("127.0.0.1:0", router(ide), Limites::default()).unwrap();
    use std::io::{Read, Write};
    let cuerpo = r#"{"descripcion":"8 bits con registros X, Y, Z y W"}"#;
    let mut t = std::net::TcpStream::connect(s.direccion()).unwrap();
    write!(
        t,
        "POST /api/arquitectura/borrador HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cuerpo}",
        cuerpo.len()
    )
    .unwrap();
    let mut r = String::new();
    t.read_to_string(&mut r).unwrap();
    assert!(r.starts_with("HTTP/1.1 200"), "{r}");
    let j = Json::analizar(r.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    let texto = j.get("texto").and_then(Json::como_texto).unwrap();
    assert!(texto.contains("nombre = \"W\""));
    assert!(j.get("supuestos").and_then(Json::como_lista).is_some());
    s.detener();
}
