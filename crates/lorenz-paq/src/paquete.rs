//! El paquete: manifiesto, archivos y su codificacion en un solo archivo `.lpkg`
//! (RF-ECO-001, RF-ECO-002, RF-ECO-007, RF-ECO-008).
//!
//! Formato del archivo (D-109): texto y bytes mezclados, sin compresion.
//!
//! ```text
//! LORENZ-PAQUETE 1
//! ARCHIVO <ruta> <longitud> <sha256>
//! <longitud bytes>
//! ... (uno por archivo; el primero es paquete.toml)
//! FIN <sha256 de todo lo anterior>
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

use crate::sha256::sha256_hex;
use crate::version::{Requisito, Version};

pub const CABECERA: &str = "LORENZ-PAQUETE 1";
pub const NOMBRE_MANIFIESTO: &str = "paquete.toml";
/// Version del formato de descripcion de arquitectura que esta version de Lorenz entiende.
pub const FORMATO_SOPORTADO: &str = "0.1";
pub const MAX_ARCHIVO: usize = 16 << 20;
pub const MAX_TOTAL: usize = 64 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorPaquete {
    Manifiesto(String),
    Formato(String),
    /// La suma de un archivo o la del paquete entero no coincide.
    Integridad(String),
    Ruta(String),
    Entrada(String),
    Dependencia(String),
}

