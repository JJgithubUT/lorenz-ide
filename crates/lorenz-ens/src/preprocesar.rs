//! Preprocesado: inclusion de archivos y macros con parametros (RF-ENS-008, RF-ENS-009).
//!
//! Convierte un fuente con `.incluir` y `.macro` en una lista plana de lineas que el ensamblador
//! trata como cualquier otra. Cada linea conserva el archivo y el numero donde nacio; una linea
//! que viene de una macro se atribuye a la linea que la invoco, que es lo que el programador ve.
//!
//! ```text
//! .incluir "otros.s"          ; ruta relativa al archivo que incluye
//!
//! .macro CARGA2 a, b, valor   ; parametros separados por comas
//!     LDI \a, \valor
//!     LDI \b, \valor
//! .endm
//!
//!     CARGA2 A, B, 7          ; se expande a dos LDI
//! ```
//!
//! Dentro de una macro, `\nombre` se sustituye por el argumento y `\@` por un numero unico de cada
//! expansion, para poder declarar etiquetas locales (`bucle\@:`).

use std::collections::BTreeMap;
use std::path::Path;

use crate::ensamblador::Fuente;
use crate::error::{Causa, ErrorEns};
use crate::fuente::{analizar_linea, dividir_argumentos, Sentencia};

/// Profundidad maxima de inclusiones anidadas.
pub const PROFUNDIDAD_INCLUSION: usize = 32;
/// Profundidad maxima de macros que invocan macros.
pub const PROFUNDIDAD_MACROS: usize = 16;

/// Una linea ya preprocesada: `(archivo, numero, texto)`.
pub type LineaPlana = (String, usize, String);

/// El resultado de preprocesar: las lineas planas y todos los archivos leidos, el principal primero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preprocesado {
    pub lineas: Vec<LineaPlana>,
    pub fuentes: Vec<Fuente>,
}

/// De donde salen los archivos incluidos.
pub trait Cargador {
    /// Lee `ruta`, interpretada desde el archivo `desde`. Devuelve el nombre con el que se citara
    /// y su texto.
    fn leer(&self, desde: &str, ruta: &str) -> Result<(String, String), String>;
}

/// Lee del disco; las rutas relativas se miden desde la carpeta del archivo que incluye.
pub struct CargadorDeDisco;

impl Cargador for CargadorDeDisco {
    fn leer(&self, desde: &str, ruta: &str) -> Result<(String, String), String> {
        let base = Path::new(desde).parent().unwrap_or_else(|| Path::new(""));
        let completa = base.join(ruta);
        let nombre = completa.to_string_lossy().into_owned();
        std::fs::read_to_string(&completa)
            .map(|t| (nombre, t))
            .map_err(|e| e.to_string())
    }
}

/// Archivos en memoria, para pruebas y para quien ya tiene el texto.
#[derive(Default)]
pub struct CargadorEnMemoria {
    pub archivos: BTreeMap<String, String>,
}

impl Cargador for CargadorEnMemoria {
    fn leer(&self, _desde: &str, ruta: &str) -> Result<(String, String), String> {
        self.archivos
            .get(ruta)
            .map(|t| (ruta.to_string(), t.clone()))
            .ok_or_else(|| "no existe".to_string())
    }
}

#[derive(Debug, Clone)]
struct Macro {
    parametros: Vec<String>,
    cuerpo: Vec<String>,
    definida_en: (String, usize),
}

struct Estado<'a> {
    fuentes: Vec<Fuente>,
    cargador: &'a dyn Cargador,
    reservadas: Vec<String>,
    macros: BTreeMap<String, Macro>,
    salida: Vec<LineaPlana>,
    errores: Vec<ErrorEns>,
    contador_expansiones: usize,
}

fn error(archivo: &str, linea: usize, columna: usize, causa: Causa) -> ErrorEns {
    ErrorEns {
        archivo: archivo.to_string(),
        linea,
        columna,
        causa,
    }
}

