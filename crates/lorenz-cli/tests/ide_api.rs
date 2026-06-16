//! Pruebas de la API del IDE contra un servidor real en un puerto libre.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use lorenz_cli::ide::{router, Ide};
use lorenz_web::{Json, Limites, Servidor};

static N: AtomicUsize = AtomicUsize::new(0);

fn proyecto() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-ide-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d).unwrap();
    let origen = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ejercicios/maquina.toml");
    std::fs::copy(origen, d.join("maquina.toml")).unwrap();
    std::fs::write(
        d.join("p.s"),
        "        LI    R1, 42\n        ADDI  R2, R1, 1\n        HLT\n",
    )
    .unwrap();
    d
}

struct Cliente(Servidor);

impl Cliente {
    fn nuevo(raiz: &PathBuf) -> Self {
        let ide = Arc::new(Ide::nuevo(raiz));
        Cliente(Servidor::iniciar("127.0.0.1:0", router(ide), Limites::default()).unwrap())
    }

    fn pedir(&self, metodo: &str, ruta: &str, cuerpo: Option<&str>) -> (u16, Json) {
        let mut c = TcpStream::connect(self.0.direccion()).unwrap();
        let cuerpo = cuerpo.unwrap_or("");
        write!(
            c,
            "{metodo} {ruta} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
        let mut o = String::new();
        c.read_to_string(&mut o).unwrap();
        let estado = o[9..12].parse().unwrap();
        let texto = o.split_once("\r\n\r\n").map_or("", |x| x.1);
        (estado, Json::analizar(texto).unwrap_or(Json::Nulo))
    }
}

fn entero(j: &Json, k: &str) -> i64 {
    j.get(k)
        .and_then(Json::como_entero)
        .unwrap_or_else(|| panic!("falta {k} en {}", j.serializar()))
}

#[test]
fn archivos_del_proyecto() {
    let raiz = proyecto();
    let c = Cliente::nuevo(&raiz);
    let (e, j) = c.pedir("GET", "/api/proyecto", None);
    assert_eq!(e, 200);
    assert!(j.serializar().contains("p.s"));
    let (e, _) = c.pedir(
        "PUT",
        "/api/archivo",
        Some(r#"{"ruta":"sub/n.s","texto":"HLT\n"}"#),
    );
    assert_eq!(e, 200);
    assert_eq!(
        std::fs::read_to_string(raiz.join("sub/n.s")).unwrap(),
        "HLT\n"
    );
    let (_, j) = c.pedir("GET", "/api/archivo?ruta=sub/n.s", None);
    assert_eq!(j.get("texto").and_then(Json::como_texto), Some("HLT\n"));
    assert_eq!(c.pedir("DELETE", "/api/archivo?ruta=sub/n.s", None).0, 200);
    assert_eq!(c.pedir("GET", "/api/archivo?ruta=sub/n.s", None).0, 404);
}

#[test]
fn no_deja_salir_del_proyecto() {
    let raiz = proyecto();
    let c = Cliente::nuevo(&raiz);
    for ruta in ["../x", "/etc/passwd", "a/../../x", "C:/x"] {
        let (e, _) = c.pedir("GET", &format!("/api/archivo?ruta={ruta}"), None);
        assert_eq!(e, 400, "{ruta}");
    }
    let (e, _) = c.pedir(
        "PUT",
        "/api/archivo",
        Some(r#"{"ruta":"../fuera.txt","texto":"x"}"#),
    );
    assert_eq!(e, 400);
    assert!(!raiz.parent().unwrap().join("fuera.txt").exists());
    #[cfg(unix)]
    {
        let ajeno = proyecto();
        std::os::unix::fs::symlink(&ajeno, raiz.join("enlace")).unwrap();
        assert_eq!(
            c.pedir(
                "PUT",
                "/api/archivo",
                Some(r#"{"ruta":"enlace/x.txt","texto":"x"}"#)
            )
            .0,
            400
        );
        assert!(!ajeno.join("x.txt").exists());
    }
}

#[test]
fn ensamblar_informa_errores() {
    let raiz = proyecto();
    std::fs::write(raiz.join("mal.s"), "        FOO R1\n").unwrap();
    let c = Cliente::nuevo(&raiz);
    let (e, j) = c.pedir(
        "POST",
        "/api/ensamblar",
        Some(r#"{"maquina":"maquina.toml","fuente":"p.s"}"#),
    );
    assert_eq!(e, 200);
    assert_eq!(entero(&j, "palabras"), 3);
    let (e, j) = c.pedir(
        "POST",
        "/api/ensamblar",
        Some(r#"{"maquina":"maquina.toml","fuente":"mal.s"}"#),
    );
    assert_eq!(e, 422);
    assert!(!j.get("errores").unwrap().como_lista().unwrap().is_empty());
    assert_eq!(c.pedir("POST", "/api/ensamblar", Some("{")).0, 400);
    assert_eq!(c.pedir("POST", "/api/ensamblar", Some("{}")).0, 400);
}

#[test]
fn sesion_paso_a_paso() {
    let raiz = proyecto();
    let c = Cliente::nuevo(&raiz);
    let (e, j) = c.pedir(
        "POST",
        "/api/sesiones",
        Some(r#"{"maquina":"maquina.toml","programa":"p.s"}"#),
    );
    assert_eq!(e, 201);
    let id = entero(&j, "id");
    let (_, est) = c.pedir("GET", &format!("/api/sesiones/{id}/estado"), None);
    assert_eq!(entero(&est, "instrucciones"), 0);
    assert!(!est
        .get("registros")
        .unwrap()
        .como_lista()
        .unwrap()
        .is_empty());
    let (e, j) = c.pedir(
        "POST",
        &format!("/api/sesiones/{id}/paso"),
        Some(r#"{"n":1}"#),
    );
    assert_eq!(e, 200);
    assert_eq!(entero(j.get("estado").unwrap(), "instrucciones"), 1);
    let (_, j) = c.pedir("POST", &format!("/api/sesiones/{id}/continuar"), Some("{}"));
    assert_eq!(
        j.get("parada")
            .unwrap()
            .get("tipo")
            .and_then(Json::como_texto),
        Some("detenida")
    );
    let regs = j
        .get("estado")
        .unwrap()
        .get("registros")
        .unwrap()
        .como_lista()
        .unwrap()
        .to_vec();
    let r2 = regs
        .iter()
        .find(|r| r.get("nombre").and_then(Json::como_texto) == Some("R2"))
        .unwrap();
    assert_eq!(entero(r2, "valor"), 43);
    let (_, j) = c.pedir("POST", &format!("/api/sesiones/{id}/reiniciar"), None);
    assert_eq!(entero(&j, "instrucciones"), 0);
    assert_eq!(
        c.pedir("DELETE", &format!("/api/sesiones/{id}"), None).0,
        200
    );
    assert_eq!(
        c.pedir("GET", &format!("/api/sesiones/{id}/estado"), None)
            .0,
        404
    );
}

#[test]
fn rupturas_y_memoria() {
    let raiz = proyecto();
    let c = Cliente::nuevo(&raiz);
    let id = entero(
        &c.pedir(
            "POST",
            "/api/sesiones",
            Some(r#"{"maquina":"maquina.toml","programa":"p.s"}"#),
        )
        .1,
        "id",
    );
    let (e, r) = c.pedir(
        "POST",
        &format!("/api/sesiones/{id}/rupturas"),
        Some(r#"{"direccion":1}"#),
    );
    assert_eq!(e, 201);
    let rid = entero(&r, "id");
    let (_, j) = c.pedir("POST", &format!("/api/sesiones/{id}/continuar"), Some("{}"));
    assert_eq!(
        j.get("parada")
            .unwrap()
            .get("tipo")
            .and_then(Json::como_texto),
        Some("ruptura")
    );
    assert_eq!(entero(j.get("estado").unwrap(), "pc"), 1);
    let (_, l) = c.pedir("GET", &format!("/api/sesiones/{id}/rupturas"), None);
    assert_eq!(
        entero(
            &l.get("rupturas").unwrap().como_lista().unwrap()[0],
            "aciertos"
        ),
        1
    );
    assert_eq!(
        c.pedir(
            "DELETE",
            &format!("/api/sesiones/{id}/rupturas/{rid}"),
            None
        )
        .0,
        200
    );
    assert_eq!(
        c.pedir(
            "DELETE",
            &format!("/api/sesiones/{id}/rupturas/{rid}"),
            None
        )
        .0,
        404
    );
    let (e, m) = c.pedir(
        "GET",
        &format!("/api/sesiones/{id}/memoria?desde=0&n=3"),
        None,
    );
    assert_eq!(e, 200);
    assert_eq!(m.get("celdas").unwrap().como_lista().unwrap().len(), 3);
    assert_eq!(
        c.pedir("GET", &format!("/api/sesiones/{id}/memoria?n=99999"), None)
            .0,
        400
    );
    assert_eq!(
        c.pedir(
            "POST",
            &format!("/api/sesiones/{id}/rupturas"),
            Some(r#"{"direccion":"nada_asi"}"#)
        )
        .0,
        422
    );
}

#[test]
fn sesiones_independientes_y_limite() {
    let raiz = proyecto();
    let c = Cliente::nuevo(&raiz);
    let abrir = || {
        entero(
            &c.pedir(
                "POST",
                "/api/sesiones",
                Some(r#"{"maquina":"maquina.toml","programa":"p.s"}"#),
            )
            .1,
            "id",
        )
    };
    let (a, b) = (abrir(), abrir());
    assert_ne!(a, b);
    c.pedir("POST", &format!("/api/sesiones/{a}/continuar"), Some("{}"));
    let (_, ea) = c.pedir("GET", &format!("/api/sesiones/{a}/estado"), None);
    let (_, eb) = c.pedir("GET", &format!("/api/sesiones/{b}/estado"), None);
    assert_eq!(entero(&ea, "instrucciones"), 3);
    assert_eq!(entero(&eb, "instrucciones"), 0);
    for _ in 0..lorenz_cli::ide::MAX_SESIONES {
        c.pedir(
            "POST",
            "/api/sesiones",
            Some(r#"{"maquina":"maquina.toml","programa":"p.s"}"#),
        );
    }
    assert_eq!(
        c.pedir(
            "POST",
            "/api/sesiones",
            Some(r#"{"maquina":"maquina.toml","programa":"p.s"}"#)
        )
        .0,
        422
    );
    assert_eq!(c.pedir("POST", "/api/sesiones/abc/paso", None).0, 400);
    assert_eq!(
        c.pedir("POST", &format!("/api/sesiones/{a}/volar"), None).0,
        404
    );
}
