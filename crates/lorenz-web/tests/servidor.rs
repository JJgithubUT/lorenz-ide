use std::io::{Read, Write};
use std::net::TcpStream;

use lorenz_web::{Limites, Respuesta, Router, Servidor};

fn router() -> Router {
    Router::nuevo()
        .get("/hola", |_| Respuesta::texto(200, "hola"))
        .get("/api/:id/x", |p| {
            Respuesta::texto(200, p.parametro("id").unwrap_or(""))
        })
        .post("/eco", |p| {
            Respuesta::nueva(200, "application/octet-stream", p.cuerpo.clone())
        })
        .get("/boom", |_| panic!("fallo"))
        .get("/f/*resto", |p| {
            Respuesta::texto(200, p.parametro("resto").unwrap_or(""))
        })
}

fn pedir(s: &Servidor, crudo: &[u8]) -> String {
    let mut c = TcpStream::connect(s.direccion()).unwrap();
    c.write_all(crudo).unwrap();
    let mut o = Vec::new();
    let _ = c.read_to_end(&mut o);
    String::from_utf8_lossy(&o).into_owned()
}

fn nuevo(l: Limites) -> Servidor {
    Servidor::iniciar("127.0.0.1:0", router(), l).unwrap()
}

#[test]
fn sirve_rutas_y_parametros() {
    let s = nuevo(Limites::default());
    assert!(pedir(&s, b"GET /hola HTTP/1.1\r\nHost: localhost\r\n\r\n").ends_with("hola"));
    assert!(pedir(&s, b"GET /api/42/x HTTP/1.1\r\nHost: 127.0.0.1:80\r\n\r\n").ends_with("42"));
    assert!(pedir(
        &s,
        b"GET /f/a%20b/c.txt HTTP/1.1\r\nHost: localhost\r\n\r\n"
    )
    .ends_with("a b/c.txt"));
    s.detener();
}

#[test]
fn errores_de_ruta() {
    let s = nuevo(Limites::default());
    assert!(pedir(&s, b"GET /nada HTTP/1.1\r\nHost: localhost\r\n\r\n").starts_with("HTTP/1.1 404"));
    let r = pedir(&s, b"DELETE /hola HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(r.starts_with("HTTP/1.1 405") && r.contains("Allow: GET"));
    assert!(pedir(&s, b"GET /boom HTTP/1.1\r\nHost: localhost\r\n\r\n").starts_with("HTTP/1.1 500"));
    assert!(
        pedir(&s, b"GET /hola HTTP/1.1\r\nHost: localhost\r\n\r\n").ends_with("hola"),
        "sigue vivo tras un panico"
    );
    s.detener();
}

#[test]
fn cuerpo_y_limites() {
    let s = nuevo(Limites {
        cuerpo_max: 10,
        cabeceras_max: 200,
        ..Limites::default()
    });
    assert!(pedir(
        &s,
        b"POST /eco HTTP/1.1\r\nHost: localhost\r\nContent-Length: 3\r\n\r\nabc"
    )
    .ends_with("abc"));
    assert!(pedir(
        &s,
        b"POST /eco HTTP/1.1\r\nHost: localhost\r\nContent-Length: 99\r\n\r\n"
    )
    .starts_with("HTTP/1.1 413"));
    assert!(pedir(&s, b"POST /eco HTTP/1.1\r\nHost: localhost\r\n\r\n").starts_with("HTTP/1.1 411"));
    let grande = format!(
        "GET /hola HTTP/1.1\r\nHost: localhost\r\nX: {}\r\n\r\n",
        "a".repeat(500)
    );
    assert!(pedir(&s, grande.as_bytes()).starts_with("HTTP/1.1 400"));
    assert!(pedir(&s, b"BASURA\r\n\r\n").starts_with("HTTP/1.1 400"));
    assert!(pedir(
        &s,
        b"POST /eco HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n"
    )
    .starts_with("HTTP/1.1 501"));
    s.detener();
}

#[test]
fn rechaza_hosts_ajenos() {
    let s = nuevo(Limites::default());
    assert!(
        pedir(&s, b"GET /hola HTTP/1.1\r\nHost: malo.example\r\n\r\n").starts_with("HTTP/1.1 403")
    );
    assert!(pedir(&s, b"GET /hola HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 403"));
    assert!(
        pedir(&s, b"GET /hola HTTP/1.1\r\nHost: [::1]:8080\r\n\r\n").starts_with("HTTP/1.1 200")
    );
    s.detener();
}

#[test]
fn head_sin_cuerpo_y_cierre() {
    let s = nuevo(Limites::default());
    let r = pedir(&s, b"HEAD /hola HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(r.starts_with("HTTP/1.1 200") && r.ends_with("\r\n\r\n"));
    let d = s.direccion();
    s.detener();
    assert!(TcpStream::connect_timeout(&d, std::time::Duration::from_millis(300)).is_err());
}

#[test]
fn con_demasiadas_conexiones_responde_503() {
    let s = nuevo(Limites {
        conexiones_max: 1,
        espera: std::time::Duration::from_secs(2),
        ..Limites::default()
    });
    let mut lenta = TcpStream::connect(s.direccion()).unwrap();
    lenta.write_all(b"GET /hola HTT").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(pedir(&s, b"GET /hola HTTP/1.1\r\nHost: localhost\r\n\r\n").starts_with("HTTP/1.1 503"));
    drop(lenta);
    s.detener();
}
