//! Ejercicios guiados (RF-ECO-016).
//!
//! Un ejercicio es una carpeta con:
//!
//! ```text
//! ejercicio.toml   titulo, nivel, enunciado, pistas
//! pruebas.toml     las pruebas de `lorenz probar` que decide si esta resuelto
//! solucion.s       el archivo que escribe quien resuelve (viene con un esqueleto)
//! referencia.s     una solucion de ejemplo (opcional)
//! ```
//!
//! `verificar` ejecuta las pruebas sobre la solucion y dice si el ejercicio esta resuelto y, si no,
//! que prueba falla y por que.

use std::io::Write;
use std::path::{Path, PathBuf};

use lorenz_tst::ejecutor::Veredicto;
use lorenz_tst::formato::Proyecto as ProyectoPruebas;
use lorenz_tst::suite::{ejecutar_suite, texto_de, Informe, Opciones};
use serde::Deserialize;

use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};

pub const NOMBRE_ARCHIVO: &str = "ejercicio.toml";
const PRUEBAS_POR_OMISION: &str = "pruebas.toml";
const SOLUCION_POR_OMISION: &str = "solucion.s";
/// Salida de `verificar` cuando el ejercicio no esta resuelto.
pub const SALIDA_NO_RESUELTO: i32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Archivo {
    ejercicio: DatosEjercicio,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatosEjercicio {
    pub id: String,
    pub titulo: String,
    /// De 1 (el primero) a 5 (el mas dificil).
    pub nivel: u8,
    pub tema: Option<String>,
    pub enunciado: String,
    #[serde(default)]
    pub pistas: Vec<String>,
    /// Archivo de pruebas, relativo a la carpeta del ejercicio.
    pub pruebas: Option<String>,
    /// Archivo que escribe quien resuelve, relativo a la carpeta del ejercicio.
    pub solucion: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Ejercicio {
    pub carpeta: PathBuf,
    pub datos: DatosEjercicio,
}

pub(crate) fn id_valido(n: &str) -> bool {
    !n.is_empty()
        && n.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

impl Ejercicio {
    /// Abre la carpeta de un ejercicio y valida su `ejercicio.toml`.
    pub fn abrir(carpeta: &Path) -> Result<Self, String> {
        let archivo = carpeta.join(NOMBRE_ARCHIVO);
        let texto = std::fs::read_to_string(&archivo)
            .map_err(|e| format!("{}: no se pudo leer: {e}", archivo.display()))?;
        let a: Archivo =
            toml::from_str(&texto).map_err(|e| format!("{}: {e}", archivo.display()))?;
        let d = a.ejercicio;
        let f = |m: String| format!("{}: {m}", archivo.display());
        if !id_valido(&d.id) {
            return Err(f(format!(
                "el id `{}` solo admite letras, digitos, `-` y `_`",
                d.id
            )));
        }
        if !(1..=5).contains(&d.nivel) {
            return Err(f(format!(
                "el nivel debe estar entre 1 y 5, no {}",
                d.nivel
            )));
        }
        if d.titulo.trim().is_empty() || d.enunciado.trim().is_empty() {
            return Err(f("el titulo y el enunciado no pueden estar vacios".into()));
        }
        for r in [&d.pruebas, &d.solucion].into_iter().flatten() {
            crate::proyecto::ruta_interna(r).map_err(f)?;
        }
        Ok(Self {
            carpeta: carpeta.to_path_buf(),
            datos: d,
        })
    }

    pub fn ruta_pruebas(&self) -> PathBuf {
        self.carpeta
            .join(self.datos.pruebas.as_deref().unwrap_or(PRUEBAS_POR_OMISION))
    }

    pub fn ruta_solucion(&self) -> PathBuf {
        self.carpeta.join(
            self.datos
                .solucion
                .as_deref()
                .unwrap_or(SOLUCION_POR_OMISION),
        )
    }

    pub fn ruta_referencia(&self) -> PathBuf {
        self.carpeta.join("referencia.s")
    }
}

/// Los ejercicios de una carpeta, del mas facil al mas dificil y, a igual nivel, por id.
pub fn listar(raiz: &Path) -> Result<Vec<Ejercicio>, String> {
    let entradas = std::fs::read_dir(raiz).map_err(|e| format!("{}: {e}", raiz.display()))?;
    let mut v = Vec::new();
    for e in entradas.flatten() {
        if e.path().join(NOMBRE_ARCHIVO).is_file() {
            v.push(Ejercicio::abrir(&e.path())?);
        }
    }
    v.sort_by(|a, b| (a.datos.nivel, &a.datos.id).cmp(&(b.datos.nivel, &b.datos.id)));
    Ok(v)
}

/// Lo que dejo ejecutar las pruebas sobre una solucion.
#[derive(Debug, Clone)]
pub struct Medicion {
    pub informe: Informe,
    /// Ciclos sumados de todas las pruebas.
    pub ciclos: u64,
    /// Instrucciones sumadas de todas las pruebas.
    pub instrucciones: u64,
    /// Tamano del programa en palabras, si se pudo medir.
    pub palabras: Option<u64>,
}

impl Medicion {
    pub fn resuelto(&self) -> bool {
        self.informe.resumen.fallan == 0
            && self.informe.resumen.errores == 0
            && self.informe.resumen.total > 0
    }
}

/// Ejecuta las pruebas de `pruebas` poniendo `solucion` como programa de todas ellas.
pub fn medir(pruebas: &Path, solucion: &Path) -> Result<Medicion, String> {
    let mut proyecto =
        ProyectoPruebas::desde_archivo(&pruebas.to_string_lossy()).map_err(|e| e.to_string())?;
    if !solucion.is_file() {
        return Err(format!("{}: no existe", solucion.display()));
    }
    for p in &mut proyecto.pruebas {
        p.programa = solucion.to_path_buf();
    }
    let informe = ejecutar_suite(
        &proyecto,
        &Opciones {
            hilos: 1,
            ..Opciones::default()
        },
    );
    let (ciclos, instrucciones) = informe
        .pruebas
        .iter()
        .fold((0, 0), |(c, i), r| (c + r.ciclos, i + r.instrucciones));
    let palabras = tamano(&proyecto, solucion);
    Ok(Medicion {
        informe,
        ciclos,
        instrucciones,
        palabras,
    })
}

fn tamano(p: &ProyectoPruebas, solucion: &Path) -> Option<u64> {
    let ruta = solucion.to_string_lossy();
    if ruta.ends_with(".s") || ruta.ends_with(".asm") {
        crate::ensamblar::ensamblar_archivo(&p.maquina, &ruta)
            .ok()
            .map(|(o, _)| o.tamano() as u64)
    } else {
        let t = std::fs::read_to_string(solucion).ok()?;
        lorenz_emu::programa::leer_palabras(&t, p.maquina.bits_palabra)
            .ok()
            .map(|v| v.len() as u64)
    }
}

fn ver(
    e: &Ejercicio,
    pista: Option<usize>,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let d = &e.datos;
    match pista {
        None => {
            let _ = writeln!(salida, "{} (nivel {}): {}", d.id, d.nivel, d.titulo);
            if let Some(t) = &d.tema {
                let _ = writeln!(salida, "tema: {t}");
            }
            let _ = writeln!(salida, "\n{}", d.enunciado.trim());
            let _ = writeln!(
                salida,
                "\nEscribe tu solucion en {}",
                e.ruta_solucion().display()
            );
            if !d.pistas.is_empty() {
                let _ = writeln!(
                    salida,
                    "Hay {} pista(s): `lorenz ejercicio ver <carpeta> --pista N`.",
                    d.pistas.len()
                );
            }
            SALIDA_OK
        }
        Some(n) => match n.checked_sub(1).and_then(|i| d.pistas.get(i)) {
            Some(p) => {
                let _ = writeln!(salida, "pista {n} de {}: {p}", d.pistas.len());
                SALIDA_OK
            }
            None => {
                let _ = writeln!(
                    errores,
                    "no hay pista {n}; el ejercicio tiene {}",
                    d.pistas.len()
                );
                SALIDA_ERROR_USO
            }
        },
    }
}

fn verificar(
    e: &Ejercicio,
    solucion: Option<&str>,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let sol = solucion.map_or_else(|| e.ruta_solucion(), PathBuf::from);
    let m = match medir(&e.ruta_pruebas(), &sol) {
        Ok(m) => m,
        Err(msg) => {
            let _ = writeln!(errores, "{msg}");
            return SALIDA_ERROR_USO;
        }
    };
    let _ = writeln!(salida, "{}: {}", e.datos.id, e.datos.titulo);
    for r in &m.informe.pruebas {
        let _ = write!(salida, "{}", texto_de(r));
    }
    let r = &m.informe.resumen;
    if m.resuelto() {
        let _ = writeln!(
            salida,
            "\nResuelto: {} de {} pruebas, {} ciclos, {} instrucciones{}.",
            r.pasan,
            r.total,
            m.ciclos,
            m.instrucciones,
            m.palabras
                .map_or(String::new(), |p| format!(", {p} palabras"))
        );
        SALIDA_OK
    } else {
        let _ = writeln!(
            salida,
            "\nTodavia no: pasan {} de {} pruebas{}.",
            r.pasan,
            r.total,
            if e.datos.pistas.is_empty() {
                ""
            } else {
                "; hay pistas con `ver --pista N`"
            }
        );
        if r.errores > 0 {
            SALIDA_ERROR_EJECUCION
        } else {
            SALIDA_NO_RESUELTO
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionEjercicio {
    Listar {
        raiz: String,
    },
    Ver {
        carpeta: String,
        pista: Option<usize>,
    },
    Verificar {
        carpeta: String,
        solucion: Option<String>,
    },
}

pub fn ejercicio_cli(
    accion: &AccionEjercicio,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    match accion {
        AccionEjercicio::Listar { raiz } => match listar(Path::new(raiz)) {
            Ok(v) => {
                if v.is_empty() {
                    let _ = writeln!(salida, "no hay ejercicios en {raiz}");
                }
                for e in &v {
                    let _ = writeln!(
                        salida,
                        "{:<22} nivel {}  {}",
                        e.datos.id, e.datos.nivel, e.datos.titulo
                    );
                }
                SALIDA_OK
            }
            Err(m) => {
                let _ = writeln!(errores, "{m}");
                SALIDA_ERROR_USO
            }
        },
        AccionEjercicio::Ver { carpeta, pista } => match Ejercicio::abrir(Path::new(carpeta)) {
            Ok(e) => ver(&e, *pista, salida, errores),
            Err(m) => {
                let _ = writeln!(errores, "{m}");
                SALIDA_ERROR_USO
            }
        },
        AccionEjercicio::Verificar { carpeta, solucion } => {
            match Ejercicio::abrir(Path::new(carpeta)) {
                Ok(e) => verificar(&e, solucion.as_deref(), salida, errores),
                Err(m) => {
                    let _ = writeln!(errores, "{m}");
                    SALIDA_ERROR_USO
                }
            }
        }
    }
}

/// Un veredicto por prueba, para quien quiera mas que un si o un no.
pub fn veredictos(m: &Medicion) -> Vec<(String, Veredicto)> {
    m.informe
        .pruebas
        .iter()
        .map(|r| (r.nombre.clone(), r.veredicto))
        .collect()
}
