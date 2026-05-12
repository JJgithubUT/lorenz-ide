//! La lista de proyectos recientes (RF-IDE-009).
//!
//! Se guarda en `recientes.toml`, dentro de `$LORENZ_HOME` o, si no existe, de `~/.lorenz`. Lo mas
//! reciente va primero y solo se conservan [`MAXIMO`] entradas.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const MAXIMO: usize = 10;
const ARCHIVO: &str = "recientes.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Entrada {
    ruta: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Contenido {
    #[serde(default, rename = "reciente")]
    recientes: Vec<Entrada>,
}

/// Un proyecto de la lista, con lo que se sabe de el en este momento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reciente {
    pub ruta: String,
    pub nombre: String,
    pub existe: bool,
}

#[derive(Debug, Clone)]
pub struct Recientes {
    archivo: PathBuf,
}

/// La carpeta de datos del usuario: `$LORENZ_HOME` o `~/.lorenz`.
pub fn carpeta_de_usuario() -> PathBuf {
    if let Some(h) = std::env::var_os("LORENZ_HOME") {
        return PathBuf::from(h);
    }
    let casa = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .unwrap_or_default();
    PathBuf::from(casa).join(".lorenz")
}

impl Recientes {
    pub fn por_omision() -> Self {
        Self::en(carpeta_de_usuario().join(ARCHIVO))
    }

    pub fn en(archivo: impl Into<PathBuf>) -> Self {
        Self {
            archivo: archivo.into(),
        }
    }

    fn leer(&self) -> Vec<Entrada> {
        std::fs::read_to_string(&self.archivo)
            .ok()
            .and_then(|t| toml::from_str::<Contenido>(&t).ok())
            .map(|c| c.recientes)
            .unwrap_or_default()
    }

    fn guardar(&self, v: Vec<Entrada>) -> std::io::Result<()> {
        let texto = toml::to_string(&Contenido { recientes: v })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        if let Some(d) = self.archivo.parent() {
            std::fs::create_dir_all(d)?;
        }
        // Se escribe en un archivo aparte y se renombra: nunca queda una lista a medias.
        let tmp = self
            .archivo
            .with_extension(format!("tmp{}", std::process::id()));
        std::fs::write(&tmp, texto)?;
        std::fs::rename(&tmp, &self.archivo)
    }

    /// Pone un proyecto al principio de la lista. `ruta` es la carpeta o su `proyecto.toml`.
    pub fn agregar(&self, ruta: &Path) -> std::io::Result<()> {
        let carpeta = if ruta.is_dir() {
            ruta.to_path_buf()
        } else {
            ruta.parent()
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
        };
        let absoluta = std::fs::canonicalize(&carpeta)?;
        let texto = absoluta.to_string_lossy().into_owned();
        let mut v = self.leer();
        v.retain(|e| e.ruta != texto);
        v.insert(0, Entrada { ruta: texto });
        v.truncate(MAXIMO);
        self.guardar(v)
    }

    /// La lista, de la mas reciente a la mas antigua.
    pub fn listar(&self) -> Vec<Reciente> {
        self.leer()
            .into_iter()
            .map(|e| {
                let p = Path::new(&e.ruta);
                let nombre = std::fs::read_to_string(p.join(crate::proyecto::NOMBRE_ARCHIVO))
                    .ok()
                    .and_then(|t| crate::proyecto::Proyecto::leer(&t).ok())
                    .map(|p| p.proyecto.nombre)
                    .or_else(|| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_default();
                Reciente {
                    existe: p.join(crate::proyecto::NOMBRE_ARCHIVO).is_file(),
                    nombre,
                    ruta: e.ruta,
                }
            })
            .collect()
    }

    /// Quita la entrada `n` (desde 1). Devuelve si existia.
    pub fn olvidar(&self, n: usize) -> std::io::Result<bool> {
        let mut v = self.leer();
        if n == 0 || n > v.len() {
            return Ok(false);
        }
        v.remove(n - 1);
        self.guardar(v)?;
        Ok(true)
    }

    pub fn olvidar_todo(&self) -> std::io::Result<()> {
        self.guardar(Vec::new())
    }
}

/// Apunta un proyecto abierto en la lista del usuario; un fallo al escribirla no interrumpe nada.
pub fn registrar(ruta: &str) {
    let _ = Recientes::por_omision().agregar(Path::new(ruta));
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionRecientes {
    Listar,
    Abrir(usize),
    Olvidar(usize),
    OlvidarTodo,
}

pub fn recientes_cli(
    accion: &AccionRecientes,
    en: Option<&str>,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};
    let r = en.map_or_else(Recientes::por_omision, Recientes::en);
    match accion {
        AccionRecientes::Listar => {
            let l = r.listar();
            if l.is_empty() {
                let _ = writeln!(salida, "no hay proyectos recientes");
            }
            for (i, p) in l.iter().enumerate() {
                let nota = if p.existe { "" } else { "  (ya no existe)" };
                let _ = writeln!(salida, "{:>2}  {}  {}{nota}", i + 1, p.nombre, p.ruta);
            }
            SALIDA_OK
        }
        AccionRecientes::Abrir(n) => match r.listar().get(n.wrapping_sub(1)) {
            Some(p) if p.existe => {
                registrar_en(&r, &p.ruta);
                crate::proyecto::proyecto_cli(&p.ruta, None, salida, errores)
            }
            Some(p) => {
                let _ = writeln!(errores, "{}: el proyecto ya no existe", p.ruta);
                SALIDA_ERROR_EJECUCION
            }
            None => {
                let _ = writeln!(errores, "no hay un proyecto reciente numero {n}");
                SALIDA_ERROR_USO
            }
        },
        AccionRecientes::Olvidar(n) => match r.olvidar(*n) {
            Ok(true) => SALIDA_OK,
            Ok(false) => {
                let _ = writeln!(errores, "no hay un proyecto reciente numero {n}");
                SALIDA_ERROR_USO
            }
            Err(e) => {
                let _ = writeln!(errores, "{e}");
                SALIDA_ERROR_EJECUCION
            }
        },
        AccionRecientes::OlvidarTodo => match r.olvidar_todo() {
            Ok(()) => SALIDA_OK,
            Err(e) => {
                let _ = writeln!(errores, "{e}");
                SALIDA_ERROR_EJECUCION
            }
        },
    }
}

fn registrar_en(r: &Recientes, ruta: &str) {
    let _ = r.agregar(Path::new(ruta));
}
