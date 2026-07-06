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

    fn pedir_bytes(&self, metodo: &str, ruta: &str) -> (u16, Vec<u8>) {
        let mut c = TcpStream::connect(self.0.direccion()).unwrap();
        write!(c, "{metodo} {ruta} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut o = Vec::new();
        c.read_to_end(&mut o).unwrap();
        let estado = std::str::from_utf8(&o[9..12]).unwrap().parse().unwrap();
        let corte = o.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        (estado, o[corte..].to_vec())
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

#[test]
fn la_orden_ide_arranca_y_se_cierra() {
    let raiz = proyecto();
    let (mut s, mut e) = (Vec::new(), Vec::new());
    let args: Vec<String> = [
        "ide",
        raiz.to_str().unwrap(),
        "--puerto",
        "0",
        "--segundos",
        "0",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(
        lorenz_cli::ejecutar_cli(&args, &mut s, &mut e),
        lorenz_cli::SALIDA_OK
    );
    assert!(String::from_utf8(s)
        .unwrap()
        .contains("Lorenz IDE en http://127.0.0.1:"));
    let args: Vec<String> = ["ide", "no-existe-xyz"].map(String::from).to_vec();
    assert_eq!(
        lorenz_cli::ejecutar_cli(&args, &mut Vec::new(), &mut Vec::new()),
        lorenz_cli::SALIDA_ERROR_USO
    );
    let args: Vec<String> = ["--seguro", "ide"].map(String::from).to_vec();
    assert_ne!(
        lorenz_cli::ejecutar_cli(&args, &mut Vec::new(), &mut Vec::new()),
        lorenz_cli::SALIDA_OK
    );
    let args: Vec<String> = ["ide", "--puerto", "99999"].map(String::from).to_vec();
    assert_eq!(
        lorenz_cli::ejecutar_cli(&args, &mut Vec::new(), &mut Vec::new()),
        lorenz_cli::SALIDA_ERROR_USO
    );
}

#[test]
fn editar_estado_desensamblar_y_retroceder() {
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
    let base = format!("/api/sesiones/{id}");
    let (_, d) = c.pedir("GET", &format!("{base}/desensamblado?desde=0&n=3"), None);
    let lineas = d.get("lineas").unwrap().como_lista().unwrap();
    assert_eq!(lineas.len(), 3);
    assert_eq!(
        lineas[0].get("actual").and_then(Json::como_bool),
        Some(true)
    );
    assert_eq!(
        c.pedir(
            "POST",
            &format!("{base}/grabar"),
            Some(r#"{"activa":true}"#)
        )
        .0,
        200
    );
    c.pedir("POST", &format!("{base}/paso"), Some(r#"{"n":2}"#));
    let (e, j) = c.pedir("POST", &format!("{base}/atras"), Some(r#"{"n":1}"#));
    assert_eq!(e, 200);
    assert_eq!(entero(&j, "pc"), 1);
    let (e, _) = c.pedir(
        "POST",
        &format!("{base}/registro"),
        Some(r#"{"nombre":"R5","valor":7}"#),
    );
    assert_eq!(e, 200);
    assert_eq!(
        c.pedir(
            "POST",
            &format!("{base}/registro"),
            Some(r#"{"nombre":"Z99","valor":7}"#)
        )
        .0,
        422
    );
    assert_eq!(
        c.pedir(
            "POST",
            &format!("{base}/registro"),
            Some(r#"{"nombre":"R5"}"#)
        )
        .0,
        400
    );
    assert_eq!(
        c.pedir(
            "POST",
            &format!("{base}/memoria"),
            Some(r#"{"direccion":200,"valor":9}"#)
        )
        .0,
        200
    );
    let (_, m) = c.pedir("GET", &format!("{base}/memoria?desde=200&n=1"), None);
    assert_eq!(
        m.get("celdas").unwrap().como_lista().unwrap()[0].como_entero(),
        Some(9)
    );
    assert_eq!(
        c.pedir("GET", &format!("{base}/pantalla/nada"), None).0,
        404
    );
    assert_eq!(c.pedir("GET", &format!("{base}/serie/nada"), None).0, 404);
}

#[test]
fn api_del_editor() {
    let raiz = proyecto();
    std::fs::write(
        raiz.join("q.s"),
        "inicio: LI R1, 5\n        ADDI R2, R1, 1\n        BEQ R1, R1, inicio\n        FOO R3\n",
    )
    .unwrap();
    let c = Cliente::nuevo(&raiz);
    // lenguaje
    let (e, l) = c.pedir("GET", "/api/lenguaje?maquina=maquina.toml", None);
    assert_eq!(e, 200);
    let mn: Vec<_> = l
        .get("mnemonicos")
        .unwrap()
        .como_lista()
        .unwrap()
        .iter()
        .filter_map(Json::como_texto)
        .collect();
    assert!(mn.contains(&"ADDI") && mn.contains(&"HLT"));
    assert!(l
        .get("registros")
        .unwrap()
        .como_lista()
        .unwrap()
        .iter()
        .any(|r| r.como_texto() == Some("R1")));
    assert!(l.get("directivas").unwrap().serializar().contains(".org"));
    assert_eq!(c.pedir("GET", "/api/lenguaje?maquina=../x", None).0, 400);
    // diagnosticos con linea
    let (e, j) = c.pedir(
        "POST",
        "/api/ensamblar",
        Some(r#"{"maquina":"maquina.toml","fuente":"q.s"}"#),
    );
    assert_eq!(e, 422);
    let d = &j.get("diagnosticos").unwrap().como_lista().unwrap()[0];
    assert_eq!(entero(d, "linea"), 4);
    assert_eq!(d.get("archivo").and_then(Json::como_texto), Some("q.s"));
    // simbolos si ensambla
    std::fs::write(
        raiz.join("q.s"),
        "inicio: LI R1, 5\n        ADDI R2, R1, 1\n        BEQ R1, R1, inicio\n        HLT\n",
    )
    .unwrap();
    let (e, j) = c.pedir(
        "POST",
        "/api/ensamblar",
        Some(r#"{"maquina":"maquina.toml","fuente":"q.s"}"#),
    );
    assert_eq!(e, 200);
    let s = &j.get("simbolos").unwrap().como_lista().unwrap()[0];
    assert_eq!(s.get("nombre").and_then(Json::como_texto), Some("inicio"));
    assert_eq!(entero(s, "linea"), 1);
    // codificacion bajo el cursor
    let (e, j) = c.pedir(
        "POST",
        "/api/codificacion",
        Some(r#"{"maquina":"maquina.toml","fuente":"q.s","linea":2}"#),
    );
    assert_eq!(e, 200);
    let w = &j.get("palabras").unwrap().como_lista().unwrap()[0];
    assert_eq!(entero(w, "direccion"), 1);
    assert!(w
        .get("binario")
        .and_then(Json::como_texto)
        .unwrap()
        .chars()
        .all(|c| c == '0' || c == '1'));
    let (_, j) = c.pedir(
        "POST",
        "/api/codificacion",
        Some(r#"{"maquina":"maquina.toml","fuente":"q.s","linea":99}"#),
    );
    assert!(j.get("palabras").unwrap().como_lista().unwrap().is_empty());
    // busqueda y reemplazo
    let (_, b) = c.pedir("POST", "/api/buscar", Some(r#"{"texto":"inicio"}"#));
    assert_eq!(b.get("resultados").unwrap().como_lista().unwrap().len(), 2);
    let (_, b) = c.pedir(
        "POST",
        "/api/buscar",
        Some(r#"{"texto":"INICIO","mayusculas":true}"#),
    );
    assert!(b
        .get("resultados")
        .unwrap()
        .como_lista()
        .unwrap()
        .is_empty());
    assert_eq!(
        c.pedir("POST", "/api/buscar", Some(r#"{"texto":""}"#)).0,
        400
    );
    let (_, r) = c.pedir(
        "POST",
        "/api/reemplazar",
        Some(r#"{"texto":"inicio","por":"arranque"}"#),
    );
    assert_eq!(entero(&r, "cambios"), 2);
    assert!(std::fs::read_to_string(raiz.join("q.s"))
        .unwrap()
        .starts_with("arranque:"));
    assert_eq!(
        c.pedir(
            "POST",
            "/api/ensamblar",
            Some(r#"{"maquina":"maquina.toml","fuente":"q.s"}"#)
        )
        .0,
        200
    );
    // documentacion
    let (e, d) = c.pedir("GET", "/api/documentacion?maquina=maquina.toml", None);
    assert_eq!(e, 200);
    assert!(d
        .get("html")
        .and_then(Json::como_texto)
        .unwrap()
        .contains("ADDI"));
}

#[test]
fn compilar_sgt_con_diagnosticos() {
    let raiz = proyecto();
    let c = Cliente::nuevo(&raiz);
    std::fs::write(
        raiz.join("a.sgt"),
        "funcion principal() {\n    var x: u16 = 1;\n    x = y;\n}\n",
    )
    .unwrap();
    let (e, j) = c.pedir(
        "POST",
        "/api/compilar",
        Some(r#"{"maquina":"maquina.toml","fuente":"a.sgt"}"#),
    );
    assert_eq!(e, 422, "{}", j.serializar());
    let d = &j.get("diagnosticos").unwrap().como_lista().unwrap()[0];
    assert_eq!(entero(d, "linea"), 3);
    assert_eq!(
        c.pedir(
            "POST",
            "/api/compilar",
            Some(r#"{"maquina":"maquina.toml","fuente":"../a.sgt"}"#)
        )
        .0,
        400
    );
}

#[test]
fn traza_ordenes_eventos_y_explicacion() {
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
    let base = format!("/api/sesiones/{id}");
    c.pedir(
        "POST",
        &format!("{base}/grabar"),
        Some(r#"{"activa":true}"#),
    );
    c.pedir("POST", &format!("{base}/paso"), Some(r#"{"n":3}"#));
    let (_, e) = c.pedir("GET", &format!("{base}/estado"), None);
    assert_eq!(entero(&e, "instante"), 3);
    assert!(entero(&e, "traza_longitud") >= 3);
    let (s, j) = c.pedir("POST", &format!("{base}/instante"), Some(r#"{"n":1}"#));
    assert_eq!(s, 200);
    assert_eq!(entero(&j, "instante"), 1);
    assert_eq!(entero(&j, "pc"), 1);
    assert_eq!(
        c.pedir("POST", &format!("{base}/instante"), Some(r#"{"n":999}"#))
            .0,
        422
    );
    assert_eq!(
        c.pedir("POST", &format!("{base}/instante"), Some("{}")).0,
        400
    );
    let (_, x) = c.pedir("GET", &format!("{base}/explicacion"), None);
    assert!(x
        .get("texto")
        .and_then(Json::como_texto)
        .unwrap()
        .contains("ADDI"));
    assert!(x.get("registros").unwrap().como_lista().is_some());
    let (_, o) = c.pedir(
        "POST",
        &format!("{base}/orden"),
        Some(r#"{"linea":"registros"}"#),
    );
    assert!(o
        .get("texto")
        .and_then(Json::como_texto)
        .unwrap()
        .contains("R1"));
    let (_, o) = c.pedir(
        "POST",
        &format!("{base}/orden"),
        Some(r#"{"linea":"paso"}"#),
    );
    assert_eq!(entero(o.get("estado").unwrap(), "instante"), 2);
    let (_, o) = c.pedir(
        "POST",
        &format!("{base}/orden"),
        Some(r#"{"linea":"nada"}"#),
    );
    assert!(o
        .get("texto")
        .and_then(Json::como_texto)
        .unwrap()
        .starts_with("error"));
    let (e, ev) = c.pedir("GET", &format!("{base}/eventos"), None);
    assert_eq!(e, 200);
    assert!(ev
        .get("interrupciones")
        .unwrap()
        .como_lista()
        .unwrap()
        .is_empty());
}

#[test]
fn mapa_de_memoria_por_regiones() {
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
    let base = format!("/api/sesiones/{id}");
    let (s, j) = c.pedir("GET", &format!("{base}/mapa?bloque=16"), None);
    assert_eq!(s, 200);
    assert_eq!(entero(&j, "bloque"), 16);
    let regiones = j.get("regiones").and_then(Json::como_lista).unwrap();
    assert!(!regiones.is_empty());
    for r in regiones {
        let desde = entero(r, "desde");
        let hasta = entero(r, "hasta");
        let esperados = ((hasta - desde + 1) as usize).div_ceil(16);
        assert_eq!(
            r.get("ocupacion").and_then(Json::como_lista).unwrap().len(),
            esperados
        );
    }
    // El programa cargado ocupa el primer bloque de alguna region.
    let ocupado = regiones.iter().any(|r| {
        r.get("ocupacion")
            .and_then(Json::como_lista)
            .unwrap()
            .iter()
            .any(|o| o.como_entero().unwrap_or(0) > 0)
    });
    assert!(ocupado);
    assert_eq!(
        c.pedir("GET", &format!("{base}/mapa?bloque=0"), None).0,
        400
    );
}

#[test]
fn el_ide_cubre_todas_las_ordenes_del_depurador() {
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
    let (_, j) = c.pedir(
        "POST",
        &format!("/api/sesiones/{id}/orden"),
        Some(r#"{"linea":"ayuda"}"#),
    );
    let ayuda = j
        .get("texto")
        .and_then(Json::como_texto)
        .unwrap()
        .to_string();
    // Las ordenes son las palabras que abren una linea sangrada con dos espacios.
    let mut ordenes: Vec<String> = ayuda
        .lines()
        .filter(|l| l.starts_with("  ") && !l.starts_with("   "))
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect();
    ordenes.sort();
    ordenes.dedup();
    assert!(ordenes.len() >= 30, "{ordenes:?}");
    let tabla: Vec<&str> = lorenz_cli::ide::paridad::PARIDAD
        .iter()
        .map(|p| p.0)
        .collect();
    for o in &ordenes {
        assert!(
            tabla.contains(&o.as_str()),
            "la orden `{o}` no tiene equivalente en el IDE"
        );
    }
    for t in &tabla {
        assert!(
            ordenes.iter().any(|o| o == t),
            "`{t}` ya no existe en el depurador"
        );
    }
    // Cada orden se entiende en el panel de ordenes: ninguna da «desconocida».
    for o in &ordenes {
        if matches!(o.as_str(), "fin" | "explorar") {
            continue;
        }
        let cuerpo = format!(r#"{{"linea":"{o}"}}"#);
        let (s, r) = c.pedir("POST", &format!("/api/sesiones/{id}/orden"), Some(&cuerpo));
        assert_eq!(s, 200);
        let t = r.get("texto").and_then(Json::como_texto).unwrap_or("");
        assert!(!t.contains("desconocida"), "{o}: {t}");
    }
}

#[test]
fn visor_de_la_unidad_grafica() {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let c = Cliente::nuevo(&raiz);
    // Una maquina sin [gpu] se rechaza con un mensaje claro.
    let (s, j) = c.pedir(
        "POST",
        "/api/gpu",
        Some(r#"{"maquina":"ejemplos/risc-16/maquina.toml","programa":"ejemplos/gpu-16/programas/triangulo.s"}"#),
    );
    assert_eq!(s, 422, "{j:?}");
    let (s, j) = c.pedir(
        "POST",
        "/api/gpu",
        Some(r#"{"maquina":"ejemplos/gpu-16/maquina.toml","programa":"ejemplos/gpu-16/programas/triangulo.s"}"#),
    );
    assert_eq!(s, 200, "{j:?}");
    let id = entero(&j, "id");
    assert!(entero(&j, "ancho") > 0 && entero(&j, "alto") > 0);
    let base = format!("/api/gpu/{id}");
    let (_, e0) = c.pedir("GET", &format!("{base}/estado"), None);
    assert_eq!(entero(&e0, "instrucciones"), 0);
    // A medio programa todavia no esta todo dibujado; al terminar, si.
    let (s, e1) = c.pedir("POST", &format!("{base}/avanzar"), Some(r#"{"n":8}"#));
    assert_eq!(s, 200);
    assert_eq!(entero(&e1, "instrucciones"), 8);
    let (_, e2) = c.pedir("POST", &format!("{base}/avanzar"), Some(r#"{"n":1000}"#));
    assert_eq!(e2.get("detenida").and_then(Json::como_bool), Some(true));
    let m = e2.get("metricas").unwrap();
    assert!(entero(m, "triangulos") >= 1);
    assert!(entero(m, "pixeles_escritos") > 0);
    // La imagen es un PNG de verdad.
    let (s, cuerpo) = c.pedir_bytes("GET", &format!("{base}/imagen"));
    assert_eq!(s, 200);
    assert_eq!(&cuerpo[..8], b"\x89PNG\r\n\x1a\n");
    let (_, px) = c.pedir("GET", &format!("{base}/pixel?x=3&y=5"), None);
    assert!(px
        .get("texto")
        .and_then(Json::como_texto)
        .unwrap()
        .contains("(3, 5)"));
    assert_eq!(c.pedir("GET", "/api/gpu/999/estado", None).0, 404);
}
