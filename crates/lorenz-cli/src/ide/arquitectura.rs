//! API del editor visual de arquitectura (RF-ARQ-038 a 041).
//!
//! El texto del archivo es la fuente de la verdad: `analizar` lo convierte en bloques para el
//! diagrama (texto a diagrama) y `editar` aplica un cambio del diagrama sobre el texto conservando
//! comentarios y formato (diagrama a texto). Ambas devuelven los fallos del cargador con su linea y
//! el bloque que los contiene, para senalarlos en el diagrama.

use std::path::Path;
use std::sync::Arc;

use lorenz_arch::editar::{Bloque, Cambio, Diagnostico, Documento, Seg, Valor};
use lorenz_web::{Json, Peticion, Respuesta, Router};

use super::api::{cuerpo, error, ok, texto_de};
use super::sesion::Ide;

fn valor_json(v: &Valor) -> Json {
    match v {
        Valor::Entero(n) => Json::Num(*n as f64),
        Valor::Texto(t) => Json::texto(t.as_str()),
        Valor::Bool(b) => Json::Bool(*b),
    }
}

fn valor_de(j: &Json) -> Option<Valor> {
    match j {
        Json::Bool(b) => Some(Valor::Bool(*b)),
        Json::Texto(t) => Some(Valor::Texto(t.clone())),
        Json::Num(_) => j.como_entero().map(Valor::Entero),
        _ => None,
    }
}

fn bloque_json(i: usize, b: &Bloque) -> Json {
    Json::objeto([
        ("indice", (i as u64).into()),
        ("clase", b.clase.nombre().into()),
        ("nombre", b.nombre.as_str().into()),
        ("linea", (b.linea as u64).into()),
        ("fin", (b.fin as u64).into()),
        (
            "ruta",
            b.ruta
                .iter()
                .map(|s| match s {
                    Seg::Clave(k) => Json::texto(k.as_str()),
                    Seg::Indice(n) => Json::Num(*n as f64),
                })
                .collect::<Vec<_>>()
                .into(),
        ),
        (
            "campos",
            b.campos
                .iter()
                .map(|c| {
                    Json::objeto([
                        ("clave", c.clave.as_str().into()),
                        ("valor", c.valor.as_ref().map_or(Json::Nulo, valor_json)),
                        ("texto", c.texto.as_str().into()),
                    ])
                })
                .collect::<Vec<_>>()
                .into(),
        ),
    ])
}

fn diagnostico_json(d: &Diagnostico, bloques: &[Bloque]) -> Json {
    // El bloque mas pequeno que contiene la linea: una bandera dentro de `[flags]`, no `[flags]`.
    let bloque = bloques
        .iter()
        .enumerate()
        .filter(|(_, b)| b.linea <= d.linea && d.linea <= b.fin)
        .min_by_key(|(_, b)| b.fin - b.linea)
        .map_or(Json::Nulo, |(i, _)| Json::from(i as u64));
    Json::objeto([
        ("linea", (d.linea as u64).into()),
        ("columna", (d.columna as u64).into()),
        ("codigo", d.codigo.as_str().into()),
        ("mensaje", d.mensaje.as_str().into()),
        ("bloque", bloque),
    ])
}

/// La respuesta comun: bloques y fallos del texto dado.
fn vista(ide: &Ide, doc: &Documento, texto: &str, ruta: Option<&str>) -> Vec<(&'static str, Json)> {
    let bloques = doc.bloques();
    let propia = ruta.and_then(|r| ide.resolver(r).ok());
    let carpeta = propia.as_deref().and_then(Path::parent);
    let fallos = lorenz_arch::editar::diagnosticos(texto, carpeta, propia.as_deref());
    vec![
        (
            "bloques",
            bloques
                .iter()
                .enumerate()
                .map(|(i, b)| bloque_json(i, b))
                .collect::<Vec<_>>()
                .into(),
        ),
        (
            "errores",
            fallos
                .iter()
                .map(|d| diagnostico_json(d, &bloques))
                .collect::<Vec<_>>()
                .into(),
        ),
        ("valida", fallos.is_empty().into()),
    ]
}

