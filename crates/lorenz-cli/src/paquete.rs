//! `lorenz paquete`: crear, verificar, inspeccionar, instalar y listar paquetes (RF-ECO-001 a
//! RF-ECO-003, RF-ECO-005 a RF-ECO-008).

use std::io::Write;
use std::path::PathBuf;

use lorenz_paq::{Almacen, Paquete};

use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_OK};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionPaquete {
    Crear {
        carpeta: String,
        salida: Option<String>,
    },
    Verificar {
        archivo: String,
    },
    Info {
        archivo: String,
    },
    Instalar {
        archivo: String,
    },
    Listar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcionesPaquete {
    /// Raiz del almacen; por omision `$LORENZ_PAQUETES` o `~/.lorenz/paquetes`.
    pub en: Option<String>,
    pub repositorios: Vec<String>,
    /// Huella SHA-256 que debe tener el paquete; obligatoria si se instala desde una direccion web.
    pub sha256: Option<String>,
}

pub fn raiz_por_omision() -> PathBuf {
    if let Some(r) = std::env::var_os("LORENZ_PAQUETES") {
        return PathBuf::from(r);
    }
    let casa = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .unwrap_or_default();
    PathBuf::from(casa).join(".lorenz").join("paquetes")
}

fn leer(archivo: &str) -> Result<Paquete, String> {
    let bytes = std::fs::read(archivo).map_err(|e| format!("{archivo}: {e}"))?;
    Paquete::desde_bytes(&bytes).map_err(|e| format!("{archivo}: {e}"))
}

/// Lee el paquete de un archivo o de una direccion `http://`, comprobando la huella si se dio.
///
/// Desde la red la huella es obligatoria: es lo unico que garantiza que el paquete es el esperado
/// (no hay TLS, decision D-115).
fn leer_origen(origen: &str, huella: Option<&str>) -> Result<Paquete, String> {
    let bytes = if lorenz_paq::descarga::es_url(origen) {
        if huella.is_none() {
            return Err(format!(
                "{origen}: para instalar desde la red hay que dar la huella con --sha256 (el paquete no se descarga por un canal cifrado)"
            ));
        }
        lorenz_paq::descarga::descargar(origen)?
    } else {
        std::fs::read(origen).map_err(|e| format!("{origen}: {e}"))?
    };
    if let Some(esperada) = huella {
        let real = lorenz_paq::sha256_hex(&bytes);
        if !real.eq_ignore_ascii_case(esperada.trim()) {
            return Err(format!(
                "{origen}: la huella SHA-256 no coincide\n  esperada: {}\n  real:     {real}",
                esperada.trim().to_ascii_lowercase()
            ));
        }
    }
    Paquete::desde_bytes(&bytes).map_err(|e| format!("{origen}: {e}"))
}

pub fn paquete_cli(
    accion: &AccionPaquete,
    opciones: &OpcionesPaquete,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let r: Result<(), String> = (|| {
        let w = |e: std::io::Error| e.to_string();
        let almacen = || {
            Almacen::nuevo(
                opciones
                    .en
                    .as_ref()
                    .map_or_else(raiz_por_omision, PathBuf::from),
            )
        };
        match accion {
            AccionPaquete::Crear {
                carpeta,
                salida: destino,
            } => {
                let p = Paquete::desde_carpeta(carpeta).map_err(|e| e.to_string())?;
                let m = &p.manifiesto;
                let destino = destino
                    .clone()
                    .unwrap_or_else(|| format!("{}-{}.lpkg", m.nombre, m.version));
                let bytes = p.a_bytes();
                std::fs::write(&destino, &bytes).map_err(|e| format!("{destino}: {e}"))?;
                writeln!(
                    salida,
                    "{destino}: {} {} ({} archivos, {} bytes)",
                    m.nombre,
                    m.version,
                    p.archivos.len(),
                    bytes.len()
                )
                .map_err(w)?;
            }
            AccionPaquete::Verificar { archivo } => {
                let p = leer(archivo)?;
                writeln!(
                    salida,
                    "{archivo}: correcto ({} {}, {} archivos)",
                    p.manifiesto.nombre,
                    p.manifiesto.version,
                    p.archivos.len()
                )
                .map_err(w)?;
                for a in p.avisos() {
                    writeln!(errores, "aviso: {a}").map_err(w)?;
                }
            }
            AccionPaquete::Info { archivo } => {
                let p = leer(archivo)?;
                let m = &p.manifiesto;
                writeln!(salida, "{} {}", m.nombre, m.version).map_err(w)?;
                writeln!(salida, "  tipo:      {}", m.tipo.nombre()).map_err(w)?;
                writeln!(salida, "  autor:     {}", m.autor).map_err(w)?;
                writeln!(salida, "  licencia:  {}", m.licencia).map_err(w)?;
                if !m.descripcion.is_empty() {
                    writeln!(salida, "  resumen:   {}", m.descripcion).map_err(w)?;
                }
                for (d, r) in &m.dependencias {
                    writeln!(salida, "  requiere:  {d} {r}").map_err(w)?;
                }
                for (ruta, datos) in &p.archivos {
                    writeln!(salida, "  {:>9}  {ruta}", datos.len()).map_err(w)?;
                }
            }
            AccionPaquete::Instalar { archivo } => {
                let p = leer_origen(archivo, opciones.sha256.as_deref())?;
                let repos: Vec<PathBuf> = opciones.repositorios.iter().map(PathBuf::from).collect();
                let inf = almacen().instalar(&p, &repos).map_err(|e| e.to_string())?;
                for (n, v) in &inf.instalados {
                    writeln!(salida, "instalado {n} {v}").map_err(w)?;
                }
                for (n, v) in &inf.ya_estaban {
                    writeln!(salida, "ya estaba {n} {v}").map_err(w)?;
                }
                for a in &inf.avisos {
                    writeln!(errores, "aviso: {a}").map_err(w)?;
                }
            }
            AccionPaquete::Listar => {
                let v = almacen().instalados();
                if v.is_empty() {
                    writeln!(salida, "no hay paquetes instalados").map_err(w)?;
                }
                for m in v {
                    writeln!(salida, "{} {} ({})", m.nombre, m.version, m.tipo.nombre())
                        .map_err(w)?;
                }
            }
        }
        Ok(())
    })();
    match r {
        Ok(()) => SALIDA_OK,
        Err(e) => {
            let _ = writeln!(errores, "error: {e}");
            SALIDA_ERROR_EJECUCION
        }
    }
}
