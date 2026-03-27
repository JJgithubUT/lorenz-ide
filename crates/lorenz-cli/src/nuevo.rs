//! La orden `nuevo`: crea un proyecto copiando una carpeta plantilla (RF-ECO-042).

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};

fn escribir(w: &mut dyn Write, texto: &str) {
    let _ = w.write_all(texto.as_bytes());
}

/// Archivos de la plantilla, con su ruta relativa; salta lo oculto y las carpetas `target`.
fn archivos(raiz: &Path, relativa: &Path, fuera: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entradas: Vec<_> =
        std::fs::read_dir(raiz.join(relativa))?.collect::<Result<Vec<_>, _>>()?;
    entradas.sort_by_key(|e| e.file_name());
    for e in entradas {
        let nombre = e.file_name();
        let texto = nombre.to_string_lossy();
        if texto.starts_with('.') || texto == "target" {
            continue;
        }
        let ruta = relativa.join(&nombre);
        if e.file_type()?.is_dir() {
            archivos(raiz, &ruta, fuera)?;
        } else {
            fuera.push(ruta);
        }
    }
    Ok(())
}

/// Cambia el nombre de la maquina en la primera clave `nombre` de `[maquina]`.
pub fn renombrar_maquina(texto: &str, nuevo: &str) -> String {
    let mut dentro = false;
    let mut hecho = false;
    let mut salida = String::with_capacity(texto.len());
    for linea in texto.split_inclusive('\n') {
        let t = linea.trim_start();
        if t.starts_with('[') {
            dentro = t.starts_with("[maquina]");
        }
        if dentro && !hecho && t.starts_with("nombre") {
            if let Some((clave, _)) = linea.split_once('=') {
                let fin = if linea.ends_with('\n') { "\n" } else { "" };
                salida.push_str(&format!("{clave}= \"{nuevo}\"{fin}"));
                hecho = true;
                continue;
            }
        }
        salida.push_str(linea);
    }
    salida
}

pub fn nuevo_cli(
    plantilla: &str,
    destino: &str,
    nombre: Option<&str>,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let origen = Path::new(plantilla);
    if !origen.join("maquina.toml").is_file() {
        escribir(
            errores,
            &format!("{plantilla}: no es una plantilla: falta `maquina.toml`\n"),
        );
        return SALIDA_ERROR_USO;
    }
    let destino_ruta = Path::new(destino);
    if destino_ruta.exists()
        && destino_ruta
            .read_dir()
            .map(|mut d| d.next().is_some())
            .unwrap_or(true)
    {
        escribir(
            errores,
            &format!("{destino}: ya existe y no esta vacia; elige otra carpeta\n"),
        );
        return SALIDA_ERROR_USO;
    }
    let mut lista = Vec::new();
    if let Err(e) = archivos(origen, Path::new(""), &mut lista) {
        escribir(errores, &format!("{plantilla}: no se pudo leer: {e}\n"));
        return SALIDA_ERROR_EJECUCION;
    }
    for rel in &lista {
        let (de, a) = (origen.join(rel), destino_ruta.join(rel));
        if let Some(padre) = a.parent() {
            if let Err(e) = std::fs::create_dir_all(padre) {
                escribir(errores, &format!("{}: {e}\n", padre.display()));
                return SALIDA_ERROR_EJECUCION;
            }
        }
        let copiado = match (nombre, rel.file_name().and_then(|n| n.to_str())) {
            (Some(n), Some("maquina.toml")) => std::fs::read_to_string(&de)
                .and_then(|t| std::fs::write(&a, renombrar_maquina(&t, n))),
            _ => std::fs::copy(&de, &a).map(|_| ()),
        };
        if let Err(e) = copiado {
            escribir(errores, &format!("{}: {e}\n", a.display()));
            return SALIDA_ERROR_EJECUCION;
        }
        escribir(salida, &format!("{}\n", a.display()));
    }
    escribir(
        salida,
        &format!("proyecto nuevo en {destino} ({} archivos)\n", lista.len()),
    );
    SALIDA_OK
}
