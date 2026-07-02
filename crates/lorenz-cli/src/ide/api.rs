//! Las rutas de la API del IDE. Todo entra y sale como JSON.

use std::sync::Arc;

use lorenz_dbg::depurador::Parada;
use lorenz_web::{Json, Peticion, Respuesta, Router};

use super::sesion::{Ide, SesionIde};

const LIMITE_PASO: u64 = 10_000_000;
const MEMORIA_MAX: u64 = 4096;
const ARCHIVOS_MAX: usize = 5000;

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
    if t.trim().is_empty() {
        return Ok(Json::objeto([]));
    }
    Json::analizar(t).map_err(|e| error(400, format!("JSON invalido: {e}")))
}

fn texto_de<'a>(j: &'a Json, k: &str) -> Result<&'a str, Respuesta> {
    j.get(k)
        .and_then(Json::como_texto)
        .ok_or_else(|| error(400, format!("falta `{k}` (texto)")))
}

fn num_de(j: &Json, k: &str, defecto: u64) -> Result<u64, Respuesta> {
    match j.get(k) {
        None | Some(Json::Nulo) => Ok(defecto),
        Some(v) => v
            .como_entero()
            .filter(|n| *n >= 0)
            .map(|n| n as u64)
            .ok_or_else(|| error(400, format!("`{k}` debe ser un entero no negativo"))),
    }
}

/// Un numero de la URL: decimal o `0x...`.
fn numero(t: &str) -> Option<u64> {
    t.strip_prefix("0x")
        .map_or_else(|| t.parse().ok(), |h| u64::from_str_radix(h, 16).ok())
}

fn id_de(p: &Peticion) -> Result<u32, Respuesta> {
    p.parametro("id")
        .and_then(|i| i.parse().ok())
        .ok_or_else(|| error(400, "identificador de sesion invalido"))
}

fn con_sesion(ide: &Ide, p: &Peticion, f: impl FnOnce(&mut SesionIde) -> Respuesta) -> Respuesta {
    let id = match id_de(p) {
        Ok(i) => i,
        Err(r) => return r,
    };
    let Some(s) = ide.sesion(id) else {
        return error(404, format!("no existe la sesion {id}"));
    };
    let r = match s.lock() {
        Ok(mut g) => f(&mut g),
        Err(_) => error(500, "la sesion quedo en un estado inconsistente; cierrala"),
    };
    r
}

fn parada(p: &Parada) -> Json {
    match p {
        Parada::Paso { instrucciones } => Json::objeto([
            ("tipo", "paso".into()),
            ("instrucciones", (*instrucciones).into()),
        ]),
        Parada::Detenida => Json::objeto([("tipo", "detenida".into())]),
        Parada::Ruptura { id, direccion } => Json::objeto([
            ("tipo", "ruptura".into()),
            ("id", u64::from(*id).into()),
            ("direccion", (*direccion).into()),
        ]),
        Parada::Observacion { id, valor, .. } => Json::objeto([
            ("tipo", "observacion".into()),
            ("id", u64::from(*id).into()),
            ("valor", valor.map_or(Json::Nulo, Json::from)),
        ]),
        Parada::Llegada { direccion } => Json::objeto([
            ("tipo", "llegada".into()),
            ("direccion", (*direccion).into()),
        ]),
        Parada::Retorno => Json::objeto([("tipo", "retorno".into())]),
        Parada::Limite { instrucciones } => Json::objeto([
            ("tipo", "limite".into()),
            ("instrucciones", (*instrucciones).into()),
        ]),
        Parada::Fallo {
            error,
            direccion,
            instruccion,
        } => Json::objeto([
            ("tipo", "fallo".into()),
            ("mensaje", error.to_string().into()),
            ("direccion", (*direccion).into()),
            (
                "instruccion",
                instruccion.clone().map_or(Json::Nulo, Json::from),
            ),
        ]),
        Parada::CondicionInvalida { id, motivo } => Json::objeto([
            ("tipo", "condicion".into()),
            ("id", u64::from(*id).into()),
            ("mensaje", motivo.clone().into()),
        ]),
    }
}

