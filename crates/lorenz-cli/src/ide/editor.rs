//! Lo que necesita el editor: lenguaje de la arquitectura, diagnosticos, codificacion, busqueda y
//! documentacion (RF-IDE-010 a RF-IDE-016, RF-IDE-038).

use std::sync::Arc;

use lorenz_ens::ensamblador::{Ensamblador, Fuente, Objeto};
use lorenz_ens::error::ErrorEns;
use lorenz_ens::preprocesar::CargadorDeDisco;
use lorenz_web::{Json, Peticion, Respuesta, Router};

use super::sesion::Ide;

/// Directivas del ensamblador, para resaltar y autocompletar.
pub const DIRECTIVAS: [&str; 13] = [
    ".org",
    ".seccion",
    ".equ",
    ".const",
    ".db",
    ".dw",
    ".palabra",
    ".direccion",
    ".espacio",
    ".incluir",
    ".macro",
    ".fin",
    ".externo",
];

const BUSQUEDA_MAX: usize = 1000;
const ARCHIVO_MAX: u64 = 1024 * 1024;

fn ok(j: Json) -> Respuesta {
    Respuesta::json(200, &j.serializar())
}

fn error(estado: u16, m: impl Into<String>) -> Respuesta {
    Respuesta::json(
        estado,
        &Json::objeto([("error", Json::texto(m))]).serializar(),
    )
}

fn cuerpo(p: &Peticion) -> Result<Json, Respuesta> {
    let t = p
        .cuerpo_texto()
        .map_err(|_| error(400, "el cuerpo no es UTF-8"))?;
    Json::analizar(t).map_err(|e| error(400, format!("JSON invalido: {e}")))
}

fn texto<'a>(j: &'a Json, k: &str) -> Result<&'a str, Respuesta> {
    j.get(k)
        .and_then(Json::como_texto)
        .ok_or_else(|| error(400, format!("falta `{k}` (texto)")))
}

/// Ensambla un archivo y devuelve los errores con su posicion, sin darles formato.
pub fn ensamblar_estructurado(
    maquina: &lorenz_arch::modelo::Maquina,
    ruta: &str,
) -> Result<Objeto, Vec<ErrorEns>> {
    let contenido = std::fs::read_to_string(ruta).map_err(|e| {
        vec![ErrorEns {
            archivo: ruta.into(),
            linea: 1,
            columna: 1,
            causa: lorenz_ens::error::Causa::Otra(format!("no se pudo leer: {e}")),
        }]
    })?;
    let raiz = Fuente::nueva(ruta, contenido);
    let e = Ensamblador::nuevo(maquina).map_err(|m| {
        vec![ErrorEns {
            archivo: ruta.into(),
            linea: 1,
            columna: 1,
            causa: lorenz_ens::error::Causa::Otra(m),
        }]
    })?;
    e.ensamblar_con(&raiz, &CargadorDeDisco).map(|(o, _)| o)
}

pub fn diagnosticos(errores: &[ErrorEns], raiz: &std::path::Path) -> Json {
    errores
        .iter()
        .map(|e| {
            let archivo = std::path::Path::new(&e.archivo)
                .strip_prefix(raiz)
                .map_or_else(
                    |_| e.archivo.clone(),
                    |r| r.to_string_lossy().replace('\\', "/"),
                );
            Json::objeto([
                ("archivo", archivo.into()),
                ("linea", e.linea.into()),
                ("columna", e.columna.into()),
                ("mensaje", e.causa.to_string().into()),
            ])
        })
        .collect::<Vec<_>>()
        .into()
}

pub fn simbolos(o: &Objeto, raiz: &std::path::Path) -> Json {
    o.simbolos
        .iter()
        .map(|(n, s)| {
            let archivo = std::path::Path::new(&s.archivo)
                .strip_prefix(raiz)
                .map_or_else(
                    |_| s.archivo.clone(),
                    |r| r.to_string_lossy().replace('\\', "/"),
                );
            Json::objeto([
                ("nombre", n.as_str().into()),
                ("valor", Json::Num(s.valor as f64)),
                ("etiqueta", s.etiqueta.into()),
                ("archivo", archivo.into()),
                ("linea", s.linea.into()),
            ])
        })
        .collect::<Vec<_>>()
        .into()
}

