//! Lectura de una descripcion de arquitectura desde TOML.
//!
//! El TOML se deserializa a estructuras crudas que reflejan el archivo tal cual, y despues se
//! convierten al modelo validado. Separar ambas cosas permite que los mensajes de error hablen del
//! archivo y no de la estructura interna.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::bits::Codificacion;
use crate::error::{ErrorArquitectura, FalloValidacion, Regla};
use crate::modelo::*;
use crate::semantica;
use crate::validacion;
use crate::VERSION_FORMATO;

// ---------------------------------------------------------------------------
// Estructuras crudas: el archivo tal como se escribe
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ArchivoCrudo {
    maquina: MaquinaCruda,
    #[serde(default)]
    flags: BTreeMap<String, BanderaCruda>,
    #[serde(default, rename = "registro")]
    registros: Vec<RegistroCrudo>,
    #[serde(default, rename = "instruccion")]
    instrucciones: Vec<InstruccionCruda>,
    #[serde(default, rename = "region")]
    regiones: Vec<RegionCruda>,
    #[serde(default, rename = "dispositivo")]
    dispositivos: Vec<DispositivoCrudo>,
}

#[derive(Debug, Deserialize)]
struct MaquinaCruda {
    nombre: String,
    formato: String,
    bits_palabra: u32,
    bits_direccion: u32,
    #[serde(default = "endianness_por_defecto")]
    endianness: String,
    #[serde(default)]
    arranque: u64,
    #[serde(default = "pila_por_defecto")]
    pila: String,
    #[serde(default)]
    interrupciones: Option<InterrupcionesCrudas>,
}

fn endianness_por_defecto() -> String {
    "little".to_string()
}

fn pila_por_defecto() -> String {
    "descendente".to_string()
}

#[derive(Debug, Deserialize)]
struct InterrupcionesCrudas {
    #[serde(default)]
    vectores: BTreeMap<String, u64>,
    #[serde(default)]
    prioridades: BTreeMap<String, u32>,
    #[serde(default)]
    habilitadas_al_arrancar: bool,
}

#[derive(Debug, Deserialize)]
struct BanderaCruda {
    bit: u32,
    #[serde(default)]
    descripcion: String,
}

#[derive(Debug, Deserialize)]
struct RegistroCrudo {
    nombre: String,
    bits: u32,
    #[serde(default = "rol_por_defecto")]
    rol: String,
    #[serde(default)]
    inicial: u64,
}

fn rol_por_defecto() -> String {
    "general".to_string()
}

#[derive(Debug, Deserialize)]
struct InstruccionCruda {
    mnemonico: String,
    #[serde(default)]
    sintaxis: String,
    codigo: String,
    #[serde(default)]
    semantica: String,
    #[serde(default)]
    afecta: Vec<String>,
    #[serde(default)]
    ciclos: Option<toml::Value>,
    #[serde(default)]
    descripcion: String,
    #[serde(default, rename = "operando")]
    operandos: Vec<OperandoCrudo>,
}

#[derive(Debug, Deserialize)]
struct OperandoCrudo {
    nombre: String,
    tipo: String,
    #[serde(default)]
    de: Vec<String>,
    #[serde(default)]
    signo: bool,
    #[serde(default)]
    rango: Option<[i64; 2]>,
}

#[derive(Debug, Deserialize)]
struct RegionCruda {
    nombre: String,
    desde: u64,
    hasta: u64,
    #[serde(default = "acceso_por_defecto")]
    acceso: String,
    #[serde(default)]
    dispositivo: Option<String>,
    #[serde(default)]
    contenido: Option<String>,
}

fn acceso_por_defecto() -> String {
    "rw".to_string()
}

#[derive(Debug, Deserialize)]
struct DispositivoCrudo {
    nombre: String,
    tipo: String,
    #[serde(default)]
    puertos: BTreeMap<String, u64>,
    #[serde(default)]
    interrupcion: Option<u32>,
    #[serde(default)]
    ancho: Option<u32>,
    #[serde(default)]
    alto: Option<u32>,
    #[serde(default)]
    formato: Option<String>,
    #[serde(default)]
    tamano_bloque: Option<u32>,
    #[serde(default)]
    cantidad_bloques: Option<u32>,
    #[serde(default)]
    script: Option<String>,
    #[serde(default)]
    periodo: Option<u64>,
}

// ---------------------------------------------------------------------------
// Carga
// ---------------------------------------------------------------------------

/// Lee y valida una descripcion de arquitectura desde un archivo.
pub fn cargar_archivo(ruta: impl AsRef<Path>) -> Result<Maquina, ErrorArquitectura> {
    let texto = std::fs::read_to_string(ruta)?;
    cargar(&texto)
}