fn estado(s: &SesionIde) -> Json {
    let e = s.depurador.emulador();
    let st = e.estado();
    let registros: Vec<Json> = st
        .nombres()
        .iter()
        .map(|n| {
            Json::objeto([
                ("nombre", n.as_str().into()),
                ("valor", st.registro(n).map_or(Json::Nulo, Json::from)),
                (
                    "bits",
                    st.mascara_de_registro(n)
                        .map_or(Json::Nulo, |m| u64::from(64 - m.leading_zeros()).into()),
                ),
            ])
        })
        .collect();
    let banderas: Vec<Json> = st
        .nombres_de_banderas()
        .into_iter()
        .map(|n| {
            Json::objeto([
                ("nombre", n.into()),
                ("valor", st.bandera(n).unwrap_or(false).into()),
            ])
        })
        .collect();
    let pc = st.pc();
    let ubic = s.depurador.ubicacion_de(pc).map_or(Json::Nulo, |(a, l)| {
        Json::objeto([("archivo", a.into()), ("linea", l.into())])
    });
    Json::objeto([
        ("pc", pc.into()),
        ("sp", st.sp().map_or(Json::Nulo, Json::from)),
        ("registros", registros.into()),
        ("banderas", banderas.into()),
        ("instrucciones", st.instrucciones().into()),
        ("ciclos", st.ciclos().into()),
        ("detenida", st.detenida().into()),
        ("ubicacion", ubic),
        (
            "etiqueta",
            s.depurador.etiqueta_en(pc).map_or(Json::Nulo, Json::from),
        ),
        ("maquina", s.maquina.as_str().into()),
        ("programa", s.programa.as_str().into()),
        ("dispositivos", dispositivos(s)),
        ("arranque", s.depurador.maquina().arranque.into()),
        ("instante", s.depurador.instante().into()),
        (
            "traza_longitud",
            s.depurador.traza().map_or(0, |t| t.longitud()).into(),
        ),
        (
            "bits_palabra",
            u64::from(s.depurador.maquina().bits_palabra).into(),
        ),
    ])
}

fn dispositivos(s: &SesionIde) -> Json {
    use lorenz_arch::modelo::TipoDispositivo as T;
    s.depurador
        .maquina()
        .dispositivos
        .iter()
        .map(|d| {
            let (tipo, ancho, alto) = match &d.tipo {
                T::Pantalla { ancho, alto, .. } => ("pantalla", *ancho, *alto),
                T::Teclado => ("teclado", 0, 0),
                T::Uart => ("uart", 0, 0),
                T::Temporizador { .. } => ("temporizador", 0, 0),
                T::Bloques { .. } => ("bloques", 0, 0),
                T::Dma => ("dma", 0, 0),
                _ => ("otro", 0, 0),
            };
            Json::objeto([
                ("nombre", d.nombre.as_str().into()),
                ("tipo", tipo.into()),
                ("ancho", u64::from(ancho).into()),
                ("alto", u64::from(alto).into()),
            ])
        })
        .collect::<Vec<_>>()
        .into()
}

fn rupturas(s: &SesionIde) -> Json {
    s.depurador
        .rupturas()
        .iter()
        .map(|r| {
            Json::objeto([
                ("id", u64::from(r.id).into()),
                ("direccion", r.direccion.into()),
                (
                    "condicion",
                    r.condicion
                        .as_ref()
                        .map_or(Json::Nulo, |c| c.texto().into()),
                ),
                ("activa", r.activa.into()),
                ("aciertos", r.aciertos.into()),
            ])
        })
        .collect::<Vec<_>>()
        .into()
}