fn archivos_de_texto(dir: &std::path::Path, fuera: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut e: Vec<_> = rd.filter_map(Result::ok).collect();
    e.sort_by_key(|x| x.file_name());
    for x in e {
        let n = x.file_name().to_string_lossy().into_owned();
        if n.starts_with('.') || n == "target" {
            continue;
        }
        let Ok(t) = x.file_type() else { continue };
        if t.is_symlink() {
            continue;
        }
        if t.is_dir() {
            archivos_de_texto(&x.path(), fuera);
        } else if x.metadata().is_ok_and(|m| m.len() <= ARCHIVO_MAX) {
            fuera.push(x.path());
        }
    }
}

/// Posiciones (en bytes) de `aguja` en `pajar`; sin distinguir mayusculas ASCII si `mayusculas` es falso.
pub fn posiciones(pajar: &str, aguja: &str, mayusculas: bool) -> Vec<usize> {
    if aguja.is_empty() || aguja.len() > pajar.len() {
        return Vec::new();
    }
    let (p, a) = (pajar.as_bytes(), aguja.as_bytes());
    let mut v = Vec::new();
    let mut i = 0;
    while i + a.len() <= p.len() {
        let coincide = if mayusculas {
            &p[i..i + a.len()] == a
        } else {
            p[i..i + a.len()].eq_ignore_ascii_case(a)
        };
        if coincide && pajar.is_char_boundary(i) && pajar.is_char_boundary(i + a.len()) {
            v.push(i);
            i += a.len();
        } else {
            i += 1;
        }
    }
    v
}