fn objeto(pares: Vec<(&'static str, Json)>) -> Json {
    Json::Objeto(pares.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn ruta_de(j: &Json) -> Result<Vec<Seg>, String> {
    j.como_lista()
        .ok_or("`ruta` debe ser una lista")?
        .iter()
        .map(|s| match s {
            Json::Texto(t) => Ok(Seg::Clave(t.clone())),
            Json::Num(_) => s
                .como_entero()
                .filter(|n| *n >= 0)
                .map(|n| Seg::Indice(n as usize))
                .ok_or_else(|| "indice invalido en `ruta`".to_string()),
            _ => Err("`ruta` solo admite textos y numeros".to_string()),
        })
        .collect()
}

fn cambio_de(j: &Json) -> Result<Cambio, String> {
    let tipo = j
        .get("tipo")
        .and_then(Json::como_texto)
        .ok_or("falta `cambio.tipo`")?;
    match tipo {
        "fijar" => Ok(Cambio::Fijar {
            ruta: ruta_de(j.get("ruta").ok_or("falta `ruta`")?)?,
            valor: j
                .get("valor")
                .and_then(valor_de)
                .ok_or("falta `valor` (texto, entero o booleano)")?,
        }),
        "quitar" => Ok(Cambio::Quitar {
            ruta: ruta_de(j.get("ruta").ok_or("falta `ruta`")?)?,
        }),
        "anadir" => {
            let lista = j
                .get("lista")
                .and_then(Json::como_texto)
                .ok_or("falta `lista`")?
                .to_string();
            let campos = j
                .get("campos")
                .and_then(Json::como_lista)
                .ok_or("falta `campos` (lista de {clave, valor})")?
                .iter()
                .map(|c| {
                    let k = c
                        .get("clave")
                        .and_then(Json::como_texto)
                        .ok_or("falta `clave`")?;
                    let v = c
                        .get("valor")
                        .and_then(valor_de)
                        .ok_or("`valor` invalido")?;
                    Ok((k.to_string(), v))
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(Cambio::Anadir { lista, campos })
        }
        otro => Err(format!("cambio desconocido: `{otro}`")),
    }
}

fn pedir(p: &Peticion) -> Result<(Json, String), Respuesta> {
    let j = cuerpo(p)?;
    let t = texto_de(&j, "texto")?.to_string();
    Ok((j, t))
}

pub fn rutas(router: Router, ide: Arc<Ide>) -> Router {
    let (a, b) = (ide.clone(), ide);
    router
        .post("/api/arquitectura/analizar", move |p| {
            let (j, texto) = match pedir(p) {
                Ok(x) => x,
                Err(r) => return r,
            };
            let ruta = j.get("ruta").and_then(Json::como_texto);
            match Documento::analizar(&texto) {
                Ok(doc) => ok(objeto(vista(&a, &doc, &texto, ruta))),
                Err(e) => {
                    // Sin TOML valido no hay diagrama, pero si el fallo con su linea.
                    let propia = ruta.and_then(|r| a.resolver(r).ok());
                    let fallos = lorenz_arch::editar::diagnosticos(
                        &texto,
                        propia.as_deref().and_then(Path::parent),
                        propia.as_deref(),
                    );
                    ok(objeto(vec![
                        ("bloques", Json::Lista(vec![])),
                        (
                            "errores",
                            fallos
                                .iter()
                                .map(|d| diagnostico_json(d, &[]))
                                .collect::<Vec<_>>()
                                .into(),
                        ),
                        ("valida", false.into()),
                        ("aviso", e.0.into()),
                    ]))
                }
            }
        })
        .post("/api/arquitectura/borrador", move |p| {
            let j = match cuerpo(p) {
                Ok(j) => j,
                Err(r) => return r,
            };
            let d = match texto_de(&j, "descripcion") {
                Ok(d) => d,
                Err(r) => return r,
            };
            match lorenz_arch::borrador::desde_descripcion(d) {
                Ok(b) => ok(objeto(vec![
                    ("texto", b.texto.into()),
                    (
                        "supuestos",
                        b.supuestos
                            .iter()
                            .map(|s| Json::texto(s.as_str()))
                            .collect::<Vec<_>>()
                            .into(),
                    ),
                ])),
                Err(e) => error(422, e),
            }
        })
        .post("/api/arquitectura/editar", move |p| {
            let (j, texto) = match pedir(p) {
                Ok(x) => x,
                Err(r) => return r,
            };
            let ruta = j.get("ruta").and_then(Json::como_texto);
            let cambio = match j
                .get("cambio")
                .ok_or("falta `cambio`".to_string())
                .and_then(cambio_de)
            {
                Ok(c) => c,
                Err(e) => return error(400, e),
            };
            let mut doc = match Documento::analizar(&texto) {
                Ok(d) => d,
                Err(e) => return error(422, e.0),
            };
            if let Err(e) = doc.aplicar(&cambio) {
                return error(422, e.0);
            }
            let nuevo = doc.texto();
            // Los bloques y lineas se calculan sobre el texto ya cambiado.
            let doc = match Documento::analizar(&nuevo) {
                Ok(d) => d,
                Err(e) => return error(500, e.0),
            };
            let mut v = vista(&b, &doc, &nuevo, ruta);
            v.push(("texto", nuevo.into()));
            ok(objeto(v))
        })
}
