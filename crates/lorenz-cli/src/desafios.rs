//! Desafios con metricas objetivas (RF-ECO-014).
//!
//! Un desafio es una carpeta con `desafio.toml` y un archivo de pruebas. Una solucion cuenta solo
//! si pasa todas las pruebas; entre las que cuentan se ordena por la metrica del desafio (ciclos,
//! instrucciones o palabras del programa; menos es mejor). Las demas metricas desempatan a la hora
//! de mostrar, pero no cambian el puesto: dos soluciones con la misma metrica principal empatan.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::ejercicios::{id_valido, medir, Medicion};
use crate::ordenes::{SALIDA_ERROR_USO, SALIDA_OK};

pub const NOMBRE_ARCHIVO: &str = "desafio.toml";
/// Alguna de las soluciones dadas no resuelve el desafio.
pub const SALIDA_NO_RESUELVE: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Metrica {
    Ciclos,
    Instrucciones,
    Palabras,
}

impl Metrica {
    pub fn nombre(self) -> &'static str {
        match self {
            Metrica::Ciclos => "ciclos",
            Metrica::Instrucciones => "instrucciones",
            Metrica::Palabras => "palabras",
        }
    }

    fn valor(self, m: &Medicion) -> u64 {
        match self {
            Metrica::Ciclos => m.ciclos,
            Metrica::Instrucciones => m.instrucciones,
            Metrica::Palabras => m.palabras.unwrap_or(u64::MAX),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Archivo {
    desafio: DatosDesafio,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatosDesafio {
    pub id: String,
    pub titulo: String,
    pub enunciado: String,
    pub metrica: Metrica,
    pub pruebas: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Desafio {
    pub carpeta: PathBuf,
    pub datos: DatosDesafio,
}

impl Desafio {
    pub fn abrir(carpeta: &Path) -> Result<Self, String> {
        let archivo = carpeta.join(NOMBRE_ARCHIVO);
        let texto = std::fs::read_to_string(&archivo)
            .map_err(|e| format!("{}: no se pudo leer: {e}", archivo.display()))?;
        let a: Archivo =
            toml::from_str(&texto).map_err(|e| format!("{}: {e}", archivo.display()))?;
        let d = a.desafio;
        let f = |m: String| format!("{}: {m}", archivo.display());
        if !id_valido(&d.id) {
            return Err(f(format!(
                "el id `{}` solo admite letras, digitos, `-` y `_`",
                d.id
            )));
        }
        if d.titulo.trim().is_empty() || d.enunciado.trim().is_empty() {
            return Err(f("el titulo y el enunciado no pueden estar vacios".into()));
        }
        if let Some(p) = &d.pruebas {
            crate::proyecto::ruta_interna(p).map_err(f)?;
        }
        Ok(Self {
            carpeta: carpeta.to_path_buf(),
            datos: d,
        })
    }

    pub fn ruta_pruebas(&self) -> PathBuf {
        self.carpeta
            .join(self.datos.pruebas.as_deref().unwrap_or("pruebas.toml"))
    }
}

/// Una solucion ya medida.
#[derive(Debug, Clone)]
pub struct Entrada {
    pub solucion: String,
    pub medicion: Result<Medicion, String>,
}

impl Entrada {
    fn valida(&self) -> Option<&Medicion> {
        self.medicion.as_ref().ok().filter(|m| m.resuelto())
    }
}

/// Una fila del ranking: el puesto es `None` si la solucion no resuelve el desafio.
#[derive(Debug, Clone)]
pub struct Fila<'a> {
    pub puesto: Option<usize>,
    pub entrada: &'a Entrada,
}

/// Ordena las soluciones: primero las que resuelven, por la metrica principal y luego por las
/// demas; despues las que no. Los empates en la metrica principal comparten puesto (1, 1, 3...).
pub fn ranking<'a>(d: &Desafio, entradas: &'a [Entrada]) -> Vec<Fila<'a>> {
    let metrica = d.datos.metrica;
    let clave = |m: &Medicion| {
        (
            metrica.valor(m),
            m.ciclos,
            m.instrucciones,
            m.palabras.unwrap_or(u64::MAX),
        )
    };
    let mut validas: Vec<&Entrada> = entradas.iter().filter(|e| e.valida().is_some()).collect();
    validas.sort_by_key(|e| clave(e.valida().unwrap()));
    let mut filas = Vec::new();
    let mut anterior: Option<(u64, usize)> = None;
    for (i, e) in validas.iter().enumerate() {
        let v = metrica.valor(e.valida().unwrap());
        let puesto = match anterior {
            Some((pv, pp)) if pv == v => pp,
            _ => i + 1,
        };
        anterior = Some((v, puesto));
        filas.push(Fila {
            puesto: Some(puesto),
            entrada: e,
        });
    }
    filas.extend(
        entradas
            .iter()
            .filter(|e| e.valida().is_none())
            .map(|e| Fila {
                puesto: None,
                entrada: e,
            }),
    );
    filas
}

fn nombre_corto(ruta: &str) -> String {
    Path::new(ruta)
        .file_name()
        .map_or_else(|| ruta.to_string(), |n| n.to_string_lossy().into_owned())
}

fn evaluar_todas(d: &Desafio, soluciones: &[String]) -> Vec<Entrada> {
    soluciones
        .iter()
        .map(|s| Entrada {
            solucion: s.clone(),
            medicion: medir(&d.ruta_pruebas(), Path::new(s)),
        })
        .collect()
}

fn tabla(filas: &[Fila<'_>], salida: &mut dyn Write) {
    let _ = writeln!(
        salida,
        "{:>6}  {:<28} {:>8} {:>14} {:>9}",
        "puesto", "solucion", "ciclos", "instrucciones", "palabras"
    );
    for f in filas {
        let nombre = nombre_corto(&f.entrada.solucion);
        match &f.entrada.medicion {
            Ok(m) if f.puesto.is_some() => {
                let marca = if f.puesto == Some(1) { "*" } else { " " };
                let _ = writeln!(
                    salida,
                    "{:>5}{marca}  {:<28} {:>8} {:>14} {:>9}",
                    f.puesto.unwrap_or(0),
                    nombre,
                    m.ciclos,
                    m.instrucciones,
                    m.palabras.map_or("-".to_string(), |p| p.to_string())
                );
            }
            Ok(m) => {
                let _ = writeln!(
                    salida,
                    "{:>6}  {:<28} no resuelve: pasan {} de {} pruebas",
                    "-", nombre, m.informe.resumen.pasan, m.informe.resumen.total
                );
            }
            Err(e) => {
                let _ = writeln!(salida, "{:>6}  {:<28} no se pudo medir: {e}", "-", nombre);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionDesafio {
    Ver {
        carpeta: String,
    },
    Evaluar {
        carpeta: String,
        solucion: String,
    },
    Comparar {
        carpeta: String,
        soluciones: Vec<String>,
    },
}

pub fn desafio_cli(accion: &AccionDesafio, salida: &mut dyn Write, errores: &mut dyn Write) -> i32 {
    let (carpeta, soluciones) = match accion {
        AccionDesafio::Ver { carpeta } => (carpeta, Vec::new()),
        AccionDesafio::Evaluar { carpeta, solucion } => (carpeta, vec![solucion.clone()]),
        AccionDesafio::Comparar {
            carpeta,
            soluciones,
        } => (carpeta, soluciones.clone()),
    };
    let d = match Desafio::abrir(Path::new(carpeta)) {
        Ok(d) => d,
        Err(e) => {
            let _ = writeln!(errores, "{e}");
            return SALIDA_ERROR_USO;
        }
    };
    if matches!(accion, AccionDesafio::Ver { .. }) {
        let _ = writeln!(salida, "{}: {}", d.datos.id, d.datos.titulo);
        let _ = writeln!(
            salida,
            "metrica: {} (menos es mejor)\n",
            d.datos.metrica.nombre()
        );
        let _ = writeln!(salida, "{}", d.datos.enunciado.trim());
        return SALIDA_OK;
    }
    let entradas = evaluar_todas(&d, &soluciones);
    if let [Entrada {
        medicion: Err(e), ..
    }] = entradas.as_slice()
    {
        let _ = writeln!(errores, "{e}");
        return SALIDA_ERROR_USO;
    }
    let filas = ranking(&d, &entradas);
    let _ = writeln!(
        salida,
        "{}: {} (metrica: {})",
        d.datos.id,
        d.datos.titulo,
        d.datos.metrica.nombre()
    );
    if filas.first().is_some_and(|f| f.puesto.is_none()) {
        // Nadie resuelve: no hay primera fila valida que marcar.
        for f in &filas {
            if let Ok(m) = &f.entrada.medicion {
                let _ = writeln!(
                    salida,
                    "{}: no resuelve, pasan {} de {} pruebas",
                    nombre_corto(&f.entrada.solucion),
                    m.informe.resumen.pasan,
                    m.informe.resumen.total
                );
            } else if let Err(e) = &f.entrada.medicion {
                let _ = writeln!(
                    salida,
                    "{}: no se pudo medir: {e}",
                    nombre_corto(&f.entrada.solucion)
                );
            }
        }
    } else {
        tabla(&filas, salida);
    }
    if entradas.iter().all(|e| e.valida().is_some()) {
        SALIDA_OK
    } else {
        SALIDA_NO_RESUELVE
    }
}