pub fn rutas(router: Router, ide: Arc<Ide>) -> Router {
    let (a, b, c, d, e, g) = (
        ide.clone(),
        ide.clone(),
        ide.clone(),
        ide.clone(),
        ide.clone(),
        ide,
    );
    router
        .get("/api/lenguaje", move |p| {
            let Some(m) = p.consulta("maquina") else {
                return error(400, "falta `maquina`");
            };
            let ruta = match a.ruta_texto(m) {
                Ok(r) => r,
                Err(e) => return error(400, e),
            };
            let maquina = match lorenz_arch::cargar_archivo(&ruta) {
                Ok(x) => x,
                Err(er) => return error(422, format!("{m}: {er}")),
            };
            let mn = match Ensamblador::nuevo(&maquina) {
                Ok(x) => x
                    .mnemonicos()
                    .into_iter()
                    .map(Json::from)
                    .collect::<Vec<_>>(),
                Err(er) => return error(422, er),
            };
            let regs: Vec<Json> = maquina
                .registros
                .iter()
                .map(|r| r.nombre.as_str().into())
                .chain(maquina.pares.iter().map(|r| r.nombre.as_str().into()))
                .collect();
            let ins: Vec<Json> = maquina
                .instrucciones
                .iter()
                .map(|i| {
                    Json::objeto([
                        ("mnemonico", i.mnemonico.as_str().into()),
                        ("sintaxis", i.sintaxis.as_str().into()),
                        ("descripcion", i.descripcion.as_str().into()),
                    ])
                })
                .collect();
            ok(Json::objeto([
                ("nombre", maquina.nombre.as_str().into()),
                ("bits_palabra", u64::from(maquina.bits_palabra).into()),
                ("mnemonicos", mn.into()),
                ("registros", regs.into()),
                ("instrucciones", ins.into()),
                (
                    "directivas",
                    DIRECTIVAS
                        .iter()
                        .map(|d| Json::from(*d))
                        .collect::<Vec<_>>()
                        .into(),
                ),
            ]))
        })
        .post("/api/compilar", move |p| {
            let j = match cuerpo(p) {
                Ok(j) => j,
                Err(r) => return r,
            };
            let (m, f) = match (texto(&j, "maquina"), texto(&j, "fuente")) {
                (Ok(m), Ok(f)) => (m, f),
                (Err(r), _) | (_, Err(r)) => return r,
            };
            let (rm, rf) = match (g.ruta_texto(m), g.ruta_texto(f)) {
                (Ok(x), Ok(y)) => (x, y),
                (Err(e), _) | (_, Err(e)) => return error(400, e),
            };
            let maquina = match lorenz_arch::cargar_archivo(&rm) {
                Ok(x) => x,
                Err(er) => return error(422, format!("{m}: {er}")),
            };
            let t = match std::fs::read_to_string(&rf) {
                Ok(t) => t,
                Err(er) => return error(404, format!("{f}: {er}")),
            };
            let r = lorenz_sgt::compilar::compilar_con(
                &rf,
                &t,
                &maquina,
                lorenz_sgt::analisis::Opciones::default(),
                &lorenz_sgt::compilar::en_carpeta_de(&rf),
            );
            match r {
                Ok(c) => ok(Json::objeto([
                    ("palabras", c.objeto.tamano().into()),
                    ("simbolos", simbolos(&c.objeto, g.raiz())),
                ])),
                Err(ds) => {
                    let diag: Vec<Json> = ds
                        .iter()
                        .map(|d| {
                            let archivo = std::path::Path::new(&d.archivo)
                                .strip_prefix(g.raiz())
                                .map_or_else(
                                    |_| d.archivo.clone(),
                                    |r| r.to_string_lossy().replace('\\', "/"),
                                );
                            Json::objeto([
                                ("archivo", archivo.into()),
                                ("linea", u64::from(d.lugar.linea).into()),
                                ("columna", u64::from(d.lugar.columna).into()),
                                ("mensaje", d.mensaje.as_str().into()),
                                (
                                    "sugerencia",
                                    d.sugerencia.clone().map_or(Json::Nulo, Json::from),
                                ),
                            ])
                        })
                        .collect();
                    Respuesta::json(
                        422,
                        &Json::objeto([
                            ("error", "no se pudo compilar".into()),
                            ("diagnosticos", diag.into()),
                        ])
                        .serializar(),
                    )
                }
            }
        })
        .post("/api/codificacion", move |p| {
            let j = match cuerpo(p) {
                Ok(j) => j,
                Err(r) => return r,
            };
            let (m, f) = match (texto(&j, "maquina"), texto(&j, "fuente")) {
                (Ok(m), Ok(f)) => (m, f),
                (Err(r), _) | (_, Err(r)) => return r,
            };
            let Some(linea) = j
                .get("linea")
                .and_then(Json::como_entero)
                .filter(|l| *l >= 1)
            else {
                return error(400, "falta `linea` (desde 1)");
            };
            let (rm, rf) = match (b.ruta_texto(m), b.ruta_texto(f)) {
                (Ok(x), Ok(y)) => (x, y),
                (Err(e), _) | (_, Err(e)) => return error(400, e),
            };
            let maquina = match lorenz_arch::cargar_archivo(&rm) {
                Ok(x) => x,
                Err(er) => return error(422, format!("{m}: {er}")),
            };
            let objeto = match ensamblar_estructurado(&maquina, &rf) {
                Ok(o) => o,
                Err(errs) => {
                    return Respuesta::json(
                        422,
                        &Json::objeto([
                            ("error", "no se pudo ensamblar".into()),
                            ("diagnosticos", diagnosticos(&errs, b.raiz())),
                        ])
                        .serializar(),
                    )
                }
            };
            let ancho = (maquina.bits_palabra as usize).div_ceil(4);
            let Some(entrada) = objeto.mapa.iter().find(|x| {
                x.linea == linea as usize && std::path::Path::new(&x.archivo).ends_with(f)
            }) else {
                return ok(Json::objeto([("palabras", Json::Lista(vec![]))]));
            };
            let mut palabras = Vec::new();
            for dir in entrada.direccion..entrada.direccion + entrada.longitud as u64 {
                let v = objeto
                    .palabras()
                    .into_iter()
                    .find(|(d, _)| *d == dir)
                    .map(|(_, w)| w);
                if let Some(w) = v {
                    palabras.push(Json::objeto([
                        ("direccion", dir.into()),
                        (
                            "binario",
                            format!("{w:0bits$b}", bits = maquina.bits_palabra as usize).into(),
                        ),
                        ("hex", format!("{w:0ancho$x}").into()),
                    ]));
                }
            }
            ok(Json::objeto([("palabras", palabras.into())]))
        })
        .post("/api/buscar", move |p| {
            let j = match cuerpo(p) {
                Ok(j) => j,
                Err(r) => return r,
            };
            let aguja = match texto(&j, "texto") {
                Ok(t) if !t.is_empty() => t,
                Ok(_) => return error(400, "el texto a buscar esta vacio"),
                Err(r) => return r,
            };
            let may = j
                .get("mayusculas")
                .and_then(Json::como_bool)
                .unwrap_or(false);
            let mut archivos = Vec::new();
            archivos_de_texto(c.raiz(), &mut archivos);
            let mut res = Vec::new();
            'fuera: for ruta in archivos {
                let Ok(t) = std::fs::read_to_string(&ruta) else {
                    continue;
                };
                let rel = ruta
                    .strip_prefix(c.raiz())
                    .map(|r| r.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                for (n, linea) in t.lines().enumerate() {
                    for col in posiciones(linea, aguja, may) {
                        if res.len() >= BUSQUEDA_MAX {
                            break 'fuera;
                        }
                        res.push(Json::objeto([
                            ("ruta", rel.clone().into()),
                            ("linea", (n + 1).into()),
                            ("columna", (col + 1).into()),
                            ("texto", linea.into()),
                        ]));
                    }
                }
            }
            let truncado = res.len() >= BUSQUEDA_MAX;
            ok(Json::objeto([
                ("resultados", res.into()),
                ("truncado", truncado.into()),
            ]))
        })
        .post("/api/reemplazar", move |p| {
            let j = match cuerpo(p) {
                Ok(j) => j,
                Err(r) => return r,
            };
            let (aguja, por) = match (texto(&j, "texto"), texto(&j, "por")) {
                (Ok(t), Ok(r)) if !t.is_empty() => (t, r),
                (Ok(_), Ok(_)) => return error(400, "el texto a buscar esta vacio"),
                (Err(r), _) | (_, Err(r)) => return r,
            };
            let may = j
                .get("mayusculas")
                .and_then(Json::como_bool)
                .unwrap_or(false);
            let mut archivos = Vec::new();
            archivos_de_texto(d.raiz(), &mut archivos);
            let (mut cambios, mut tocados) = (0usize, Vec::new());
            for ruta in archivos {
                let Ok(t) = std::fs::read_to_string(&ruta) else {
                    continue;
                };
                let pos = posiciones(&t, aguja, may);
                if pos.is_empty() {
                    continue;
                }
                let mut nuevo = String::with_capacity(t.len());
                let mut ultimo = 0;
                for i in &pos {
                    nuevo.push_str(&t[ultimo..*i]);
                    nuevo.push_str(por);
                    ultimo = i + aguja.len();
                }
                nuevo.push_str(&t[ultimo..]);
                let rel = ruta
                    .strip_prefix(d.raiz())
                    .map(|r| r.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                if d.permitir_escritura(&rel, &nuevo).is_err() {
                    continue;
                }
                if std::fs::write(&ruta, nuevo).is_ok() {
                    cambios += pos.len();
                    tocados.push(Json::from(
                        ruta.strip_prefix(d.raiz())
                            .map(|r| r.to_string_lossy().replace('\\', "/"))
                            .unwrap_or_default(),
                    ));
                }
            }
            ok(Json::objeto([
                ("cambios", cambios.into()),
                ("archivos", tocados.into()),
            ]))
        })
        .get("/api/documentacion", move |p| {
            let Some(m) = p.consulta("maquina") else {
                return error(400, "falta `maquina`");
            };
            let ruta = match e.ruta_texto(m) {
                Ok(r) => r,
                Err(er) => return error(400, er),
            };
            let t = match std::fs::read_to_string(&ruta) {
                Ok(t) => t,
                Err(er) => return error(404, format!("{m}: {er}")),
            };
            match crate::documentar::generar(&ruta, &t) {
                Ok(md) => ok(Json::objeto([
                    ("markdown", md.clone().into()),
                    ("html", crate::html::markdown_a_html(&md).into()),
                ])),
                Err(er) => error(422, er),
            }
        })
}
