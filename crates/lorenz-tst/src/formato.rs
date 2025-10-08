//! El archivo de pruebas de un proyecto (RF-TST-001).
//!
//! ```toml
//! formato = "lorenz-pruebas-0.1"
//! maquina = "maquina.toml"
//! semilla = 42
//!
//! [[prueba]]
//! nombre   = "suma con acarreo"
//! etiquetas = ["aritmetica"]
//! programa = "programas/suma.s"
//! limite   = 1000
//! [prueba.inicial]
//! registros = { B = 3 }
//! [prueba.espera]
//! registros = { A = 44 }
//! banderas  = { C = true }
//! ciclos    = { valor = 9, tolerancia = 1 }
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use lorenz_arch::modelo::{Maquina, TipoDispositivo};
use serde::Deserialize;

use crate::error::ErrorPruebas;
use crate::numero::Num;

pub const FORMATO_PRUEBAS: &str = "lorenz-pruebas-0.1";

/// Instrucciones que puede ejecutar una prueba si no declara otro limite (RF-TST-010).
pub const LIMITE_POR_OMISION: u64 = 1_000_000;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TramoDef {
    pub direccion: Num,
    pub palabras: Vec<Num>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InicialDef {
    #[serde(default)]
    pub registros: BTreeMap<String, Num>,
    #[serde(default)]
    pub banderas: BTreeMap<String, bool>,
    #[serde(default)]
    pub memoria: Vec<TramoDef>,
}

/// Cuando ocurre un evento simulado: antes de la instruccion numero N (0 es la primera), o en
/// cuanto se hayan consumido al menos C ciclos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Momento {
    Instruccion(u64),
    Ciclo(u64),
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EntradaDef {
    /// Dispositivo que la recibe; si falta, el primero de un tipo que admita entrada.
    pub dispositivo: Option<String>,
    /// Texto que se teclea o se envia por serie.
    pub texto: Option<String>,
    /// Codigos sueltos, para teclas que no son texto.
    pub codigos: Option<Vec<Num>>,
    pub en_instruccion: Option<u64>,
    pub en_ciclo: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InterrupcionDef {
    pub linea: u32,
    pub en_instruccion: Option<u64>,
    pub en_ciclo: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SalidaDef {
    pub dispositivo: String,
    pub texto: Option<String>,
    pub codigos: Option<Vec<Num>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PuertoDef {
    pub puerto: Num,
    pub valores: Vec<Num>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CiclosDef {
    pub valor: u64,
    #[serde(default)]
    pub tolerancia: u64,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EsperaDef {
    /// Si la maquina debe haberse detenido al terminar; `true` por omision.
    pub detenida: Option<bool>,
    pub pc: Option<Num>,
    #[serde(default)]
    pub registros: BTreeMap<String, Num>,
    #[serde(default)]
    pub banderas: BTreeMap<String, bool>,
    #[serde(default)]
    pub memoria: Vec<TramoDef>,
    #[serde(default)]
    pub salida: Vec<SalidaDef>,
    #[serde(default)]
    pub puertos: Vec<PuertoDef>,
    pub ciclos: Option<CiclosDef>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PruebaDef {
    pub nombre: String,
    #[serde(default)]
    pub etiquetas: Vec<String>,
    pub programa: String,
    pub limite: Option<u64>,
    #[serde(default)]
    pub inicial: InicialDef,
    #[serde(default)]
    pub entrada: Vec<EntradaDef>,
    #[serde(default)]
    pub interrupcion: Vec<InterrupcionDef>,
    #[serde(default)]
    pub espera: EsperaDef,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ArchivoPruebas {
    formato: String,
    maquina: String,
    semilla: Option<u64>,
    #[serde(default)]
    prueba: Vec<PruebaDef>,
}

/// Un evento simulado ya ordenado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evento {
    Tecla { dispositivo: String, codigo: u64 },
    Serie { dispositivo: String, bytes: Vec<u8> },
    Interrupcion { linea: u32 },
}

/// Una prueba lista para ejecutar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prueba {
    pub def: PruebaDef,
    /// Ruta del programa, ya resuelta respecto al archivo de pruebas.
    pub programa: PathBuf,
    pub eventos: Vec<(Momento, Evento)>,
}

impl Prueba {
    pub fn nombre(&self) -> &str {
        &self.def.nombre
    }

    pub fn limite(&self) -> u64 {
        self.def.limite.unwrap_or(LIMITE_POR_OMISION)
    }

    pub fn tiene_etiqueta(&self, etiqueta: &str) -> bool {
        self.def
            .etiquetas
            .iter()
            .any(|e| e.eq_ignore_ascii_case(etiqueta))
    }
}

/// Un archivo de pruebas leido y validado contra su arquitectura.
#[derive(Debug, Clone)]
pub struct Proyecto {
    pub ruta: PathBuf,
    pub ruta_maquina: PathBuf,
    pub maquina: Maquina,
    pub semilla: u64,
    pub pruebas: Vec<Prueba>,
}

fn error_prueba(prueba: &str, motivo: impl Into<String>) -> ErrorPruebas {
    ErrorPruebas::Prueba {
        prueba: prueba.to_string(),
        motivo: motivo.into(),
    }
}

impl Proyecto {
    pub fn desde_archivo(ruta: &str) -> Result<Proyecto, ErrorPruebas> {
        let texto = std::fs::read_to_string(ruta).map_err(|e| ErrorPruebas::Lectura {
            ruta: ruta.to_string(),
            motivo: e.to_string(),
        })?;
        let base = Path::new(ruta).parent().unwrap_or_else(|| Path::new(""));
        Self::desde_texto(&texto, ruta, base)
    }

    /// `ruta` solo sirve para los mensajes; `base` es la carpeta desde la que se resuelven la
    /// arquitectura y los programas.
    pub fn desde_texto(texto: &str, ruta: &str, base: &Path) -> Result<Proyecto, ErrorPruebas> {
        let a: ArchivoPruebas = toml::from_str(texto).map_err(|e| ErrorPruebas::Sintaxis {
            ruta: ruta.to_string(),
            motivo: e.to_string().trim().to_string(),
        })?;
        if a.formato != FORMATO_PRUEBAS {
            return Err(ErrorPruebas::Formato {
                encontrado: a.formato,
                esperado: FORMATO_PRUEBAS,
            });
        }
        let ruta_maquina = base.join(&a.maquina);
        let maquina =
            lorenz_arch::cargar_archivo(&ruta_maquina).map_err(|e| ErrorPruebas::Maquina {
                ruta: ruta_maquina.to_string_lossy().into_owned(),
                motivo: e.to_string(),
            })?;
        let mut vistos = std::collections::BTreeSet::new();
        let mut pruebas = Vec::new();
        for def in a.prueba {
            if !vistos.insert(def.nombre.clone()) {
                return Err(ErrorPruebas::NombreRepetido(def.nombre));
            }
            let eventos = validar(&maquina, &def)?;
            pruebas.push(Prueba {
                programa: base.join(&def.programa),
                eventos,
                def,
            });
        }
        Ok(Proyecto {
            ruta: PathBuf::from(ruta),
            ruta_maquina,
            maquina,
            semilla: a.semilla.unwrap_or(0),
            pruebas,
        })
    }

    /// Las pruebas que quedan al filtrar por nombre (contiene el texto) o por etiqueta
    /// (RF-TST-012). Sin filtros, todas.
    pub fn seleccionar(&self, nombre: Option<&str>, etiqueta: Option<&str>) -> Vec<&Prueba> {
        self.pruebas
            .iter()
            .filter(|p| {
                nombre.map_or(true, |n| {
                    p.nombre()
                        .to_ascii_lowercase()
                        .contains(&n.to_ascii_lowercase())
                })
            })
            .filter(|p| etiqueta.map_or(true, |e| p.tiene_etiqueta(e)))
            .collect()
    }
}

/// Comprueba que todo lo que la prueba nombra existe en la maquina y ordena sus eventos.
fn validar(m: &Maquina, def: &PruebaDef) -> Result<Vec<(Momento, Evento)>, ErrorPruebas> {
    let n = def.nombre.as_str();
    if n.trim().is_empty() {
        return Err(error_prueba(
            "(sin nombre)",
            "el nombre no puede estar vacio",
        ));
    }
    if def.limite == Some(0) {
        return Err(error_prueba(
            n,
            "el limite de instrucciones debe ser mayor que cero",
        ));
    }
    let registro = |nombre: &str, donde: &str| {
        if m.registros.iter().any(|r| r.nombre == nombre) {
            Ok(())
        } else {
            Err(error_prueba(
                n,
                format!("{donde}: la maquina no tiene un registro llamado `{nombre}`"),
            ))
        }
    };
    let bandera = |nombre: &str, donde: &str| {
        if m.banderas.iter().any(|b| b.nombre == nombre) {
            Ok(())
        } else {
            Err(error_prueba(
                n,
                format!("{donde}: la maquina no tiene una bandera llamada `{nombre}`"),
            ))
        }
    };
    for r in def.inicial.registros.keys() {
        registro(r, "inicial")?;
    }
    for b in def.inicial.banderas.keys() {
        bandera(b, "inicial")?;
    }
    for r in def.espera.registros.keys() {
        registro(r, "espera")?;
    }
    for b in def.espera.banderas.keys() {
        bandera(b, "espera")?;
    }
    for t in def.inicial.memoria.iter().chain(&def.espera.memoria) {
        if t.palabras.is_empty() {
            return Err(error_prueba(n, "un tramo de memoria no tiene palabras"));
        }
        let ultima = t.direccion.0.checked_add(t.palabras.len() as u64 - 1);
        if ultima.map_or(true, |u| u > m.direccion_maxima()) {
            return Err(error_prueba(
                n,
                format!(
                    "el tramo que empieza en {:#x} se sale del espacio de direcciones",
                    t.direccion.0
                ),
            ));
        }
    }
    for p in &def.espera.puertos {
        if p.valores.is_empty() {
            return Err(error_prueba(
                n,
                "espera: un puerto sin valores no comprueba nada",
            ));
        }
    }
    for s in &def.espera.salida {
        if !m.dispositivos.iter().any(|d| d.nombre == s.dispositivo) {
            return Err(error_prueba(
                n,
                format!(
                    "espera: la maquina no tiene un dispositivo `{}`",
                    s.dispositivo
                ),
            ));
        }
        if s.texto.is_none() == s.codigos.is_none() {
            return Err(error_prueba(
                n,
                format!(
                    "espera: la salida de `{}` debe tener `texto` o `codigos`, uno de los dos",
                    s.dispositivo
                ),
            ));
        }
        let es_uart = m
            .dispositivos
            .iter()
            .any(|d| d.nombre == s.dispositivo && matches!(d.tipo, TipoDispositivo::Uart));
        if !es_uart {
            return Err(error_prueba(
                n,
                format!(
                    "espera: no se puede comprobar la salida de `{}`; solo las UART producen salida de texto (para otros puertos usa `puertos`)",
                    s.dispositivo
                ),
            ));
        }
    }

    let mut eventos = Vec::new();
    for e in &def.entrada {
        let momento = momento(n, e.en_instruccion, e.en_ciclo)?;
        let (nombre, tipo) = match &e.dispositivo {
            Some(d) => {
                let disp = m
                    .dispositivos
                    .iter()
                    .find(|x| &x.nombre == d)
                    .ok_or_else(|| {
                        error_prueba(
                            n,
                            format!("entrada: la maquina no tiene un dispositivo `{d}`"),
                        )
                    })?;
                (disp.nombre.clone(), &disp.tipo)
            }
            None => {
                let disp = m
                    .dispositivos
                    .iter()
                    .find(|x| matches!(x.tipo, TipoDispositivo::Teclado | TipoDispositivo::Uart))
                    .ok_or_else(|| {
                        error_prueba(n, "entrada: la maquina no tiene teclado ni UART")
                    })?;
                (disp.nombre.clone(), &disp.tipo)
            }
        };
        if e.texto.is_none() == e.codigos.is_none() {
            return Err(error_prueba(
                n,
                "entrada: debe tener `texto` o `codigos`, uno de los dos",
            ));
        }
        let bytes: Vec<u64> = match (&e.texto, &e.codigos) {
            (Some(t), _) => t.bytes().map(u64::from).collect(),
            (_, Some(c)) => c.iter().map(|x| x.0).collect(),
            _ => unreachable!(),
        };
        match tipo {
            TipoDispositivo::Teclado => {
                for c in bytes {
                    eventos.push((
                        momento,
                        Evento::Tecla {
                            dispositivo: nombre.clone(),
                            codigo: c,
                        },
                    ));
                }
            }
            TipoDispositivo::Uart => {
                if let Some(c) = bytes.iter().find(|c| **c > 0xFF) {
                    return Err(error_prueba(
                        n,
                        format!("entrada: {c:#x} no cabe en un byte de la UART"),
                    ));
                }
                eventos.push((
                    momento,
                    Evento::Serie {
                        dispositivo: nombre,
                        bytes: bytes.iter().map(|c| *c as u8).collect(),
                    },
                ));
            }
            _ => {
                return Err(error_prueba(
                    n,
                    format!("entrada: `{nombre}` no es un teclado ni una UART"),
                ))
            }
        }
    }
    for i in &def.interrupcion {
        let momento = momento(n, i.en_instruccion, i.en_ciclo)?;
        if !m
            .interrupciones
            .as_ref()
            .is_some_and(|c| c.vectores.contains_key(&i.linea))
        {
            return Err(error_prueba(
                n,
                format!(
                    "interrupcion: la linea {} no tiene vector declarado",
                    i.linea
                ),
            ));
        }
        eventos.push((momento, Evento::Interrupcion { linea: i.linea }));
    }
    // Estable: a igual momento se conserva el orden en que se escribieron.
    eventos.sort_by_key(|(m, _)| *m);
    Ok(eventos)
}

fn momento(
    prueba: &str,
    instruccion: Option<u64>,
    ciclo: Option<u64>,
) -> Result<Momento, ErrorPruebas> {
    match (instruccion, ciclo) {
        (Some(_), Some(_)) => Err(error_prueba(
            prueba,
            "un evento no puede tener `en_instruccion` y `en_ciclo` a la vez",
        )),
        (Some(i), None) => Ok(Momento::Instruccion(i)),
        (None, Some(c)) => Ok(Momento::Ciclo(c)),
        (None, None) => Ok(Momento::Instruccion(0)),
    }
}
