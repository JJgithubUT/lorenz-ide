//! El IDE alojado (`lorenz servir`): espacios aislados, cuotas, compartir, derivar y galeria.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_cli::ide::servicio::{OpcionesServicio, Servicio};
use lorenz_web::{Json, Limites, Servidor};

static N: AtomicUsize = AtomicUsize::new(0);

fn datos() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-servicio-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&d);
    d
}

struct Respuesta {
    estado: u16,
    cabeceras: String,
    cuerpo: Vec<u8>,
}

impl Respuesta {
    fn json(&self) -> Json {
        Json::analizar(std::str::from_utf8(&self.cuerpo).unwrap()).unwrap_or(Json::Nulo)
    }
    fn texto(&self) -> String {
        String::from_utf8_lossy(&self.cuerpo).into_owned()
    }
    fn cabecera(&self, n: &str) -> Option<String> {
        self.cabeceras.lines().find_map(|l| {
            let (k, v) = l.split_once(": ")?;
            k.eq_ignore_ascii_case(n).then(|| v.to_string())
        })
    }
}

struct Sitio {
    servidor: Servidor,
}

/// Una persona con navegador: guarda su galleta.
struct Visitante<'a> {
    sitio: &'a Sitio,
    galleta: Option<String>,
}

impl Sitio {
    fn nuevo(opciones: OpcionesServicio) -> Sitio {
        let servicio = Servicio::nuevo(opciones).unwrap();
        let servidor =
            Servidor::iniciar_con("127.0.0.1:0", servicio.manejador(), Limites::default()).unwrap();
        Sitio { servidor }
    }

    fn visitante(&self) -> Visitante<'_> {
        Visitante {
            sitio: self,
            galleta: None,
        }
    }
}

impl Visitante<'_> {
    fn con<'a>(galleta: &str, sitio: &'a Sitio) -> Visitante<'a> {
        Visitante {
            sitio,
            galleta: Some(galleta.to_string()),
        }
    }

    fn pedir_con(&mut self, metodo: &str, ruta: &str, cuerpo: &str, extra: &str) -> Respuesta {
        let mut c = TcpStream::connect(self.sitio.servidor.direccion()).unwrap();
        let galleta = self
            .galleta
            .as_ref()
            .map_or(String::new(), |g| format!("Cookie: lorenz={g}\r\n"));
        write!(
            c,
            "{metodo} {ruta} HTTP/1.1\r\nHost: localhost\r\n{galleta}{extra}Content-Length: {}\r\n\r\n{cuerpo}",
            cuerpo.len()
        )
        .unwrap();
        let mut o = Vec::new();
        c.read_to_end(&mut o).unwrap();
        let corte = o.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        let cab = String::from_utf8_lossy(&o[..corte]).into_owned();
        let estado = cab[9..12].parse().unwrap();
        let r = Respuesta {
            estado,
            cabeceras: cab,
            cuerpo: o[corte..].to_vec(),
        };
        if let Some(sc) = r.cabecera("set-cookie") {
            if let Some(v) = sc.strip_prefix("lorenz=") {
                self.galleta = v.split(';').next().map(str::to_string);
            }
        }
        r
    }

    fn pedir(&mut self, metodo: &str, ruta: &str, cuerpo: &str) -> Respuesta {
        self.pedir_con(metodo, ruta, cuerpo, "")
    }

    fn escribir(&mut self, ruta: &str, texto: &str) -> Respuesta {
        let c = Json::objeto([("ruta", ruta.into()), ("texto", texto.into())]).serializar();
        self.pedir("PUT", "/api/archivo", &c)
    }

    fn rutas(&mut self) -> Vec<String> {
        let r = self.pedir("GET", "/api/proyecto", "").json();
        r.get("archivos")
            .and_then(Json::como_lista)
            .unwrap()
            .iter()
            .filter_map(|a| a.get("ruta").and_then(Json::como_texto).map(str::to_string))
            .collect()
    }
}

fn texto_de(j: &Json, k: &str) -> String {
    j.get(k).and_then(Json::como_texto).unwrap().to_string()
}

const MAQ: &str = "ejemplo-8/maquina.toml";
const PROG: &str = "ejemplo-8/programas/suma.s";