fn listar(raiz: &std::path::Path, dir: &std::path::Path, fuera: &mut Vec<Json>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entradas: Vec<_> = rd.filter_map(Result::ok).collect();
    entradas.sort_by_key(|e| e.file_name());
    for e in entradas {
        if fuera.len() >= ARCHIVOS_MAX {
            return;
        }
        let nombre = e.file_name().to_string_lossy().into_owned();
        if nombre.starts_with('.') || nombre == "target" {
            continue;
        }
        let Ok(t) = e.file_type() else { continue };
        if t.is_symlink() {
            continue;
        }
        let rel = e
            .path()
            .strip_prefix(raiz)
            .map(|r| r.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        if t.is_dir() {
            fuera.push(Json::objeto([
                ("ruta", rel.clone().into()),
                ("directorio", true.into()),
            ]));
            listar(raiz, &e.path(), fuera);
        } else {
            let tam = e.metadata().map_or(0, |m| m.len());
            fuera.push(Json::objeto([
                ("ruta", rel.into()),
                ("directorio", false.into()),
                ("bytes", tam.into()),
            ]));
        }
    }
}

/// Construye el enrutador del IDE sobre un proyecto.
pub fn router(ide: Arc<Ide>) -> Router {
    let (a, b, c, d, e, f, g, h, i, j, k, l) = (
        ide.clone(),
        ide.clone(),
        ide.clone(),
        ide.clone(),
        ide.clone(),
        ide.clone(),
        ide.clone(),
        ide.clone(),
        ide.clone(),
        ide.clone(),
        ide.clone(),
        ide.clone(),
    );
    let (m1, m2, m3) = (ide.clone(), ide.clone(), ide.clone());
    let para_editor = ide.clone();
    let base = Router::nuevo()
        .get("/", |_| Respuesta::html(super::PAGINA))
        .get("/web/:archivo", |p| {
            let n = p.parametro("archivo").unwrap_or("");
            match super::ESTATICOS.iter().find(|(nombre, _)| *nombre == n) {
                Some((nombre, contenido)) => {
                    Respuesta::nueva(200, super::tipo_de(nombre), *contenido)
                        .con_cabecera("Cache-Control", "no-cache")
                }
                None => Respuesta::texto(404, "no existe"),
            }
        })
        .get("/api/proyecto", move |_| {
            let mut v = Vec::new();
            listar(a.raiz(), a.raiz(), &mut v);
            ok(Json::objeto([("archivos", v.into())]))
        })
        .get("/api/archivo", move |p| {
            let Some(r) = p.consulta("ruta") else {
                return error(400, "falta `ruta`");
            };
            let ruta = match b.resolver(r) {
                Ok(x) => x,
                Err(m) => return error(400, m),
            };
            match std::fs::read(&ruta) {
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(t) => ok(Json::objeto([("ruta", r.into()), ("texto", t.into())])),
                    Err(_) => error(415, format!("`{r}` no es un archivo de texto")),
                },
                Err(e) => error(404, format!("{r}: {e}")),
            }
        })
        .put("/api/archivo", move |p| {
            let j = match cuerpo(p) {
                Ok(j) => j,
                Err(r) => return r,
            };
            let (ruta, texto) = match (texto_de(&j, "ruta"), texto_de(&j, "texto")) {
                (Ok(r), Ok(t)) => (r, t),
                (Err(r), _) | (_, Err(r)) => return r,
            };
            let destino = match c.resolver(ruta) {
                Ok(x) => x,
                Err(m) => return error(400, m),
            };
            if let Some(padre) = destino.parent() {
                if let Err(e) = std::fs::create_dir_all(padre) {
                    return error(500, format!("{ruta}: {e}"));
                }
            }
            match std::fs::write(&destino, texto) {
                Ok(()) => ok(Json::objeto([
                    ("ruta", ruta.into()),
                    ("bytes", texto.len().into()),
                ])),
                Err(e) => error(500, format!("{ruta}: {e}")),
            }
        })
        .delete("/api/archivo", move |p| {
            let Some(r) = p.consulta("ruta") else {
                return error(400, "falta `ruta`");
            };
            let ruta = match d.resolver(r) {
                Ok(x) => x,
                Err(m) => return error(400, m),
            };
            match std::fs::remove_file(&ruta) {
                Ok(()) => ok(Json::objeto([("ruta", r.into())])),
                Err(e) => error(404, format!("{r}: {e}")),
            }
        })
        .post("/api/ensamblar", move |p| {
            let j = match cuerpo(p) {
                Ok(j) => j,
                Err(r) => return r,
            };
            let (m, f) = match (texto_de(&j, "maquina"), texto_de(&j, "fuente")) {
                (Ok(m), Ok(f)) => (m, f),
                (Err(r), _) | (_, Err(r)) => return r,
            };
            let (rm, rf) = match (e.ruta_texto(m), e.ruta_texto(f)) {
                (Ok(a), Ok(b)) => (a, b),
                (Err(m), _) | (_, Err(m)) => return error(400, m),
            };
            let maquina = match lorenz_arch::cargar_archivo(&rm) {
                Ok(x) => x,
                Err(er) => return error(422, format!("{m}: {er}")),
            };
            match super::editor::ensamblar_estructurado(&maquina, &rf) {
                Ok(o) => ok(Json::objeto([
                    ("palabras", o.tamano().into()),
                    (
                        "avisos",
                        o.avisos
                            .iter()
                            .map(|a| Json::texto(a.to_string()))
                            .collect::<Vec<_>>()
                            .into(),
                    ),
                    ("simbolos", super::editor::simbolos(&o, e.raiz())),
                ])),
                Err(er) => Respuesta::json(
                    422,
                    &Json::objeto([
                        ("error", "no se pudo ensamblar".into()),
                        (
                            "errores",
                            er.iter()
                                .map(|x| Json::texto(x.to_string()))
                                .collect::<Vec<_>>()
                                .into(),
                        ),
                        ("diagnosticos", super::editor::diagnosticos(&er, e.raiz())),
                    ])
                    .serializar(),
                ),
            }
        })
        .post("/api/sesiones", move |p| {
            let j = match cuerpo(p) {
                Ok(j) => j,
                Err(r) => return r,
            };
            let (m, pr) = match (texto_de(&j, "maquina"), texto_de(&j, "programa")) {
                (Ok(m), Ok(pr)) => (m, pr),
                (Err(r), _) | (_, Err(r)) => return r,
            };
            match f.abrir(m, pr) {
                Ok(id) => Respuesta::json(
                    201,
                    &Json::objeto([("id", u64::from(id).into())]).serializar(),
                ),
                Err(e) => error(422, e),
            }
        })
        .get("/api/sesiones", move |_| {
            ok(Json::objeto([(
                "sesiones",
                g.ids()
                    .into_iter()
                    .map(|i| Json::from(u64::from(i)))
                    .collect::<Vec<_>>()
                    .into(),
            )]))
        })
        .delete("/api/sesiones/:id", move |p| match id_de(p) {
            Ok(id) if h.cerrar(id) => ok(Json::objeto([("cerrada", u64::from(id).into())])),
            Ok(id) => error(404, format!("no existe la sesion {id}")),
            Err(r) => r,
        })
        .get("/api/sesiones/:id/estado", {
            let ide = i;
            move |p| con_sesion(&ide, p, |s| ok(estado(s)))
        })
        .get("/api/sesiones/:id/memoria", {
            let ide = k;
            move |p| {
                let desde = p.consulta("desde").map_or(Some(0), numero);
                let n = p.consulta("n").map_or(Some(64), numero);
                let (Some(desde), Some(n)) = (desde, n) else {
                    return error(400, "`desde` y `n` deben ser numeros");
                };
                if n > MEMORIA_MAX {
                    return error(400, format!("`n` es como mucho {MEMORIA_MAX}"));
                }
                con_sesion(&ide, p, |s| {
                    let m = s.depurador.emulador().memoria();
                    let celdas: Vec<Json> = (0..n)
                        .map(|i| {
                            desde
                                .checked_add(i)
                                .and_then(|d| m.leer_privilegiado(d).ok())
                                .map_or(Json::Nulo, Json::from)
                        })
                        .collect();
                    ok(Json::objeto([
                        ("desde", desde.into()),
                        ("celdas", celdas.into()),
                    ]))
                })
            }
        })
        .get("/api/sesiones/:id/desensamblado", {
            let ide = m1;
            move |p| {
                let desde = p.consulta("desde").map_or(Some(0), numero);
                let n = p.consulta("n").map_or(Some(16), numero);
                let (Some(desde), Some(n)) = (desde, n) else {
                    return error(400, "`desde` y `n` deben ser numeros");
                };
                if n > 1024 {
                    return error(400, "`n` es como mucho 1024");
                }
                con_sesion(&ide, p, |s| {
                    let pc = s.depurador.emulador().estado().pc();
                    let lineas: Vec<Json> =
                        lorenz_dbg::vistas::desensamblar(&s.depurador, desde, n as usize)
                            .into_iter()
                            .map(|(d, t)| {
                                Json::objeto([
                                    ("direccion", d.into()),
                                    ("texto", t.into()),
                                    (
                                        "etiqueta",
                                        s.depurador.etiqueta_en(d).map_or(Json::Nulo, Json::from),
                                    ),
                                    (
                                        "linea",
                                        s.depurador
                                            .ubicacion_de(d)
                                            .map_or(Json::Nulo, |(_, l)| l.into()),
                                    ),
                                    ("actual", (d == pc).into()),
                                ])
                            })
                            .collect();
                    ok(Json::objeto([("lineas", lineas.into())]))
                })
            }
        })
        .get("/api/sesiones/:id/pantalla/:nombre", {
            let ide = m2;
            move |p| {
                let nombre = p.parametro("nombre").unwrap_or("").to_string();
                con_sesion(&ide, p, |s| {
                    match s.depurador.emulador().pantalla(&nombre) {
                        Ok(img) => ok(Json::objeto([
                            ("ancho", u64::from(img.ancho).into()),
                            ("alto", u64::from(img.alto).into()),
                            (
                                "pixeles",
                                img.pixeles
                                    .iter()
                                    .map(|b| Json::from(u64::from(*b)))
                                    .collect::<Vec<_>>()
                                    .into(),
                            ),
                        ])),
                        Err(e) => error(404, e.to_string()),
                    }
                })
            }
        })
        .get("/api/sesiones/:id/serie/:nombre", {
            let ide = m3;
            move |p| {
                let nombre = p.parametro("nombre").unwrap_or("").to_string();
                con_sesion(&ide, p, |s| {
                    match s.depurador.emulador_mut().texto_de_uart(&nombre) {
                        Ok(t) => ok(Json::objeto([("texto", t.into())])),
                        Err(e) => error(404, e.to_string()),
                    }
                })
            }
        })
        .post("/api/sesiones/:id/serie/:nombre", {
            let ide = ide.clone();
            move |p| {
                let j = match cuerpo(p) {
                    Ok(j) => j,
                    Err(r) => return r,
                };
                let t = match texto_de(&j, "texto") {
                    Ok(t) => t.to_string(),
                    Err(r) => return r,
                };
                let nombre = p.parametro("nombre").unwrap_or("").to_string();
                con_sesion(&ide, p, |s| {
                    match s.depurador.recibir_serie(&nombre, t.as_bytes()) {
                        Ok(()) => ok(estado(s)),
                        Err(e) => error(422, e.to_string()),
                    }
                })
            }
        })
        .get("/api/sesiones/:id/eventos", {
            let ide = ide.clone();
            move |p| {
                con_sesion(&ide, p, |s| {
                    let e = s.depurador.emulador();
                    let ints: Vec<Json> = e
                        .interrupciones_atendidas()
                        .iter()
                        .map(|i| {
                            Json::objeto([
                                ("linea", u64::from(i.linea).into()),
                                ("ciclo", i.ciclo.into()),
                                ("instruccion", i.instruccion.into()),
                                ("pc_interrumpido", i.pc_interrumpido.into()),
                                ("manejador", i.manejador.into()),
                            ])
                        })
                        .collect();
                    ok(Json::objeto([
                        ("interrupciones", ints.into()),
                        ("sin_registrar", e.interrupciones_sin_registrar().into()),
                        (
                            "pendientes",
                            e.interrupciones_pendientes()
                                .into_iter()
                                .map(|l| Json::from(u64::from(l)))
                                .collect::<Vec<_>>()
                                .into(),
                        ),
                    ]))
                })
            }
        })
        .get("/api/sesiones/:id/explicacion", {
            let ide = ide.clone();
            move |p| {
                con_sesion(&ide, p, |s| match s.depurador.explicar() {
                    Ok(x) => ok(Json::objeto([
                        ("direccion", x.direccion.into()),
                        ("texto", x.texto.into()),
                        ("descripcion", x.descripcion.into()),
                        (
                            "operaciones",
                            x.operaciones
                                .into_iter()
                                .map(Json::from)
                                .collect::<Vec<_>>()
                                .into(),
                        ),
                        (
                            "registros",
                            x.registros
                                .iter()
                                .map(|r| {
                                    Json::objeto([
                                        ("nombre", r.nombre.as_str().into()),
                                        ("antes", r.antes.into()),
                                        ("despues", r.despues.into()),
                                    ])
                                })
                                .collect::<Vec<_>>()
                                .into(),
                        ),
                        (
                            "escrituras",
                            x.escrituras
                                .iter()
                                .map(|w| {
                                    Json::objeto([
                                        ("direccion", w.direccion.into()),
                                        ("antes", w.antes.into()),
                                        ("despues", w.despues.into()),
                                    ])
                                })
                                .collect::<Vec<_>>()
                                .into(),
                        ),
                        ("salto", x.salto.map_or(Json::Nulo, Json::from)),
                        ("pc_despues", x.pc_despues.into()),
                        ("ciclos", x.ciclos.into()),
                        ("detiene", x.detiene.into()),
                    ])),
                    Err(e) => error(422, e.to_string()),
                })
            }
        })
        .get("/api/sesiones/:id/rupturas", {
            let ide = l;
            move |p| con_sesion(&ide, p, |s| ok(Json::objeto([("rupturas", rupturas(s))])))
        })
        .post("/api/sesiones/:id/rupturas", {
            let ide = ide.clone();
            move |p| {
                let j = match cuerpo(p) {
                    Ok(j) => j,
                    Err(r) => return r,
                };
                let dir = match j.get("direccion") {
                    Some(Json::Texto(t)) => Some(t.clone()),
                    Some(v) => v.como_entero().map(|n| n.to_string()),
                    None => None,
                };
                let Some(dir) = dir else {
                    return error(400, "falta `direccion`");
                };
                let cond = j
                    .get("condicion")
                    .and_then(Json::como_texto)
                    .map(str::to_string);
                con_sesion(&ide, p, |s| {
                    let direccion = match s.depurador.resolver(&dir) {
                        Ok(d) => d,
                        Err(e) => return error(422, e.to_string()),
                    };
                    match s.depurador.poner_ruptura(direccion, cond.as_deref()) {
                        Ok(id) => Respuesta::json(
                            201,
                            &Json::objeto([
                                ("id", u64::from(id).into()),
                                ("direccion", direccion.into()),
                            ])
                            .serializar(),
                        ),
                        Err(e) => error(422, e.to_string()),
                    }
                })
            }
        })
        .delete("/api/sesiones/:id/rupturas/:ruptura", {
            let ide = ide;
            move |p| {
                let Some(r) = p.parametro("ruptura").and_then(|r| r.parse::<u32>().ok()) else {
                    return error(400, "identificador de ruptura invalido");
                };
                con_sesion(&ide, p, |s| match s.depurador.borrar(r) {
                    Ok(()) => ok(Json::objeto([("borrada", u64::from(r).into())])),
                    Err(e) => error(404, e.to_string()),
                })
            }
        })
        .post("/api/sesiones/:id/:accion", {
            let ide = j;
            move |p| {
                let j = match cuerpo(p) {
                    Ok(j) => j,
                    Err(r) => return r,
                };
                let accion = p.parametro("accion").unwrap_or("").to_string();
                con_sesion(&ide, p, |s| {
                    let resultado = match accion.as_str() {
                        "paso" => {
                            num_de(&j, "n", 1).map(|n| s.depurador.paso(n.clamp(1, LIMITE_PASO)))
                        }
                        "continuar" => num_de(&j, "limite", LIMITE_PASO)
                            .map(|l| s.depurador.continuar(l.min(LIMITE_PASO))),
                        "hasta" => (|| {
                            let dir = num_de(&j, "direccion", u64::MAX)?;
                            if dir == u64::MAX {
                                return Err(error(400, "falta `direccion`"));
                            }
                            Ok(s.depurador
                                .hasta(dir, num_de(&j, "limite", LIMITE_PASO)?.min(LIMITE_PASO)))
                        })(),
                        "reiniciar" => {
                            let inicio = s.depurador.maquina().arranque;
                            s.depurador.emulador_mut().reiniciar();
                            s.depurador.emulador_mut().estado_mut().fijar_pc(inicio);
                            return ok(estado(s));
                        }
                        "atras" | "adelante" => {
                            let n = match num_de(&j, "n", 1) {
                                Ok(n) => n,
                                Err(r) => return r,
                            };
                            let r = if accion == "atras" {
                                s.depurador.atras(n)
                            } else {
                                s.depurador.adelante(n)
                            };
                            return match r {
                                Ok(()) => ok(estado(s)),
                                Err(e) => error(422, e.to_string()),
                            };
                        }
                        "instante" => {
                            let n = match num_de(&j, "n", u64::MAX) {
                                Ok(n) if n != u64::MAX => n,
                                Ok(_) => return error(400, "falta `n`"),
                                Err(r) => return r,
                            };
                            return match s.depurador.ir_a(n) {
                                Ok(()) => ok(estado(s)),
                                Err(e) => error(422, e.to_string()),
                            };
                        }
                        "orden" => {
                            let linea = match texto_de(&j, "linea") {
                                Ok(l) => l.to_string(),
                                Err(r) => return r,
                            };
                            let r = s.sesion.ejecutar(&linea);
                            return ok(Json::objeto([
                                ("texto", r.texto.into()),
                                ("salir", r.salir.into()),
                                ("estado", estado(s)),
                            ]));
                        }
                        "grabar" => {
                            s.depurador
                                .grabar(j.get("activa").and_then(Json::como_bool).unwrap_or(true));
                            return ok(estado(s));
                        }
                        "registro" | "memoria" | "puerto" => {
                            let (a, v) = match (
                                num_de(
                                    &j,
                                    if accion == "registro" {
                                        "valor"
                                    } else {
                                        "direccion"
                                    },
                                    u64::MAX,
                                ),
                                num_de(&j, "valor", u64::MAX),
                            ) {
                                (Ok(a), Ok(v)) => (a, v),
                                (Err(r), _) | (_, Err(r)) => return r,
                            };
                            if v == u64::MAX {
                                return error(400, "falta `valor`");
                            }
                            let r = match accion.as_str() {
                                "registro" => match texto_de(&j, "nombre") {
                                    Ok(n) => s.depurador.fijar_registro(n, v),
                                    Err(r) => return r,
                                },
                                "memoria" if a == u64::MAX => {
                                    return error(400, "falta `direccion`")
                                }
                                "memoria" => s.depurador.escribir_memoria(a, v),
                                _ if a == u64::MAX => return error(400, "falta `direccion`"),
                                _ => s.depurador.escribir_puerto(a, v),
                            };
                            return match r {
                                Ok(()) => ok(estado(s)),
                                Err(e) => error(422, e.to_string()),
                            };
                        }
                        _ => return error(404, format!("accion desconocida `{accion}`")),
                    };
                    match resultado {
                        Ok(pa) => ok(Json::objeto([
                            ("parada", parada(&pa)),
                            ("estado", estado(s)),
                        ])),
                        Err(r) => r,
                    }
                })
            }
        });
    super::editor::rutas(base, para_editor)
}