/// Deja un fuente listo para ensamblar.
///
/// `mnemonicos` son los nombres de instruccion de la maquina: una macro no puede llamarse igual.
pub fn preprocesar(
    raiz: &Fuente,
    cargador: &dyn Cargador,
    mnemonicos: &[&str],
) -> Result<Preprocesado, Vec<ErrorEns>> {
    let mut estado = Estado {
        fuentes: vec![raiz.clone()],
        cargador,
        reservadas: mnemonicos.iter().map(|m| m.to_ascii_lowercase()).collect(),
        macros: BTreeMap::new(),
        salida: Vec::new(),
        errores: Vec::new(),
        contador_expansiones: 0,
    };
    let mut pila = vec![raiz.nombre.clone()];
    estado.archivo(&raiz.nombre, &raiz.texto, &mut pila);
    if estado.errores.is_empty() {
        Ok(Preprocesado {
            lineas: estado.salida,
            fuentes: estado.fuentes,
        })
    } else {
        Err(estado.errores)
    }
}

impl Estado<'_> {
    fn archivo(&mut self, nombre: &str, texto: &str, pila: &mut Vec<String>) {
        let lineas: Vec<&str> = texto.lines().collect();
        let mut i = 0;
        while i < lineas.len() {
            let numero = i + 1;
            let linea = lineas[i];
            i += 1;
            let analizada = match analizar_linea(numero, linea) {
                Ok(l) => l,
                Err(_) => {
                    // El ensamblador reportara el error de sintaxis con su columna.
                    self.salida
                        .push((nombre.to_string(), numero, linea.to_string()));
                    continue;
                }
            };
            match &analizada.sentencia {
                Some(Sentencia::Directiva {
                    nombre: directiva,
                    argumentos,
                }) => {
                    let d = directiva.texto.to_ascii_lowercase();
                    match d.as_str() {
                        "incluir" | "include" => {
                            self.etiquetas_previas(nombre, numero, &analizada.etiquetas);
                            self.incluir(nombre, numero, directiva.columna, argumentos, pila);
                        }
                        "macro" => {
                            let (fin, cuerpo) = recoger_cuerpo(&lineas, i);
                            self.definir_macro(
                                nombre,
                                numero,
                                directiva.columna,
                                argumentos,
                                &cuerpo,
                                terminada(&lineas, fin),
                            );
                            i = fin + usize::from(terminada(&lineas, fin));
                        }
                        "endm" | "endmacro" | "fmacro" => self.errores.push(error(
                            nombre,
                            numero,
                            directiva.columna,
                            Causa::Sintaxis(format!(
                                "`.{}` sin un `.macro` abierto",
                                directiva.texto
                            )),
                        )),
                        _ => self
                            .salida
                            .push((nombre.to_string(), numero, linea.to_string())),
                    }
                }
                Some(Sentencia::Instruccion {
                    mnemonico,
                    operandos,
                }) if self
                    .macros
                    .contains_key(&mnemonico.texto.to_ascii_lowercase()) =>
                {
                    self.etiquetas_previas(nombre, numero, &analizada.etiquetas);
                    let args = if operandos.texto.trim().is_empty() {
                        Vec::new()
                    } else {
                        dividir_argumentos(&operandos.texto, operandos.columna)
                            .into_iter()
                            .map(|t| t.texto)
                            .collect()
                    };
                    self.expandir(
                        nombre,
                        numero,
                        mnemonico.columna,
                        &mnemonico.texto,
                        &args,
                        1,
                    );
                }
                _ => self
                    .salida
                    .push((nombre.to_string(), numero, linea.to_string())),
            }
        }
    }

    /// Las etiquetas de una linea de directiva o de macro se conservan como una linea propia.
    fn etiquetas_previas(
        &mut self,
        archivo: &str,
        numero: usize,
        etiquetas: &[crate::fuente::Texto],
    ) {
        if etiquetas.is_empty() {
            return;
        }
        let texto: Vec<String> = etiquetas.iter().map(|e| format!("{}:", e.texto)).collect();
        self.salida
            .push((archivo.to_string(), numero, texto.join(" ")));
    }

    fn incluir(
        &mut self,
        archivo: &str,
        numero: usize,
        columna: usize,
        argumentos: &[crate::fuente::Texto],
        pila: &mut Vec<String>,
    ) {
        let [arg] = argumentos else {
            self.errores.push(error(
                archivo,
                numero,
                columna,
                Causa::ArgumentosDeDirectiva {
                    directiva: ".incluir".into(),
                    esperado: "una ruta entre comillas".into(),
                },
            ));
            return;
        };
        let t = arg.texto.trim();
        if t.len() < 2 || !t.starts_with('"') || !t.ends_with('"') {
            self.errores.push(error(
                archivo,
                numero,
                arg.columna,
                Causa::ArgumentosDeDirectiva {
                    directiva: ".incluir".into(),
                    esperado: "una ruta entre comillas".into(),
                },
            ));
            return;
        }
        let ruta = &t[1..t.len() - 1];
        if pila.len() >= PROFUNDIDAD_INCLUSION {
            self.errores.push(error(
                archivo,
                numero,
                arg.columna,
                Causa::Otra(format!(
                    "demasiadas inclusiones anidadas (maximo {PROFUNDIDAD_INCLUSION})"
                )),
            ));
            return;
        }
        match self.cargador.leer(archivo, ruta) {
            Err(motivo) => self.errores.push(error(
                archivo,
                numero,
                arg.columna,
                Causa::NoSePudoLeer {
                    archivo: ruta.to_string(),
                    motivo,
                },
            )),
            Ok((nombre, texto)) => {
                if pila.contains(&nombre) {
                    self.errores.push(error(
                        archivo,
                        numero,
                        arg.columna,
                        Causa::InclusionCircular { archivo: nombre },
                    ));
                    return;
                }
                pila.push(nombre.clone());
                if !self.fuentes.iter().any(|f| f.nombre == nombre) {
                    self.fuentes
                        .push(Fuente::nueva(nombre.clone(), texto.clone()));
                }
                self.archivo(&nombre, &texto, pila);
                pila.pop();
            }
        }
    }

    fn definir_macro(
        &mut self,
        archivo: &str,
        numero: usize,
        columna: usize,
        argumentos: &[crate::fuente::Texto],
        cuerpo: &[String],
        cerrada: bool,
    ) {
        if !cerrada {
            self.errores.push(error(
                archivo,
                numero,
                columna,
                Causa::Sintaxis("la macro no termina: falta `.endm`".into()),
            ));
            return;
        }
        let Some((nombre, parametros)) = argumentos.split_first() else {
            self.errores.push(error(
                archivo,
                numero,
                columna,
                Causa::ArgumentosDeDirectiva {
                    directiva: ".macro".into(),
                    esperado: "un nombre y, opcionalmente, parametros".into(),
                },
            ));
            return;
        };
        // `.macro NOMBRE a, b` llega como un unico argumento "NOMBRE a" y luego "b".
        let (nombre_macro, primero) = match nombre.texto.split_once(char::is_whitespace) {
            Some((n, resto)) => (n.to_string(), Some(resto.trim().to_string())),
            None => (nombre.texto.clone(), None),
        };
        let mut params: Vec<String> = primero.into_iter().filter(|p| !p.is_empty()).collect();
        params.extend(parametros.iter().map(|p| p.texto.clone()));
        let clave = nombre_macro.to_ascii_lowercase();
        if self.reservadas.contains(&clave) {
            self.errores.push(error(
                archivo,
                numero,
                nombre.columna,
                Causa::Otra(format!(
                    "`{nombre_macro}` ya es una instruccion de la maquina y no puede ser una macro"
                )),
            ));
            return;
        }
        if let Some(previa) = self.macros.get(&clave) {
            self.errores.push(error(
                archivo,
                numero,
                nombre.columna,
                Causa::Otra(format!(
                    "la macro `{nombre_macro}` ya se definio en {}:{}",
                    previa.definida_en.0, previa.definida_en.1
                )),
            ));
            return;
        }
        for p in &params {
            let valido = !p.is_empty()
                && p.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !p.chars().next().unwrap().is_ascii_digit();
            if !valido {
                self.errores.push(error(
                    archivo,
                    numero,
                    nombre.columna,
                    Causa::Sintaxis(format!("`{p}` no es un nombre de parametro valido")),
                ));
                return;
            }
        }
        self.macros.insert(
            clave,
            Macro {
                parametros: params,
                cuerpo: cuerpo.to_vec(),
                definida_en: (archivo.to_string(), numero),
            },
        );
    }

    fn expandir(
        &mut self,
        archivo: &str,
        numero: usize,
        columna: usize,
        nombre: &str,
        args: &[String],
        profundidad: usize,
    ) {
        let Some(m) = self.macros.get(&nombre.to_ascii_lowercase()).cloned() else {
            return;
        };
        if profundidad > PROFUNDIDAD_MACROS {
            self.errores.push(error(
                archivo,
                numero,
                columna,
                Causa::Otra(format!(
                    "la macro `{nombre}` se expande a si misma sin terminar (maximo {PROFUNDIDAD_MACROS} niveles)"
                )),
            ));
            return;
        }
        if args.len() != m.parametros.len() {
            self.errores.push(error(
                archivo,
                numero,
                columna,
                Causa::Otra(format!(
                    "la macro `{nombre}` recibe {} argumento(s) y se le pasaron {}",
                    m.parametros.len(),
                    args.len()
                )),
            ));
            return;
        }
        self.contador_expansiones += 1;
        let id = self.contador_expansiones;
        for linea in &m.cuerpo {
            let mut texto = linea.clone();
            // Los parametros mas largos primero: `\ab` no debe romperse por un parametro `a`.
            let mut orden: Vec<usize> = (0..m.parametros.len()).collect();
            orden.sort_by_key(|&k| std::cmp::Reverse(m.parametros[k].len()));
            for k in orden {
                texto = texto.replace(&format!("\\{}", m.parametros[k]), &args[k]);
            }
            texto = texto.replace("\\@", &format!("{id}"));
            // Una linea del cuerpo puede invocar otra macro.
            if let Ok(l) = analizar_linea(numero, &texto) {
                if let Some(Sentencia::Instruccion {
                    mnemonico,
                    operandos,
                }) = &l.sentencia
                {
                    if self
                        .macros
                        .contains_key(&mnemonico.texto.to_ascii_lowercase())
                    {
                        if !l.etiquetas.is_empty() {
                            let e: Vec<String> = l
                                .etiquetas
                                .iter()
                                .map(|e| format!("{}:", e.texto))
                                .collect();
                            self.salida.push((archivo.to_string(), numero, e.join(" ")));
                        }
                        let a: Vec<String> = if operandos.texto.trim().is_empty() {
                            Vec::new()
                        } else {
                            dividir_argumentos(&operandos.texto, 1)
                                .into_iter()
                                .map(|t| t.texto)
                                .collect()
                        };
                        let n = mnemonico.texto.clone();
                        self.expandir(archivo, numero, columna, &n, &a, profundidad + 1);
                        continue;
                    }
                }
            }
            self.salida.push((archivo.to_string(), numero, texto));
        }
    }
}

/// Lineas del cuerpo de una macro desde `desde` (indice base 0) hasta su `.endm`.
/// Devuelve el indice de la linea del `.endm` (o el final) y el cuerpo.
fn recoger_cuerpo(lineas: &[&str], desde: usize) -> (usize, Vec<String>) {
    let mut cuerpo = Vec::new();
    let mut j = desde;
    while j < lineas.len() {
        if es_fin_de_macro(lineas[j]) {
            return (j, cuerpo);
        }
        cuerpo.push(lineas[j].to_string());
        j += 1;
    }
    (j, cuerpo)
}

fn es_fin_de_macro(linea: &str) -> bool {
    matches!(
        analizar_linea(0, linea),
        Ok(l) if matches!(
            &l.sentencia,
            Some(Sentencia::Directiva { nombre, .. })
                if matches!(nombre.texto.to_ascii_lowercase().as_str(), "endm" | "endmacro" | "fmacro")
        )
    )
}

fn terminada(lineas: &[&str], fin: usize) -> bool {
    fin < lineas.len() && es_fin_de_macro(lineas[fin])
}