#[test]
fn cada_visitante_tiene_su_espacio_con_ejemplos() {
    let d = datos();
    let sitio = Sitio::nuevo(OpcionesServicio::en(&d));
    let mut a = sitio.visitante();
    let r = a.pedir("GET", "/api/proyecto", "");
    assert_eq!(r.estado, 200);
    assert!(r.cabecera("set-cookie").unwrap().contains("HttpOnly"));
    assert!(a.rutas().contains(&MAQ.to_string()));
    assert_eq!(a.escribir("mio.s", "        HLT\n").estado, 200);
    let mut b = sitio.visitante();
    assert!(b.rutas().contains(&MAQ.to_string()));
    assert!(!b.rutas().contains(&"mio.s".to_string()), "B ve lo de A");
    assert_ne!(a.galleta, b.galleta);
    // Una galleta inventada no abre el espacio de nadie: se le da uno nuevo.
    let mut c = Visitante::con("00000000000000000000000000000000", &sitio);
    assert!(!c.rutas().contains(&"mio.s".to_string()));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn el_trabajo_sobrevive_a_un_reinicio() {
    let d = datos();
    let galleta;
    {
        let sitio = Sitio::nuevo(OpcionesServicio::en(&d));
        let mut a = sitio.visitante();
        a.escribir("guardado.s", "        HLT\n");
        galleta = a.galleta.clone().unwrap();
    }
    let sitio = Sitio::nuevo(OpcionesServicio::en(&d));
    let mut a = Visitante::con(&galleta, &sitio);
    assert!(a.rutas().contains(&"guardado.s".to_string()));
    let r = a.pedir("GET", "/api/archivo?ruta=guardado.s", "");
    assert!(texto_de(&r.json(), "texto").contains("HLT"));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn cuotas_y_rutas_peligrosas() {
    let d = datos();
    let mut o = OpcionesServicio::en(&d);
    o.restricciones.max_archivo = 100;
    o.restricciones.max_archivos = 12;
    let sitio = Sitio::nuevo(o);
    let mut a = sitio.visitante();
    assert_eq!(a.escribir("grande.s", &"x".repeat(101)).estado, 403);
    assert_eq!(a.escribir("peque.s", "HLT").estado, 200);
    // Los ejemplos ocupan 6 archivos: caben seis mas, no siete.
    for i in 0..5 {
        assert_eq!(a.escribir(&format!("a{i}.s"), "HLT").estado, 200);
    }
    let r = a.escribir("de-mas.s", "HLT");
    assert_eq!(r.estado, 403, "{}", r.texto());
    assert!(r.texto().contains("maximo"));
    // Reescribir uno que ya existe no cuenta como archivo nuevo.
    assert_eq!(a.escribir("peque.s", "NOP").estado, 200);
    // Rutas que salen, absolutas, dispositivos externos
    for malo in [
        "hereda = \"../../etc/passwd\"\n",
        "hereda = \"/etc/passwd\"\n",
        ".incluir \"../../../secreto\"\n",
        "tipo = \"externo\"\n",
    ] {
        assert_eq!(a.escribir("x.toml", malo).estado, 403, "{malo}");
    }
    // El archivo mismo no puede estar fuera
    assert_eq!(a.escribir("../fuera.s", "HLT").estado, 400);
    assert_eq!(a.escribir("/abs.s", "HLT").estado, 400);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn modo_exportar_y_origen() {
    let d = datos();
    let sitio = Sitio::nuevo(OpcionesServicio::en(&d));
    let mut a = sitio.visitante();
    let m = a.pedir("GET", "/api/modo", "").json();
    assert_eq!(m.get("alojado").and_then(Json::como_bool), Some(true));
    let lim = m.get("limitaciones").and_then(Json::como_lista).unwrap();
    assert!(lim.len() >= 5);
    assert!(lim
        .iter()
        .any(|l| texto_de(l, "funcion").contains("externos")));
    // Exportar da un .lpkg valido con el trabajo del visitante
    a.escribir("tarea.s", "        HLT\n");
    let r = a.pedir("GET", "/api/exportar", "");
    assert_eq!(r.estado, 200);
    assert!(r
        .cabecera("content-disposition")
        .unwrap()
        .contains("proyecto.lpkg"));
    let paq = lorenz_paq::Paquete::desde_bytes(&r.cuerpo).expect("paquete valido e integro");
    assert!(paq.archivos.contains_key(MAQ) && paq.archivos.contains_key("tarea.s"));
    assert_eq!(paq.manifiesto.nombre, "proyecto-web");
    // Una peticion que cambia cosas desde otra pagina se rechaza
    let c = Json::objeto([("ruta", "x.s".into()), ("texto", "HLT".into())]).serializar();
    let r = a.pedir_con("PUT", "/api/archivo", &c, "Origin: http://malo.example\r\n");
    assert_eq!(r.estado, 403);
    let r = a.pedir_con("PUT", "/api/archivo", &c, "Origin: http://localhost\r\n");
    assert_eq!(r.estado, 200);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn limite_de_peticiones() {
    let d = datos();
    let mut o = OpcionesServicio::en(&d);
    o.peticiones_por_minuto = 5;
    let sitio = Sitio::nuevo(o);
    let mut a = sitio.visitante();
    let estados: Vec<u16> = (0..8)
        .map(|_| a.pedir("GET", "/api/modo", "").estado)
        .collect();
    assert_eq!(&estados[..5], &[200; 5]);
    assert!(estados[5..].iter().all(|e| *e == 429), "{estados:?}");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn compartir_abrir_ejecutar_y_derivar() {
    let d = datos();
    let sitio = Sitio::nuevo(OpcionesServicio::en(&d));
    let mut ana = sitio.visitante();
    let cuerpo = Json::objeto([("maquina", MAQ.into()), ("programa", PROG.into())]).serializar();
    let r = ana.pedir("POST", "/api/compartir", &cuerpo);
    assert_eq!(r.estado, 200, "{}", r.texto());
    let id = texto_de(&r.json(), "id");
    assert_eq!(id.len(), 20);
    // El mismo contenido da el mismo enlace, aunque lo comparta otra persona
    let mut luis = sitio.visitante();
    assert_eq!(
        texto_de(&luis.pedir("POST", "/api/compartir", &cuerpo).json(), "id"),
        id
    );
    // Cambiar el programa cambia el enlace
    ana.escribir(PROG, "        HLT\n");
    let r2 = ana.pedir("POST", "/api/compartir", &cuerpo);
    assert_ne!(texto_de(&r2.json(), "id"), id);

    // Una tercera persona abre el enlace sin instalar nada y lo ejecuta
    let mut eva = sitio.visitante();
    let r = eva.pedir("GET", &format!("/c/{id}"), "");
    assert_eq!(r.estado, 302);
    let destino = r.cabecera("location").unwrap();
    assert!(
        destino.starts_with(&format!("/?abrir=compartido/{id}/{MAQ}")),
        "{destino}"
    );
    let sesion = Json::objeto([
        ("maquina", format!("compartido/{id}/{MAQ}").into()),
        ("programa", format!("compartido/{id}/{PROG}").into()),
    ])
    .serializar();
    let r = eva.pedir("POST", "/api/sesiones", &sesion);
    assert_eq!(r.estado, 201, "{}", r.texto());
    let sid = r.json().get("id").and_then(Json::como_entero).unwrap();
    let r = eva.pedir(
        "POST",
        &format!("/api/sesiones/{sid}/continuar"),
        r#"{"limite":10000}"#,
    );
    assert_eq!(r.estado, 200, "{}", r.texto());
    assert_eq!(
        r.json()
            .get("parada")
            .and_then(|p| p.get("tipo"))
            .and_then(Json::como_texto),
        Some("detenida")
    );
    // Lo compartido no se edita ni se borra...
    let ruta = format!("compartido/{id}/{PROG}");
    assert_eq!(eva.escribir(&ruta, "HLT").estado, 403);
    assert_eq!(
        eva.pedir("DELETE", &format!("/api/archivo?ruta={ruta}"), "")
            .estado,
        403
    );
    // ...pero se deriva una copia propia que si.
    let der =
        Json::objeto([("id", id.as_str().into()), ("nombre", "mi-copia".into())]).serializar();
    let r = eva.pedir("POST", "/api/derivar", &der);
    assert_eq!(r.estado, 200, "{}", r.texto());
    assert_eq!(texto_de(&r.json(), "programa"), format!("mi-copia/{PROG}"));
    assert_eq!(
        eva.escribir(&format!("mi-copia/{PROG}"), "        NOP\n        HLT\n")
            .estado,
        200
    );
    assert_eq!(
        eva.pedir("POST", "/api/derivar", &der).estado,
        422,
        "no pisa una copia"
    );
    let malo = Json::objeto([("id", id.as_str().into()), ("nombre", "../x".into())]).serializar();
    assert_eq!(eva.pedir("POST", "/api/derivar", &malo).estado, 400);
    // Enlaces inexistentes o inventados
    assert_eq!(eva.pedir("GET", "/c/00000000000000000000", "").estado, 404);
    assert_eq!(eva.pedir("GET", "/c/../../etc/passwd", "").estado, 404);
    // Lo compartido no puede apuntar fuera del espacio
    ana.escribir("p/maquina.toml", "[maquina]\nnombre = \"x\"\n");
    ana.escribir("p/p.s", ".incluir \"../../../etc/passwd\"\n");
    let c = Json::objeto([
        ("maquina", "p/maquina.toml".into()),
        ("programa", "p/p.s".into()),
    ])
    .serializar();
    assert_ne!(ana.pedir("POST", "/api/compartir", &c).estado, 200);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn galeria_con_busqueda() {
    let d = datos();
    let sitio = Sitio::nuevo(OpcionesServicio::en(&d));
    let mut ana = sitio.visitante();
    let cuerpo = Json::objeto([("maquina", MAQ.into()), ("programa", PROG.into())]).serializar();
    let id = texto_de(&ana.pedir("POST", "/api/compartir", &cuerpo).json(), "id");
    let pub_ = |titulo: &str| {
        Json::objeto([
            ("id", id.as_str().into()),
            ("titulo", titulo.into()),
            ("descripcion", "Una máquina de 8 bits para aprender".into()),
            (
                "etiquetas",
                Json::Lista(vec!["didáctica".into(), "8-bits".into()]),
            ),
        ])
        .serializar()
    };
    assert_eq!(ana.pedir("POST", "/api/galeria", &pub_("ab")).estado, 400);
    assert_eq!(
        ana.pedir("POST", "/api/galeria", &pub_("Mi máquina de 8 bits"))
            .estado,
        200
    );
    // Sin compartir primero no se publica
    let sin = Json::objeto([
        ("id", "00000000000000000000".into()),
        ("titulo", "Algo bueno".into()),
    ])
    .serializar();
    assert_eq!(ana.pedir("POST", "/api/galeria", &sin).estado, 404);

    let mut eva = sitio.visitante();
    let buscar = |v: &mut Visitante, q: &str| -> Vec<Json> {
        v.pedir("GET", &format!("/api/galeria?q={q}"), "")
            .json()
            .get("resultados")
            .and_then(Json::como_lista)
            .unwrap()
            .to_vec()
    };
    assert_eq!(buscar(&mut eva, "").len(), 1);
    assert_eq!(buscar(&mut eva, "maquina").len(), 1, "sin acentos");
    assert_eq!(buscar(&mut eva, "didactica+8").len(), 1, "varias palabras");
    assert_eq!(
        buscar(&mut eva, "DIDACTICA%20cohete").len(),
        0,
        "todas deben aparecer"
    );
    assert_eq!(
        buscar(&mut eva, "DIDACTICA%208").len(),
        1,
        "mayusculas y varias palabras"
    );
    assert_eq!(buscar(&mut eva, "cohete").len(), 0);
    let hallado = &buscar(&mut eva, "ejemplo")[0];
    assert_eq!(texto_de(hallado, "enlace"), format!("/c/{id}"));
    assert!(hallado.get("autor").is_none(), "no se publica quien es");
    assert_eq!(hallado.get("mia").and_then(Json::como_bool), Some(false));
    assert_eq!(
        buscar(&mut ana, "")[0].get("mia").and_then(Json::como_bool),
        Some(true)
    );
    let r = eva.pedir("GET", "/api/galeria?etiqueta=DIDACTICA", "");
    assert_eq!(
        r.json()
            .get("resultados")
            .and_then(Json::como_lista)
            .unwrap()
            .len(),
        1
    );
    // Solo quien publica puede retirar
    assert_eq!(
        eva.pedir("DELETE", &format!("/api/galeria/{id}"), "")
            .estado,
        403
    );
    assert_eq!(
        ana.pedir("DELETE", &format!("/api/galeria/{id}"), "")
            .estado,
        200
    );
    assert_eq!(buscar(&mut eva, "").len(), 0);
    let _ = std::fs::remove_dir_all(d);
}