/// Lee y valida una descripcion de arquitectura desde su texto TOML.
///
/// Devuelve la maquina solo si pasa todas las reglas de validacion; en caso contrario devuelve la
/// lista completa de fallos, no solo el primero.
pub fn cargar(texto: &str) -> Result<Maquina, ErrorArquitectura> {
    let crudo: ArchivoCrudo = toml::from_str(texto)?;

    comprobar_version(&crudo.maquina.formato)?;

    let mut fallos = Vec::new();
    let maquina = convertir(crudo, &mut fallos)?;

    fallos.extend(validacion::validar(&maquina));

    if fallos.is_empty() {
        Ok(maquina)
    } else {
        Err(ErrorArquitectura::Validacion(fallos))
    }
}

/// Una version mayor distinta se rechaza en lugar de interpretarse a medias (RF-ARQ-002).
fn comprobar_version(pedida: &str) -> Result<(), ErrorArquitectura> {
    let mayor = |v: &str| v.split('.').next().unwrap_or("").to_string();
    if mayor(pedida) == mayor(VERSION_FORMATO) {
        Ok(())
    } else {
        Err(ErrorArquitectura::VersionFormato {
            pedida: pedida.to_string(),
            soportada: VERSION_FORMATO.to_string(),
        })
    }
}

fn convertir(
    crudo: ArchivoCrudo,
    fallos: &mut Vec<FalloValidacion>,
) -> Result<Maquina, ErrorArquitectura> {
    let m = crudo.maquina;

    let endianness = match m.endianness.as_str() {
        "little" => Endianness::Little,
        "big" => Endianness::Big,
        otro => {
            fallos.push(fallo(
                Regla::ValorFueraDeRango,
                format!("endianness '{otro}' desconocido; se esperaba 'little' o 'big'"),
            ));
            Endianness::Little
        }
    };

    let pila = match m.pila.as_str() {
        "descendente" => SentidoPila::Descendente,
        "ascendente" => SentidoPila::Ascendente,
        otro => {
            fallos.push(fallo(
                Regla::ValorFueraDeRango,
                format!("sentido de pila '{otro}' desconocido; se esperaba 'descendente' o 'ascendente'"),
            ));
            SentidoPila::Descendente
        }
    };

    let registros = crudo
        .registros
        .into_iter()
        .map(|r| {
            let rol = match r.rol.as_str() {
                "general" => RolRegistro::General,
                "pc" => RolRegistro::ContadorPrograma,
                "sp" => RolRegistro::PunteroPila,
                "flags" => RolRegistro::Banderas,
                "indirecto" => RolRegistro::Indirecto,
                "fijo" => RolRegistro::Fijo,
                otro => {
                    fallos.push(fallo(
                        Regla::ValorFueraDeRango,
                        format!(
                            "el registro '{}' declara el rol desconocido '{}'",
                            r.nombre, otro
                        ),
                    ));
                    RolRegistro::General
                }
            };
            Registro {
                nombre: r.nombre,
                bits: r.bits,
                rol,
                inicial: r.inicial,
            }
        })
        .collect();

    let banderas = crudo
        .flags
        .into_iter()
        .map(|(nombre, b)| Bandera {
            nombre,
            bit: b.bit,
            descripcion: b.descripcion,
        })
        .collect();

    let mut instrucciones = Vec::new();
    for i in crudo.instrucciones {
        let codificacion =
            Codificacion::analizar(&i.codigo).map_err(|e| ErrorArquitectura::Codificacion {
                mnemonico: i.mnemonico.clone(),
                causa: e.to_string(),
            })?;

        let operandos = i
            .operandos
            .into_iter()
            .map(|o| {
                let tipo = match o.tipo.as_str() {
                    "registro" => TipoOperando::Registro { permitidos: o.de },
                    "inmediato" => TipoOperando::Inmediato {
                        signo: o.signo,
                        rango: o.rango.map(|r| (r[0], r[1])).unwrap_or((0, 0)),
                    },
                    "direccion" => TipoOperando::Direccion,
                    "desplazamiento" => TipoOperando::Desplazamiento,
                    "puerto" => TipoOperando::Puerto,
                    otro => {
                        fallos.push(fallo(
                            Regla::ValorFueraDeRango,
                            format!(
                                "el operando '{}' de '{}' declara el tipo desconocido '{}'",
                                o.nombre, i.mnemonico, otro
                            ),
                        ));
                        TipoOperando::Direccion
                    }
                };
                Operando {
                    nombre: o.nombre,
                    tipo,
                }
            })
            .collect();

        let ciclos = convertir_ciclos(i.ciclos.as_ref(), &i.mnemonico, fallos);

        let sintaxis = if i.sintaxis.is_empty() {
            i.mnemonico.clone()
        } else {
            i.sintaxis
        };

        let programa = match semantica::analizar(&i.semantica) {
            Ok(p) => p,
            Err(e) => {
                fallos.push(fallo(
                    Regla::SemanticaInvalida,
                    format!("la semantica de '{}' no es valida: {}", i.mnemonico, e),
                ));
                semantica::Programa::default()
            }
        };

        instrucciones.push(Instruccion {
            mnemonico: i.mnemonico,
            sintaxis,
            codificacion,
            operandos,
            semantica: i.semantica,
            programa,
            afecta: i.afecta,
            ciclos,
            descripcion: i.descripcion,
        });
    }

    let regiones = crudo
        .regiones
        .into_iter()
        .map(|r| {
            let acceso = match r.acceso.as_str() {
                "r" => Acceso {
                    lectura: true,
                    escritura: false,
                },
                "w" => Acceso {
                    lectura: false,
                    escritura: true,
                },
                "rw" => Acceso {
                    lectura: true,
                    escritura: true,
                },
                otro => {
                    fallos.push(fallo(
                        Regla::ValorFueraDeRango,
                        format!(
                            "la region '{}' declara el acceso desconocido '{}'",
                            r.nombre, otro
                        ),
                    ));
                    Acceso {
                        lectura: true,
                        escritura: true,
                    }
                }
            };
            Region {
                nombre: r.nombre,
                desde: r.desde,
                hasta: r.hasta,
                acceso,
                dispositivo: r.dispositivo,
                contenido: r.contenido,
            }
        })
        .collect();

    let dispositivos = crudo
        .dispositivos
        .into_iter()
        .map(|d| {
            let tipo = match d.tipo.as_str() {
                "pantalla" => TipoDispositivo::Pantalla {
                    ancho: d.ancho.unwrap_or(64),
                    alto: d.alto.unwrap_or(32),
                    formato: d.formato.unwrap_or_else(|| "indice1".to_string()),
                },
                "teclado" => TipoDispositivo::Teclado,
                "uart" => TipoDispositivo::Uart,
                "temporizador" => TipoDispositivo::Temporizador {
                    periodo: d.periodo.unwrap_or(1000),
                },
                "bloques" => TipoDispositivo::Bloques {
                    tamano_bloque: d.tamano_bloque.unwrap_or(512),
                    cantidad_bloques: d.cantidad_bloques.unwrap_or(64),
                },
                "generico" => TipoDispositivo::Generico {
                    script: d.script.unwrap_or_default(),
                },
                otro => {
                    fallos.push(fallo(
                        Regla::ValorFueraDeRango,
                        format!(
                            "el dispositivo '{}' declara el tipo desconocido '{}'",
                            d.nombre, otro
                        ),
                    ));
                    TipoDispositivo::Generico {
                        script: String::new(),
                    }
                }
            };
            Dispositivo {
                nombre: d.nombre,
                tipo,
                puertos: d.puertos,
                interrupcion: d.interrupcion,
            }
        })
        .collect();

    let interrupciones = m.interrupciones.map(|i| Interrupciones {
        vectores: i
            .vectores
            .into_iter()
            .filter_map(|(k, v)| k.parse::<u32>().ok().map(|linea| (linea, v)))
            .collect(),
        prioridades: i
            .prioridades
            .into_iter()
            .filter_map(|(k, v)| k.parse::<u32>().ok().map(|linea| (linea, v)))
            .collect(),
        habilitadas_al_arrancar: i.habilitadas_al_arrancar,
    });

    Ok(Maquina {
        nombre: m.nombre,
        formato: m.formato,
        bits_palabra: m.bits_palabra,
        bits_direccion: m.bits_direccion,
        endianness,
        arranque: m.arranque,
        pila,
        registros,
        banderas,
        instrucciones,
        regiones,
        dispositivos,
        interrupciones,
    })
}

fn convertir_ciclos(
    valor: Option<&toml::Value>,
    mnemonico: &str,
    fallos: &mut Vec<FalloValidacion>,
) -> Ciclos {
    match valor {
        None => Ciclos::Fijo(1),
        Some(toml::Value::Integer(n)) if *n >= 0 => Ciclos::Fijo(*n as u32),
        Some(toml::Value::Table(t)) => {
            let leer = |clave: &str| t.get(clave).and_then(|v| v.as_integer()).unwrap_or(1) as u32;
            Ciclos::SegunSalto {
                tomado: leer("tomado"),
                no_tomado: leer("no_tomado"),
            }
        }
        Some(_) => {
            fallos.push(fallo(
                Regla::ValorFueraDeRango,
                format!("el coste en ciclos de '{mnemonico}' no es un numero ni una tabla tomado/no_tomado"),
            ));
            Ciclos::Fijo(1)
        }
    }
}

fn fallo(regla: Regla, detalle: String) -> FalloValidacion {
    FalloValidacion {
        regla,
        detalle,
        posicion: None,
    }
}