impl fmt::Display for ErrorPaquete {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifiesto(m) => write!(f, "manifiesto no valido: {m}"),
            Self::Formato(m) => write!(f, "paquete no valido: {m}"),
            Self::Integridad(m) => write!(f, "paquete danado: {m}"),
            Self::Ruta(m) => write!(f, "ruta no permitida: {m}"),
            Self::Entrada(m) | Self::Dependencia(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for ErrorPaquete {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tipo {
    Arquitectura,
    Dispositivo,
    Biblioteca,
    Proyecto,
}

impl Tipo {
    pub fn nombre(self) -> &'static str {
        match self {
            Self::Arquitectura => "arquitectura",
            Self::Dispositivo => "dispositivo",
            Self::Biblioteca => "biblioteca",
            Self::Proyecto => "proyecto",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Crudo {
    paquete: CrudoPaquete,
    #[serde(default)]
    dependencias: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CrudoPaquete {
    nombre: String,
    version: String,
    #[serde(default)]
    autor: String,
    licencia: Option<String>,
    tipo: String,
    descripcion: Option<String>,
    /// Version del formato de arquitectura que el paquete exige.
    requiere_formato: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifiesto {
    pub nombre: String,
    pub version: Version,
    pub autor: String,
    pub licencia: String,
    pub tipo: Tipo,
    pub descripcion: String,
    pub requiere_formato: Option<String>,
    pub dependencias: Vec<(String, Requisito)>,
}

/// Un nombre de paquete: minusculas, digitos, `-` y `_`, de 1 a 64 caracteres.
pub fn nombre_valido(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 64
        && n.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        && !n.starts_with(['-', '_'])
}

impl Manifiesto {
    pub fn leer(texto: &str) -> Result<Self, ErrorPaquete> {
        let m = |e: String| ErrorPaquete::Manifiesto(e);
        let c: Crudo = toml::from_str(texto).map_err(|e| m(e.to_string()))?;
        let p = c.paquete;
        if !nombre_valido(&p.nombre) {
            return Err(m(format!(
                "el nombre `{}` no vale: minusculas, digitos, `-` y `_`, hasta 64 caracteres",
                p.nombre
            )));
        }
        let version = Version::parsear(&p.version).map_err(m)?;
        let tipo = match p.tipo.as_str() {
            "arquitectura" => Tipo::Arquitectura,
            "dispositivo" => Tipo::Dispositivo,
            "biblioteca" => Tipo::Biblioteca,
            "proyecto" => Tipo::Proyecto,
            otro => {
                return Err(m(format!(
                    "el tipo `{otro}` no existe (arquitectura, dispositivo, biblioteca o proyecto)"
                )))
            }
        };
        let licencia = p
            .licencia
            .filter(|l| !l.trim().is_empty())
            .ok_or_else(|| m("falta `licencia` en [paquete]".into()))?;
        if p.autor.trim().is_empty() {
            return Err(m("falta `autor` en [paquete]".into()));
        }
        let mut dependencias = Vec::new();
        for (n, r) in c.dependencias {
            if !nombre_valido(&n) {
                return Err(m(format!("la dependencia `{n}` no tiene un nombre valido")));
            }
            if n == p.nombre {
                return Err(m("un paquete no puede depender de si mismo".into()));
            }
            let req = Requisito::parsear(&r).map_err(|e| m(format!("dependencia `{n}`: {e}")))?;
            dependencias.push((n, req));
        }
        Ok(Self {
            nombre: p.nombre,
            version,
            autor: p.autor,
            licencia,
            tipo,
            descripcion: p.descripcion.unwrap_or_default(),
            requiere_formato: p.requiere_formato,
            dependencias,
        })
    }
}

/// Una ruta relativa, con `/`, sin `..`, sin raiz y sin segmentos vacios.
pub fn ruta_valida(r: &str) -> Result<(), ErrorPaquete> {
    let mal = |m: &str| Err(ErrorPaquete::Ruta(format!("`{r}`: {m}")));
    if r.is_empty() || r.len() > 200 {
        return mal("vacia o demasiado larga");
    }
    if r.starts_with('/') || r.contains('\\') || r.contains(':') || r.contains('\0') {
        return mal("debe ser relativa y usar `/`");
    }
    if r.split('/').any(|s| s.is_empty() || s == "." || s == "..") {
        return mal("no puede tener segmentos vacios, `.` ni `..`");
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paquete {
    pub manifiesto: Manifiesto,
    /// Todos los archivos, `paquete.toml` incluido, por ruta.
    pub archivos: BTreeMap<String, Vec<u8>>,
}

fn ignorado(nombre: &str) -> bool {
    matches!(nombre, ".git" | "target" | ".lorenz")
        || nombre.ends_with(".lpkg")
        || nombre.starts_with(".DS_")
}

fn recorrer(
    base: &Path,
    dir: &Path,
    salida: &mut BTreeMap<String, Vec<u8>>,
    total: &mut usize,
) -> Result<(), ErrorPaquete> {
    let e = |p: &Path, x: std::io::Error| ErrorPaquete::Entrada(format!("{}: {x}", p.display()));
    let mut entradas: Vec<_> = std::fs::read_dir(dir)
        .map_err(|x| e(dir, x))?
        .collect::<Result<_, _>>()
        .map_err(|x| e(dir, x))?;
    entradas.sort_by_key(|x| x.file_name());
    for en in entradas {
        let nombre = en.file_name().to_string_lossy().into_owned();
        if ignorado(&nombre) {
            continue;
        }
        let ruta = en.path();
        let tipo = en.file_type().map_err(|x| e(&ruta, x))?;
        if tipo.is_symlink() {
            return Err(ErrorPaquete::Ruta(format!(
                "{}: los enlaces simbolicos no se empaquetan",
                ruta.display()
            )));
        }
        if tipo.is_dir() {
            recorrer(base, &ruta, salida, total)?;
        } else {
            let datos = std::fs::read(&ruta).map_err(|x| e(&ruta, x))?;
            if datos.len() > MAX_ARCHIVO {
                return Err(ErrorPaquete::Entrada(format!(
                    "{}: pesa mas de {} MiB",
                    ruta.display(),
                    MAX_ARCHIVO >> 20
                )));
            }
            *total += datos.len();
            if *total > MAX_TOTAL {
                return Err(ErrorPaquete::Entrada(format!(
                    "el paquete pesa mas de {} MiB",
                    MAX_TOTAL >> 20
                )));
            }
            let rel = ruta
                .strip_prefix(base)
                .map_err(|_| ErrorPaquete::Ruta(ruta.display().to_string()))?
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            ruta_valida(&rel)?;
            salida.insert(rel, datos);
        }
    }
    Ok(())
}

impl Paquete {
    /// Empaqueta una carpeta con un `paquete.toml` en su raiz (RF-ECO-007). Se dejan fuera `.git`,
    /// `target`, `.lorenz` y los `.lpkg`.
    pub fn desde_carpeta(carpeta: impl AsRef<Path>) -> Result<Self, ErrorPaquete> {
        let base = carpeta.as_ref();
        let mut archivos = BTreeMap::new();
        recorrer(base, base, &mut archivos, &mut 0)?;
        let texto = archivos.get(NOMBRE_MANIFIESTO).ok_or_else(|| {
            ErrorPaquete::Manifiesto(format!("falta `{NOMBRE_MANIFIESTO}` en {}", base.display()))
        })?;
        let manifiesto = Manifiesto::leer(&String::from_utf8_lossy(texto))?;
        Ok(Self {
            manifiesto,
            archivos,
        })
    }

    /// El paquete en bytes, siempre los mismos para el mismo contenido.
    pub fn a_bytes(&self) -> Vec<u8> {
        let mut s = format!("{CABECERA}\n").into_bytes();
        let mut rutas: Vec<&String> = self.archivos.keys().collect();
        // El manifiesto va primero para poder leerlo sin recorrer el resto.
        rutas.sort_by_key(|r| (r.as_str() != NOMBRE_MANIFIESTO, r.to_string()));
        for r in rutas {
            let d = &self.archivos[r];
            s.extend(format!("ARCHIVO {r} {} {}\n", d.len(), sha256_hex(d)).into_bytes());
            s.extend(d);
            s.push(b'\n');
        }
        let suma = sha256_hex(&s);
        s.extend(format!("FIN {suma}\n").into_bytes());
        s
    }

    /// Lee un paquete y **verifica su integridad** antes de devolverlo (RF-ECO-008): la suma de
    /// cada archivo y la del conjunto, las rutas y el manifiesto.
    pub fn desde_bytes(b: &[u8]) -> Result<Self, ErrorPaquete> {
        let f = |m: &str| ErrorPaquete::Formato(m.to_string());
        let linea = |pos: usize| -> Option<(&str, usize)> {
            let fin = b[pos..].iter().position(|c| *c == b'\n')? + pos;
            Some((std::str::from_utf8(&b[pos..fin]).ok()?, fin + 1))
        };
        let (cab, mut pos) = linea(0).ok_or_else(|| f("no es un paquete de Lorenz"))?;
        if cab != CABECERA {
            return Err(f(
                "no es un paquete de Lorenz o es de otra version del formato",
            ));
        }
        let mut archivos = BTreeMap::new();
        let mut total = 0usize;
        loop {
            let (l, siguiente) =
                linea(pos).ok_or_else(|| f("termina sin la linea FIN: esta truncado"))?;
            if let Some(suma) = l.strip_prefix("FIN ") {
                if siguiente != b.len() {
                    return Err(f("hay datos despues de la linea FIN"));
                }
                if sha256_hex(&b[..pos]) != suma {
                    return Err(ErrorPaquete::Integridad(
                        "la suma del paquete no coincide".into(),
                    ));
                }
                break;
            }
            let partes: Vec<&str> = l.split(' ').collect();
            let ["ARCHIVO", ruta, largo, suma] = partes[..] else {
                return Err(f("una linea de archivo esta mal formada"));
            };
            ruta_valida(ruta)?;
            let largo: usize = largo
                .parse()
                .map_err(|_| f("longitud de archivo no numerica"))?;
            if largo > MAX_ARCHIVO {
                return Err(f("un archivo declara un tamano excesivo"));
            }
            total += largo;
            if total > MAX_TOTAL {
                return Err(f("el paquete declara un tamano excesivo"));
            }
            let fin = siguiente
                .checked_add(largo)
                .filter(|x| *x < b.len())
                .ok_or_else(|| f("termina dentro de un archivo: esta truncado"))?;
            let datos = &b[siguiente..fin];
            if b[fin] != b'\n' {
                return Err(f("falta el salto de linea tras un archivo"));
            }
            if sha256_hex(datos) != suma {
                return Err(ErrorPaquete::Integridad(format!(
                    "la suma de `{ruta}` no coincide"
                )));
            }
            if archivos.insert(ruta.to_string(), datos.to_vec()).is_some() {
                return Err(f("un archivo aparece dos veces"));
            }
            pos = fin + 1;
        }
        let texto = archivos
            .get(NOMBRE_MANIFIESTO)
            .ok_or_else(|| ErrorPaquete::Manifiesto(format!("falta `{NOMBRE_MANIFIESTO}`")))?;
        let manifiesto = Manifiesto::leer(&String::from_utf8_lossy(texto))?;
        Ok(Self {
            manifiesto,
            archivos,
        })
    }

    /// Avisos sobre el contenido: hoy, que exija un formato de arquitectura que esta version no
    /// entiende (RF-ECO-006).
    pub fn avisos(&self) -> Vec<String> {
        let mut v = Vec::new();
        if let Some(req) = &self.manifiesto.requiere_formato {
            let sop = Version::parsear(FORMATO_SOPORTADO).expect("constante valida");
            match Version::parsear(req) {
                Ok(pedida) if pedida > sop => v.push(format!(
                    "el paquete `{}` exige el formato de arquitectura {req}, y esta version de Lorenz entiende hasta el {FORMATO_SOPORTADO}: puede no cargarse",
                    self.manifiesto.nombre
                )),
                Ok(_) => {}
                Err(e) => v.push(format!("`requiere_formato` no es una version: {e}")),
            }
        }
        v
    }
}
