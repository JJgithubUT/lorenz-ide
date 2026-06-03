//! Instalar paquetes desde una direccion web (RF-ECO-004).

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::JoinHandle;

use lorenz_cli::{ejecutar_cli, SALIDA_OK};
use lorenz_paq::descarga::{descargar, es_url, Url};

static N: AtomicUsize = AtomicUsize::new(0);

fn temporal() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-desc-{}-{}",
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

/// Un paquete minimo y su contenido.
fn paquete() -> Vec<u8> {
    let d = temporal();
    let f = d.join("fuente");
    std::fs::create_dir_all(&f).unwrap();
    std::fs::write(f.join("paquete.toml"), "[paquete]\nnombre = \"remoto\"\nversion = \"1.2.0\"\nautor = \"Ana\"\nlicencia = \"MIT\"\ntipo = \"biblioteca\"\n").unwrap();
    std::fs::write(f.join("a.txt"), "hola").unwrap();
    let out = d.join("remoto.lpkg");
    let (c, _, e) = correr(&[
        "paquete",
        "crear",
        f.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(c, SALIDA_OK, "{e}");
    std::fs::read(out).unwrap()
}

/// Servidor que contesta a cada peticion con lo que diga `responder(ruta)`; devuelve las rutas vistas.
fn servidor(
    peticiones: usize,
    responder: impl Fn(&str) -> Vec<u8> + Send + 'static,
) -> (String, JoinHandle<Vec<String>>) {
    let escucha = TcpListener::bind("127.0.0.1:0").unwrap();
    let direccion = escucha.local_addr().unwrap().to_string();
    let hilo = std::thread::spawn(move || {
        let mut rutas = Vec::new();
        for _ in 0..peticiones {
            let (mut flujo, _) = escucha.accept().unwrap();
            let mut lector = BufReader::new(flujo.try_clone().unwrap());
            let mut primera = String::new();
            lector.read_line(&mut primera).unwrap();
            loop {
                let mut l = String::new();
                lector.read_line(&mut l).unwrap();
                if l.trim().is_empty() {
                    break;
                }
            }
            let ruta = primera.split_whitespace().nth(1).unwrap_or("").to_string();
            let respuesta = responder(&ruta);
            rutas.push(ruta);
            let _ = flujo.write_all(&respuesta);
        }
        rutas
    });
    (direccion, hilo)
}

fn http(cuerpo: &[u8]) -> Vec<u8> {
    let mut r = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        cuerpo.len()
    )
    .into_bytes();
    r.extend_from_slice(cuerpo);
    r
}

#[test]
fn se_analizan_las_direcciones() {
    assert!(es_url("http://x/y") && es_url("HTTPS://x") && !es_url("a/b.lpkg"));
    let u = Url::analizar("http://localhost:8080/a/b.lpkg").unwrap();
    assert_eq!(
        (u.host.as_str(), u.puerto, u.ruta.as_str()),
        ("localhost", 8080, "/a/b.lpkg")
    );
    let u = Url::analizar("http://ejemplo.org").unwrap();
    assert_eq!((u.puerto, u.ruta.as_str()), (80, "/"));
    assert!(Url::analizar("https://x/y").unwrap_err().contains("HTTPS"));
    assert!(Url::analizar("ftp://x/y").is_err());
    assert!(Url::analizar("http://:80/y").is_err());
    assert!(Url::analizar("http://x:abc/y").is_err());
    assert!(Url::analizar("http://u@x/y").is_err());
}

#[test]
fn baja_con_content_length() {
    let p = paquete();
    let copia = p.clone();
    let (d, h) = servidor(1, move |_| http(&copia));
    assert_eq!(descargar(&format!("http://{d}/remoto.lpkg")).unwrap(), p);
    assert_eq!(h.join().unwrap(), ["/remoto.lpkg"]);
}

#[test]
fn baja_troceado() {
    let p = paquete();
    let copia = p.clone();
    let (d, _h) = servidor(1, move |_| {
        let mut r =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
        for trozo in copia.chunks(7) {
            r.extend_from_slice(format!("{:x}\r\n", trozo.len()).as_bytes());
            r.extend_from_slice(trozo);
            r.extend_from_slice(b"\r\n");
        }
        r.extend_from_slice(b"0\r\n\r\n");
        r
    });
    assert_eq!(descargar(&format!("http://{d}/x")).unwrap(), p);
}

#[test]
fn baja_hasta_el_cierre_de_la_conexion() {
    let p = paquete();
    let copia = p.clone();
    let (d, _h) = servidor(1, move |_| {
        let mut r = b"HTTP/1.0 200 OK\r\n\r\n".to_vec();
        r.extend_from_slice(&copia);
        r
    });
    assert_eq!(descargar(&format!("http://{d}/x")).unwrap(), p);
}

#[test]
fn sigue_las_redirecciones() {
    let p = paquete();
    let copia = p.clone();
    let (d, h) = servidor(2, move |ruta| {
        if ruta == "/viejo" {
            b"HTTP/1.1 302 Found\r\nLocation: /nuevo\r\nContent-Length: 0\r\n\r\n".to_vec()
        } else {
            http(&copia)
        }
    });
    assert_eq!(descargar(&format!("http://{d}/viejo")).unwrap(), p);
    assert_eq!(h.join().unwrap(), ["/viejo", "/nuevo"]);
}

#[test]
fn no_se_deja_redirigir_para_siempre() {
    let (d, _h) = servidor(7, |_| {
        b"HTTP/1.1 302 Found\r\nLocation: /otra\r\nContent-Length: 0\r\n\r\n".to_vec()
    });
    assert!(descargar(&format!("http://{d}/x"))
        .unwrap_err()
        .contains("redirecciones"));
}

#[test]
fn los_errores_del_servidor_se_explican() {
    let (d, _h) = servidor(1, |_| {
        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_vec()
    });
    assert!(descargar(&format!("http://{d}/x"))
        .unwrap_err()
        .contains("404"));
    let (d, _h) = servidor(1, |_| b"basura\r\n\r\n".to_vec());
    assert!(descargar(&format!("http://{d}/x"))
        .unwrap_err()
        .contains("invalida"));
    let libre = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .to_string();
    assert!(descargar(&format!("http://{libre}/x"))
        .unwrap_err()
        .contains("no se pudo conectar"));
}

#[test]
fn rechaza_lo_que_pasa_del_maximo() {
    let (d, _h) = servidor(1, |_| {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
            lorenz_paq::descarga::MAXIMO + 1
        )
        .into_bytes()
    });
    assert!(descargar(&format!("http://{d}/x"))
        .unwrap_err()
        .contains("maximo"));
}

