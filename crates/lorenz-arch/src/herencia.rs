//! Herencia entre descripciones de arquitectura (RF-ARQ-035).
//!
//! Una descripcion puede partir de otra con `hereda = "otra.toml"` en `[maquina]` y declarar solo
//! lo que cambia. La ruta es relativa a la carpeta del archivo que hereda. La mezcla se hace sobre
//! el TOML, antes de construir la maquina, de modo que el resto del cargador no sabe que existe:
//!
//! - Las claves de `[maquina]` del hijo sustituyen a las del padre.
//! - `[flags]` se mezcla por nombre de bandera.
//! - Las listas `[[registro]]`, `[[par]]`, `[[instruccion]]`, `[[region]]` y `[[dispositivo]]` se
//!   mezclan por nombre (`mnemonico` en las instrucciones): un elemento del hijo con el mismo nombre
//!   sustituye al del padre en su sitio, uno nuevo se anade al final, y uno con `eliminar = true`
//!   quita el del padre. Un elemento se sustituye entero, no clave a clave.

use std::path::{Path, PathBuf};

use toml::value::Table;
use toml::Value;

use crate::error::ErrorArquitectura;

/// Cuantos niveles de herencia se admiten, para que una cadena absurda falle con un mensaje claro.
pub const PROFUNDIDAD_MAXIMA: usize = 8;

/// Listas que se mezclan por nombre, con la clave que identifica a cada elemento.
const LISTAS: &[(&str, &str)] = &[
    ("registro", "nombre"),
    ("par", "nombre"),
    ("instruccion", "mnemonico"),
    ("region", "nombre"),
    ("dispositivo", "nombre"),
];

fn error(mensaje: impl Into<String>) -> ErrorArquitectura {
    ErrorArquitectura::Herencia(mensaje.into())
}

/// Lee `texto` y, si hereda de otra descripcion, la mezcla con ella. `carpeta` es donde se buscan
/// los padres; `visitados` lleva los archivos de la cadena para detectar ciclos.
pub fn resolver(
    texto: &str,
    carpeta: Option<&Path>,
    visitados: &mut Vec<PathBuf>,
) -> Result<Table, ErrorArquitectura> {
    let mut hijo: Table = toml::from_str(texto)?;
    let hereda = hijo
        .get_mut("maquina")
        .and_then(Value::as_table_mut)
        .and_then(|m| m.remove("hereda"));
    let Some(hereda) = hereda else {
        return Ok(hijo);
    };
    let Some(ruta_padre) = hereda.as_str() else {
        return Err(error(
            "`hereda` debe ser el nombre de un archivo entre comillas",
        ));
    };
    let Some(carpeta) = carpeta else {
        return Err(error(format!(
            "la descripcion hereda de `{ruta_padre}`, pero se cargo desde texto y no hay carpeta donde buscarla"
        )));
    };
    if visitados.len() >= PROFUNDIDAD_MAXIMA {
        return Err(error(format!(
            "la cadena de herencia pasa de {PROFUNDIDAD_MAXIMA} niveles"
        )));
    }
    let ruta = carpeta.join(ruta_padre);
    let canonica = ruta.canonicalize().unwrap_or_else(|_| ruta.clone());
    if visitados.contains(&canonica) {
        return Err(error(format!(
            "`{ruta_padre}` forma un ciclo de herencia consigo misma"
        )));
    }
    let texto_padre = std::fs::read_to_string(&ruta).map_err(|e| {
        error(format!(
            "no se pudo leer la descripcion base `{ruta_padre}`: {e}"
        ))
    })?;
    visitados.push(canonica);
    let padre = resolver(&texto_padre, ruta.parent(), visitados)?;
    visitados.pop();
    Ok(mezclar(padre, hijo))
}

fn mezclar(mut base: Table, hijo: Table) -> Table {
    for (clave, valor) in hijo {
        match (clave.as_str(), valor) {
            ("maquina" | "flags", Value::Table(t)) => {
                let destino = base
                    .entry(clave)
                    .or_insert_with(|| Value::Table(Table::new()));
                if let Some(d) = destino.as_table_mut() {
                    // Una clave del hijo sustituye a la del padre, sea cual sea su valor.
                    for (k, v) in t {
                        d.insert(k, v);
                    }
                }
            }
            (nombre, Value::Array(elementos)) if LISTAS.iter().any(|(l, _)| *l == nombre) => {
                let identidad = LISTAS.iter().find(|(l, _)| *l == nombre).unwrap().1;
                let destino = base
                    .entry(clave)
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Some(lista) = destino.as_array_mut() {
                    for e in elementos {
                        mezclar_elemento(lista, e, identidad);
                    }
                }
            }
            (_, v) => {
                base.insert(clave, v);
            }
        }
    }
    base
}

fn mezclar_elemento(lista: &mut Vec<Value>, mut nuevo: Value, identidad: &str) {
    let nombre = nuevo
        .get(identidad)
        .and_then(Value::as_str)
        .map(str::to_string);
    let eliminar = nuevo
        .get("eliminar")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if let Some(t) = nuevo.as_table_mut() {
        t.remove("eliminar");
    }
    let posicion = nombre.as_deref().and_then(|n| {
        lista
            .iter()
            .position(|e| e.get(identidad).and_then(Value::as_str) == Some(n))
    });
    match (posicion, eliminar) {
        (Some(i), true) => {
            lista.remove(i);
        }
        (Some(i), false) => lista[i] = nuevo,
        (None, true) => {}
        (None, false) => lista.push(nuevo),
    }
}

/// Archivos que intervienen en una descripcion: ella misma y su cadena de padres, en ese orden.
///
/// Sirve para saber si hay que recargar: basta con que cambie uno. Si la cadena esta rota (un
/// padre que no existe, un ciclo) se corta ahi; el error lo dara `cargar_archivo`.
pub fn cadena(ruta: &Path) -> Vec<PathBuf> {
    let mut archivos = vec![ruta.to_path_buf()];
    let mut actual = ruta.to_path_buf();
    while archivos.len() <= PROFUNDIDAD_MAXIMA {
        let Ok(texto) = std::fs::read_to_string(&actual) else {
            break;
        };
        let Some(padre) = toml::from_str::<Table>(&texto).ok().and_then(|t| {
            t.get("maquina")?
                .get("hereda")?
                .as_str()
                .map(|p| actual.parent().unwrap_or(Path::new("")).join(p))
        }) else {
            break;
        };
        if archivos.contains(&padre) {
            break;
        }
        archivos.push(padre.clone());
        actual = padre;
    }
    archivos
}