#[test]
fn instalar_desde_la_red_comprueba_la_huella() {
    let p = paquete();
    let huella = lorenz_paq::sha256_hex(&p);
    let raiz = temporal();
    let copia = p.clone();
    let (d, _h) = servidor(1, move |_| http(&copia));
    let url = format!("http://{d}/remoto.lpkg");
    let (c, s, e) = correr(&[
        "paquete",
        "instalar",
        &url,
        "--sha256",
        &huella,
        "--en",
        raiz.to_str().unwrap(),
    ]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("instalado remoto 1.2.0"), "{s}");
    let (_, s, _) = correr(&["paquete", "listar", "--en", raiz.to_str().unwrap()]);
    assert!(s.contains("remoto 1.2.0"));
}

#[test]
fn sin_huella_o_con_huella_distinta_no_se_instala() {
    let p = paquete();
    let raiz = temporal();
    let copia = p.clone();
    let (d, _h) = servidor(2, move |_| http(&copia));
    let url = format!("http://{d}/remoto.lpkg");
    let (c, _, e) = correr(&["paquete", "instalar", &url, "--en", raiz.to_str().unwrap()]);
    assert_ne!(c, SALIDA_OK, "{e}");
    assert!(e.contains("--sha256"), "{e}");
    let mala = "0".repeat(64);
    let (c, _, e) = correr(&[
        "paquete",
        "instalar",
        &url,
        "--sha256",
        &mala,
        "--en",
        raiz.to_str().unwrap(),
    ]);
    assert_ne!(c, SALIDA_OK);
    assert!(
        e.contains("no coincide") && e.contains(&lorenz_paq::sha256_hex(&p)),
        "{e}"
    );
    let (_, s, _) = correr(&["paquete", "listar", "--en", raiz.to_str().unwrap()]);
    assert!(s.contains("no hay paquetes"), "{s}");
}

#[test]
fn https_se_rechaza_con_una_salida_alternativa() {
    let (c, _, e) = correr(&[
        "paquete",
        "instalar",
        "https://ejemplo.org/x.lpkg",
        "--sha256",
        &"0".repeat(64),
    ]);
    assert_ne!(c, SALIDA_OK);
    assert!(e.contains("HTTPS"), "{e}");
}

#[test]
fn la_huella_tambien_vale_para_un_archivo_local() {
    let p = paquete();
    let d = temporal();
    let f = d.join("p.lpkg");
    std::fs::write(&f, &p).unwrap();
    let raiz = temporal();
    let (c, _, e) = correr(&[
        "paquete",
        "instalar",
        f.to_str().unwrap(),
        "--sha256",
        &"1".repeat(64),
        "--en",
        raiz.to_str().unwrap(),
    ]);
    assert_ne!(c, SALIDA_OK);
    assert!(e.contains("no coincide"));
    let (c, _, e) = correr(&[
        "paquete",
        "instalar",
        f.to_str().unwrap(),
        "--sha256",
        &lorenz_paq::sha256_hex(&p).to_uppercase(),
        "--en",
        raiz.to_str().unwrap(),
    ]);
    assert_eq!(c, SALIDA_OK, "{e}");
}
