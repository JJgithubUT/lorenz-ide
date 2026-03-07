//! El ensamblador: de texto fuente a palabras de memoria.
//!
//! Trabaja en dos pasadas (RF-ENS-003). La primera recorre las lineas, fija la direccion de cada
//! elemento y define los simbolos; como el tamano de una instruccion sale solo de su codificacion,
//! no hace falta conocer los valores para saber donde cae cada etiqueta. La segunda evalua las
//! expresiones con todos los simbolos ya conocidos y codifica.
//!
//! Los errores de cada linea se acumulan y se devuelven juntos (RF-ENS-015): un error no impide ver
//! los siguientes. Si la primera pasada falla no se intenta la segunda, porque las direcciones ya no
//! serian fiables y los errores que salieran de ahi serian ruido.

use std::collections::BTreeMap;

use lorenz_arch::modelo::{Endianness, Instruccion, Maquina, TipoOperando};
use lorenz_arch::plantilla::Plantilla;

use crate::codificar::codificar;
use crate::error::{Causa, ErrorEns};
use crate::expr::{Entorno, Expr};
use crate::fuente::{analizar_linea, Sentencia, Texto};

/// Un archivo fuente con nombre, para poder citarlo en los errores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fuente {
    pub nombre: String,
    pub texto: String,
}

impl Fuente {
    pub fn nueva(nombre: impl Into<String>, texto: impl Into<String>) -> Self {
        Fuente {
            nombre: nombre.into(),
            texto: texto.into(),
        }
    }
}

/// Palabras consecutivas que empiezan en `direccion`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segmento {
    pub direccion: u64,
    pub palabras: Vec<u64>,
}

/// De donde salio cada tramo de memoria (RF-ENS-017).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntradaMapa {
    pub direccion: u64,
    pub longitud: usize,
    pub archivo: String,
    pub linea: usize,
}

/// Un simbolo definido por el programa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Simbolo {
    pub valor: i128,
    /// `true` si es una etiqueta (una direccion), `false` si es una constante.
    pub etiqueta: bool,
    pub archivo: String,
    pub linea: usize,
}

/// Un simbolo que este archivo usa pero define otro (`.externo`), listo para que lo resuelva el
/// enlazador (RF-ENS-019).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Externo {
    pub archivo: String,
    pub linea: usize,
}

/// Una instruccion que quedo sin codificar porque usa un simbolo externo. Lleva el texto fuente
/// para que el enlazador la ensamble de nuevo, en la misma direccion, cuando ya conozca el valor
/// (RF-ENS-019). Como el tamano de una instruccion no depende de sus valores, el hueco que deja en
/// el segmento tiene el tamano exacto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reubicacion {
    pub direccion: u64,
    pub longitud: usize,
    pub archivo: String,
    pub linea: usize,
    pub fuente: String,
    /// Simbolos externos que la instruccion necesita.
    pub simbolos: Vec<String>,
}

/// El resultado de ensamblar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Objeto {
    pub segmentos: Vec<Segmento>,
    pub simbolos: BTreeMap<String, Simbolo>,
    pub mapa: Vec<EntradaMapa>,
    /// Simbolos declarados con `.externo`.
    pub externos: BTreeMap<String, Externo>,
    /// Instrucciones pendientes de enlazar.
    pub reubicaciones: Vec<Reubicacion>,
    /// Cosas dudosas que no impiden ensamblar (RF-ENS-028).
    pub avisos: Vec<Aviso>,
}

/// Un aviso del ensamblador: el programa se ensambla, pero algo merece una segunda mirada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aviso {
    pub archivo: String,
    pub linea: usize,
    pub mensaje: String,
}

impl std::fmt::Display for Aviso {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: aviso: {}",
            self.archivo, self.linea, self.mensaje
        )
    }
}

impl Objeto {
    /// Todas las palabras emitidas como pares `(direccion, palabra)`, en orden de direccion.
    pub fn palabras(&self) -> Vec<(u64, u64)> {
        let mut v: Vec<(u64, u64)> = self
            .segmentos
            .iter()
            .flat_map(|s| {
                s.palabras
                    .iter()
                    .enumerate()
                    .map(move |(i, p)| (s.direccion + i as u64, *p))
            })
            .collect();
        v.sort_unstable();
        v
    }

    /// Cuantas palabras ocupa el programa.
    pub fn tamano(&self) -> usize {
        self.segmentos.iter().map(|s| s.palabras.len()).sum()
    }

    /// Valor de un simbolo, si existe.
    pub fn simbolo(&self, nombre: &str) -> Option<i128> {
        self.simbolos.get(nombre).map(|s| s.valor)
    }
}

#[derive(Debug, Clone)]
struct Forma {
    indice: usize,
    plantilla: Plantilla,
}

/// Un ensamblador ya generado para una maquina concreta (RF-ENS-001).
#[derive(Debug, Clone)]
pub struct Ensamblador<'m> {
    maquina: &'m Maquina,
    formas: Vec<Forma>,
}

/// Secciones con nombre: cada una continua donde la dejo la ultima vez que se uso (RF-ENS-010).
#[derive(Debug, Default)]
struct Secciones {
    actual: Option<String>,
    guardadas: BTreeMap<String, u64>,
}

#[derive(Debug)]
struct Origen {
    archivo: String,
    linea: usize,
}

#[derive(Debug)]
enum Contenido {
    Instruccion {
        forma: usize,
        operandos: Vec<(String, Texto, Option<Expr>)>,
        /// Mnemonico y operandos tal como se escribieron, por si hay que ensamblarla otra vez.
        fuente: String,
    },
    Datos(Vec<DatoFuente>),
    Direcciones(Vec<(Texto, Expr)>),
    Espacio {
        cantidad: usize,
        relleno: Option<(Texto, Expr)>,
    },
}

#[derive(Debug)]
enum DatoFuente {
    Valor(Texto, Expr),
    Cadena(Vec<u64>),
}

#[derive(Debug)]
struct Elemento {
    origen: Origen,
    direccion: u64,
    contenido: Contenido,
}

struct Tabla<'a> {
    simbolos: &'a BTreeMap<String, Simbolo>,
    aqui: i128,
}

impl Entorno for Tabla<'_> {
    fn simbolo(&self, nombre: &str) -> Option<i128> {
        self.simbolos.get(nombre).map(|s| s.valor)
    }
    fn aqui(&self) -> i128 {
        self.aqui
    }
}

fn palabras_por_direccion(m: &Maquina) -> usize {
    m.bits_direccion.div_ceil(m.bits_palabra) as usize
}

impl<'m> Ensamblador<'m> {
    /// Genera el ensamblador a partir de la descripcion de la maquina.
    pub fn nuevo(maquina: &'m Maquina) -> Result<Self, String> {
        let mut formas = Vec::new();
        for (indice, ins) in maquina.instrucciones.iter().enumerate() {
            let plantilla = Plantilla::analizar(&ins.sintaxis)
                .map_err(|e| format!("la sintaxis de `{}` no es valida: {e}", ins.mnemonico))?;
            formas.push(Forma { indice, plantilla });
        }
        Ok(Ensamblador { maquina, formas })
    }

    pub fn maquina(&self) -> &Maquina {
        self.maquina
    }

    /// Mnemonicos que el ensamblador reconoce, sin repetir y en orden de declaracion.
    pub fn mnemonicos(&self) -> Vec<&str> {
        let mut v: Vec<&str> = Vec::new();
        for f in &self.formas {
            let m = f.plantilla.mnemonico();
            if !v.iter().any(|x| x.eq_ignore_ascii_case(m)) {
                v.push(m);
            }
        }
        v
    }

    /// Ensambla un programa de un solo archivo. Admite macros; `.incluir` solo funciona si se usa
    /// [`Ensamblador::ensamblar_con`] con un cargador que sepa leer los archivos.
    pub fn ensamblar(&self, fuente: &Fuente) -> Result<Objeto, Vec<ErrorEns>> {
        let sin_archivos = crate::preprocesar::CargadorEnMemoria::default();
        self.ensamblar_con(fuente, &sin_archivos).map(|(o, _)| o)
    }

    /// Ensambla un programa con `.incluir` y macros. Devuelve tambien todos los archivos leidos,
    /// el principal primero, para generar el listado.
    pub fn ensamblar_con(
        &self,
        raiz: &Fuente,
        cargador: &dyn crate::preprocesar::Cargador,
    ) -> Result<(Objeto, Vec<Fuente>), Vec<ErrorEns>> {
        let pre = crate::preprocesar::preprocesar(raiz, cargador, &self.mnemonicos())?;
        let objeto = self.ensamblar_lineas(&pre.lineas)?;
        Ok((objeto, pre.fuentes))
    }

    /// Ensambla lineas ya preprocesadas: `(archivo, numero, texto)`.
    pub fn ensamblar_lineas(
        &self,
        lineas: &[(String, usize, String)],
    ) -> Result<Objeto, Vec<ErrorEns>> {
        let mut errores = Vec::new();
        let mut simbolos: BTreeMap<String, Simbolo> = BTreeMap::new();
        let mut externos: BTreeMap<String, Externo> = BTreeMap::new();
        let elementos = self.primera_pasada(lineas, &mut simbolos, &mut externos, &mut errores);
        for (nombre, e) in &externos {
            if let Some(s) = simbolos.get(nombre) {
                errores.push(ErrorEns {
                    archivo: e.archivo.clone(),
                    linea: e.linea,
                    columna: 1,
                    causa: Causa::Otra(format!(
                        "`{nombre}` se declara externo y tambien se define en {}:{}",
                        s.archivo, s.linea
                    )),
                });
            }
        }
        if !errores.is_empty() {
            return Err(errores);
        }
        let avisos = self.avisar_banderas(&elementos, &simbolos);
        let mut objeto = self.segunda_pasada(elementos, simbolos, &externos, &mut errores);
        objeto.externos = externos;
        objeto.avisos = avisos;
        if errores.is_empty() {
            Ok(objeto)
        } else {
            Err(errores)
        }
    }

    /// Ensambla una sola instruccion escrita como texto (`LDI A, K`) en `direccion`, con los
    /// simbolos dados. Es lo que usa el enlazador para completar las reubicaciones (RF-ENS-019).
    pub fn ensamblar_instruccion_en(
        &self,
        fuente: &str,
        direccion: u64,
        simbolos: &BTreeMap<String, i128>,
        archivo: &str,
        linea: usize,
    ) -> Result<Vec<u64>, Vec<ErrorEns>> {
        let origen = Origen {
            archivo: archivo.to_string(),
            linea,
        };
        let mut errores = Vec::new();
        let analizada = match analizar_linea(linea, fuente) {
            Ok(l) => l,
            Err((col, causa)) => return Err(vec![Self::error(&origen, col, causa)]),
        };
        let Some(Sentencia::Instruccion {
            mnemonico,
            operandos,
        }) = analizada.sentencia
        else {
            return Err(vec![Self::error(
                &origen,
                1,
                Causa::Otra(format!("`{fuente}` no es una instruccion")),
            )]);
        };
        let Some((contenido, _)) = self.instruccion(&origen, &mnemonico, &operandos, &mut errores)
        else {
            return Err(errores);
        };
        let tabla_simbolos: BTreeMap<String, Simbolo> = simbolos
            .iter()
            .map(|(n, v)| {
                (
                    n.clone(),
                    Simbolo {
                        valor: *v,
                        etiqueta: false,
                        archivo: archivo.to_string(),
                        linea,
                    },
                )
            })
            .collect();
        let el = Elemento {
            origen,
            direccion,
            contenido,
        };
        let tabla = Tabla {
            simbolos: &tabla_simbolos,
            aqui: direccion as i128,
        };
        match self.emitir(&el, &tabla, &mut errores) {
            Some(p) if errores.is_empty() => Ok(p),
            _ => Err(errores),
        }
    }

    fn error(origen: &Origen, columna: usize, causa: Causa) -> ErrorEns {
        ErrorEns {
            archivo: origen.archivo.clone(),
            linea: origen.linea,
            columna,
            causa,
        }
    }

    fn definir(
        simbolos: &mut BTreeMap<String, Simbolo>,
        origen: &Origen,
        nombre: &Texto,
        valor: i128,
        etiqueta: bool,
        errores: &mut Vec<ErrorEns>,
    ) {
        if let Some(previo) = simbolos.get(&nombre.texto) {
            errores.push(Self::error(
                origen,
                nombre.columna,
                Causa::EtiquetaDuplicada {
                    nombre: nombre.texto.clone(),
                    previa: previo.linea,
                },
            ));
            return;
        }
        simbolos.insert(
            nombre.texto.clone(),
            Simbolo {
                valor,
                etiqueta,
                archivo: origen.archivo.clone(),
                linea: origen.linea,
            },
        );
    }

    fn evaluar_ahora(
        simbolos: &BTreeMap<String, Simbolo>,
        origen: &Origen,
        texto: &Texto,
        aqui: u64,
        errores: &mut Vec<ErrorEns>,
    ) -> Option<i128> {
        let expr = match Expr::analizar(&texto.texto) {
            Ok(e) => e,
            Err(e) => {
                errores.push(Self::error(origen, texto.columna, Causa::Expresion(e)));
                return None;
            }
        };
        match expr.evaluar(&Tabla {
            simbolos,
            aqui: aqui as i128,
        }) {
            Ok(v) => Some(v),
            Err(e) => {
                errores.push(Self::error(origen, texto.columna, Causa::Expresion(e)));
                None
            }
        }
    }

    fn primera_pasada(
        &self,
        lineas: &[(String, usize, String)],
        simbolos: &mut BTreeMap<String, Simbolo>,
        externos: &mut BTreeMap<String, Externo>,
        errores: &mut Vec<ErrorEns>,
    ) -> Vec<Elemento> {
        let m = self.maquina;
        let mut elementos = Vec::new();
        let mut dir = m.arranque;
        let mut secciones = Secciones::default();
        for (archivo, numero, texto) in lineas {
            let origen = Origen {
                archivo: archivo.clone(),
                linea: *numero,
            };
            let linea = match analizar_linea(*numero, texto) {
                Ok(l) => l,
                Err((col, causa)) => {
                    errores.push(Self::error(&origen, col, causa));
                    continue;
                }
            };
            for etiqueta in &linea.etiquetas {
                Self::definir(simbolos, &origen, etiqueta, dir as i128, true, errores);
            }
            let Some(sentencia) = linea.sentencia else {
                continue;
            };
            match sentencia {
                Sentencia::Constante { nombre, valor } => {
                    if let Some(v) = Self::evaluar_ahora(simbolos, &origen, &valor, dir, errores) {
                        Self::definir(simbolos, &origen, &nombre, v, false, errores);
                    }
                }
                Sentencia::Directiva { nombre, argumentos }
                    if matches!(
                        nombre.texto.to_ascii_lowercase().as_str(),
                        "externo" | "extern"
                    ) =>
                {
                    if argumentos.is_empty() {
                        errores.push(Self::error(
                            &origen,
                            nombre.columna,
                            Causa::ArgumentosDeDirectiva {
                                directiva: format!(".{}", nombre.texto),
                                esperado: "uno o mas simbolos".to_string(),
                            },
                        ));
                    }
                    for a in &argumentos {
                        externos.entry(a.texto.clone()).or_insert(Externo {
                            archivo: origen.archivo.clone(),
                            linea: origen.linea,
                        });
                    }
                }
                Sentencia::Directiva { nombre, argumentos } => {
                    self.directiva(
                        &origen,
                        &nombre,
                        &argumentos,
                        &mut dir,
                        &mut secciones,
                        simbolos,
                        &mut elementos,
                        errores,
                    );
                }
                Sentencia::Instruccion {
                    mnemonico,
                    operandos,
                } => {
                    if let Some((contenido, longitud)) =
                        self.instruccion(&origen, &mnemonico, &operandos, errores)
                    {
                        elementos.push(Elemento {
                            origen,
                            direccion: dir,
                            contenido,
                        });
                        dir = dir.wrapping_add(longitud as u64);
                    }
                }
            }
        }
        elementos
    }

    #[allow(clippy::too_many_arguments)]
    fn directiva(
        &self,
        origen: &Origen,
        nombre: &Texto,
        argumentos: &[Texto],
        dir: &mut u64,
        secciones: &mut Secciones,
        simbolos: &mut BTreeMap<String, Simbolo>,
        elementos: &mut Vec<Elemento>,
        errores: &mut Vec<ErrorEns>,
    ) {
        let m = self.maquina;
        let args_mal = |esperado: &str| {
            Self::error(
                origen,
                nombre.columna,
                Causa::ArgumentosDeDirectiva {
                    directiva: format!(".{}", nombre.texto),
                    esperado: esperado.to_string(),
                },
            )
        };
        match nombre.texto.to_ascii_lowercase().as_str() {
            "org" => {
                if argumentos.len() != 1 {
                    errores.push(args_mal("una direccion"));
                    return;
                }
                if let Some(v) =
                    Self::evaluar_ahora(simbolos, origen, &argumentos[0], *dir, errores)
                {
                    if v < 0 || v > m.direccion_maxima() as i128 {
                        errores.push(Self::error(
                            origen,
                            argumentos[0].columna,
                            Causa::OrigenInvalido { valor: v },
                        ));
                    } else {
                        *dir = v as u64;
                    }
                }
            }
            "seccion" | "section" => {
                let [arg] = argumentos else {
                    errores.push(args_mal("el nombre de una region de memoria"));
                    return;
                };
                let Some(region) = m.regiones.iter().find(|r| r.nombre == arg.texto) else {
                    errores.push(Self::error(
                        origen,
                        arg.columna,
                        Causa::Otra(format!(
                            "no hay ninguna region llamada `{}`; las secciones se llaman como las regiones de la maquina ({})",
                            arg.texto,
                            m.regiones
                                .iter()
                                .map(|r| r.nombre.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )),
                    ));
                    return;
                };
                if let Some(anterior) = secciones.actual.take() {
                    secciones.guardadas.insert(anterior, *dir);
                }
                *dir = secciones
                    .guardadas
                    .get(&region.nombre)
                    .copied()
                    .unwrap_or(region.desde);
                secciones.actual = Some(region.nombre.clone());
            }
            "equ" | "const" => {
                if argumentos.len() != 2 {
                    errores.push(args_mal("un nombre y un valor"));
                    return;
                }
                if let Some(v) =
                    Self::evaluar_ahora(simbolos, origen, &argumentos[1], *dir, errores)
                {
                    Self::definir(simbolos, origen, &argumentos[0], v, false, errores);
                }
            }
            "db" | "palabra" | "word" => {
                if argumentos.is_empty() {
                    errores.push(args_mal("al menos un dato"));
                    return;
                }
                let mut datos = Vec::new();
                let mut total = 0usize;
                for a in argumentos {
                    if let Some(cadena) = leer_cadena(&a.texto) {
                        match cadena {
                            Ok(chars) => {
                                total += chars.len();
                                datos.push(DatoFuente::Cadena(chars));
                            }
                            Err(msg) => {
                                errores.push(Self::error(origen, a.columna, Causa::Sintaxis(msg)))
                            }
                        }
                    } else {
                        match Expr::analizar(&a.texto) {
                            Ok(e) => {
                                total += 1;
                                datos.push(DatoFuente::Valor(a.clone(), e));
                            }
                            Err(e) => {
                                errores.push(Self::error(origen, a.columna, Causa::Expresion(e)))
                            }
                        }
                    }
                }
                elementos.push(Elemento {
                    origen: Origen {
                        archivo: origen.archivo.clone(),
                        linea: origen.linea,
                    },
                    direccion: *dir,
                    contenido: Contenido::Datos(datos),
                });
                *dir = dir.wrapping_add(total as u64);
            }
            "dw" | "direccion" | "addr" => {
                if argumentos.is_empty() {
                    errores.push(args_mal("al menos una direccion"));
                    return;
                }
                let mut items = Vec::new();
                for a in argumentos {
                    match Expr::analizar(&a.texto) {
                        Ok(e) => items.push((a.clone(), e)),
                        Err(e) => errores.push(Self::error(origen, a.columna, Causa::Expresion(e))),
                    }
                }
                let n = items.len() * palabras_por_direccion(m);
                elementos.push(Elemento {
                    origen: Origen {
                        archivo: origen.archivo.clone(),
                        linea: origen.linea,
                    },
                    direccion: *dir,
                    contenido: Contenido::Direcciones(items),
                });
                *dir = dir.wrapping_add(n as u64);
            }
            "espacio" | "space" | "res" => {
                if argumentos.is_empty() || argumentos.len() > 2 {
                    errores.push(args_mal(
                        "una cantidad y, opcionalmente, un valor de relleno",
                    ));
                    return;
                }
                let Some(n) = Self::evaluar_ahora(simbolos, origen, &argumentos[0], *dir, errores)
                else {
                    return;
                };
                if n < 0 || n > (m.direccion_maxima() as i128 + 1) {
                    errores.push(Self::error(
                        origen,
                        argumentos[0].columna,
                        Causa::Otra(format!("no se pueden reservar {n} palabras")),
                    ));
                    return;
                }
                let relleno = match argumentos.get(1) {
                    Some(t) => match Expr::analizar(&t.texto) {
                        Ok(e) => Some((t.clone(), e)),
                        Err(e) => {
                            errores.push(Self::error(origen, t.columna, Causa::Expresion(e)));
                            return;
                        }
                    },
                    None => None,
                };
                elementos.push(Elemento {
                    origen: Origen {
                        archivo: origen.archivo.clone(),
                        linea: origen.linea,
                    },
                    direccion: *dir,
                    contenido: Contenido::Espacio {
                        cantidad: n as usize,
                        relleno,
                    },
                });
                *dir = dir.wrapping_add(n as u64);
            }
            otra => errores.push(Self::error(
                origen,
                nombre.columna,
                Causa::DirectivaDesconocida(format!(".{otra}")),
            )),
        }
    }

    /// Busca la forma que encaja con los operandos escritos; devuelve el contenido y su longitud.
    fn instruccion(
        &self,
        origen: &Origen,
        mnemonico: &Texto,
        operandos: &Texto,
        errores: &mut Vec<ErrorEns>,
    ) -> Option<(Contenido, usize)> {
        let candidatas: Vec<(usize, &Forma)> = self
            .formas
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                f.plantilla
                    .mnemonico()
                    .eq_ignore_ascii_case(&mnemonico.texto)
            })
            .collect();
        if candidatas.is_empty() {
            errores.push(Self::error(
                origen,
                mnemonico.columna,
                Causa::MnemonicoDesconocido(mnemonico.texto.clone()),
            ));
            return None;
        }
        let mut registro_malo: Option<ErrorEns> = None;
        'formas: for (pos, forma) in &candidatas {
            let Some(capturas) = forma.plantilla.reconocer_con_posicion(&operandos.texto) else {
                continue;
            };
            let ins = &self.maquina.instrucciones[forma.indice];
            let mut resueltos = Vec::new();
            for (hueco, texto, desplazamiento) in capturas {
                let operando = ins.operandos.iter().find(|o| o.nombre == hueco)?;
                let columna = operandos.columna + operandos.texto[..desplazamiento].chars().count();
                let t = Texto { texto, columna };
                match &operando.tipo {
                    TipoOperando::Registro { permitidos } => {
                        if !permitidos.iter().any(|r| r.eq_ignore_ascii_case(&t.texto)) {
                            registro_malo.get_or_insert(Self::error(
                                origen,
                                t.columna,
                                Causa::RegistroInvalido {
                                    operando: hueco.clone(),
                                    escrito: t.texto.clone(),
                                    permitidos: permitidos.clone(),
                                },
                            ));
                            continue 'formas;
                        }
                        resueltos.push((hueco, t, None));
                    }
                    _ => match Expr::analizar(&t.texto) {
                        Ok(e) => resueltos.push((hueco, t, Some(e))),
                        Err(e) => {
                            errores.push(Self::error(origen, t.columna, Causa::Expresion(e)));
                            return None;
                        }
                    },
                }
            }
            let longitud = ins.codificacion.longitud_en_palabras();
            return Some((
                Contenido::Instruccion {
                    forma: *pos,
                    operandos: resueltos,
                    fuente: format!("{} {}", mnemonico.texto, operandos.texto.trim())
                        .trim()
                        .to_string(),
                },
                longitud,
            ));
        }
        if let Some(e) = registro_malo {
            errores.push(e);
        } else {
            errores.push(Self::error(
                origen,
                operandos.columna,
                Causa::OperandosNoEncajan {
                    mnemonico: mnemonico.texto.clone(),
                    formas: candidatas
                        .iter()
                        .map(|(_, f)| self.maquina.instrucciones[f.indice].sintaxis.clone())
                        .collect(),
                },
            ));
        }
        None
    }

    fn segunda_pasada(
        &self,
        elementos: Vec<Elemento>,
        simbolos: BTreeMap<String, Simbolo>,
        externos: &BTreeMap<String, Externo>,
        errores: &mut Vec<ErrorEns>,
    ) -> Objeto {
        let m = self.maquina;
        let mut segmentos: Vec<Segmento> = Vec::new();
        let mut mapa = Vec::new();
        let mut escritas: BTreeMap<u64, usize> = BTreeMap::new();
        let mut reubicaciones = Vec::new();
        for el in &elementos {
            let tabla = Tabla {
                simbolos: &simbolos,
                aqui: el.direccion as i128,
            };
            let usados = Self::externos_usados(el, externos);
            let palabras = if usados.is_empty() {
                self.emitir(el, &tabla, errores)
            } else if let Contenido::Instruccion { forma, fuente, .. } = &el.contenido {
                let n = m.instrucciones[self.formas[*forma].indice]
                    .codificacion
                    .longitud_en_palabras();
                reubicaciones.push(Reubicacion {
                    direccion: el.direccion,
                    longitud: n,
                    archivo: el.origen.archivo.clone(),
                    linea: el.origen.linea,
                    fuente: fuente.clone(),
                    simbolos: usados,
                });
                Some(vec![0; n])
            } else {
                errores.push(Self::error(
                    &el.origen,
                    1,
                    Causa::Otra(
                        "un simbolo externo solo se puede usar en los operandos de una instruccion"
                            .into(),
                    ),
                ));
                None
            };
            let Some(palabras) = palabras else {
                continue;
            };
            if palabras.is_empty() {
                continue;
            }
            if !self.comprobar_region(el, palabras.len(), errores) {
                continue;
            }
            let mut solapa = None;
            for i in 0..palabras.len() as u64 {
                let a = el.direccion.wrapping_add(i) & m.direccion_maxima();
                if escritas.insert(a, el.origen.linea).is_some() {
                    solapa.get_or_insert(a);
                }
            }
            if let Some(a) = solapa {
                errores.push(Self::error(&el.origen, 1, Causa::Solape { direccion: a }));
                continue;
            }
            mapa.push(EntradaMapa {
                direccion: el.direccion,
                longitud: palabras.len(),
                archivo: el.origen.archivo.clone(),
                linea: el.origen.linea,
            });
            match segmentos.last_mut() {
                Some(s) if s.direccion + s.palabras.len() as u64 == el.direccion => {
                    s.palabras.extend(palabras)
                }
                _ => segmentos.push(Segmento {
                    direccion: el.direccion,
                    palabras,
                }),
            }
        }
        Objeto {
            segmentos,
            simbolos,
            mapa,
            externos: BTreeMap::new(),
            reubicaciones,
            avisos: Vec::new(),
        }
    }

    /// Avisa de los saltos condicionales que leen una bandera que ninguna instruccion fijo
    /// desde el principio del programa, o que una llamada pudo alterar (RF-ENS-028).
    ///
    /// El analisis recorre el programa en el orden escrito. Una etiqueta puede alcanzarse desde
    /// cualquier sitio, asi que ahi todo se da por bueno y el aviso se calla; solo se avisa
    /// cuando la bandera es indeterminada de verdad dentro del tramo que se ve.
    fn avisar_banderas(
        &self,
        elementos: &[Elemento],
        simbolos: &BTreeMap<String, Simbolo>,
    ) -> Vec<Aviso> {
        use lorenz_arch::semantica::ast::{Destino, Expr, Sentencia};
        #[derive(Clone, Copy, PartialEq)]
        enum Estado {
            SinFijar,
            Fijada,
            Alterada,
            Heredada,
        }
        fn nombres(e: &Expr, fuera: &mut Vec<String>) {
            match e {
                Expr::Nombre(n) => fuera.push(n.clone()),
                Expr::Mem(a) | Expr::Entrada(a) | Expr::Unario(_, a) => nombres(a, fuera),
                Expr::Binario(_, a, b) => {
                    nombres(a, fuera);
                    nombres(b, fuera);
                }
                Expr::Literal(_) => {}
            }
        }
        /// Destinos asignados, y si algun salto condicional asigna `pc`: `(cond, asigna pc)`.
        fn recorrer(
            s: &[Sentencia],
            destinos: &mut Vec<String>,
            condiciones: &mut Vec<Vec<String>>,
        ) {
            for x in s {
                match x {
                    Sentencia::Asignar(Destino::Nombre(n), _) => destinos.push(n.clone()),
                    Sentencia::Si(c, cuerpo) => {
                        let (mut d, mut k) = (Vec::new(), Vec::new());
                        recorrer(cuerpo, &mut d, &mut k);
                        if d.iter().any(|n| n == "pc") {
                            let mut v = Vec::new();
                            nombres(c, &mut v);
                            condiciones.push(v);
                        }
                        destinos.extend(d);
                        condiciones.extend(k);
                    }
                    _ => {}
                }
            }
        }
        let m = self.maquina;
        if m.banderas.is_empty() {
            return Vec::new();
        }
        let mut estado: BTreeMap<&str, Estado> = m
            .banderas
            .iter()
            .map(|b| (b.nombre.as_str(), Estado::SinFijar))
            .collect();
        let etiquetas: std::collections::BTreeSet<u64> = simbolos
            .values()
            .filter(|s| s.etiqueta)
            .map(|s| s.valor as u64)
            .collect();
        let mut avisos = Vec::new();
        for el in elementos {
            if etiquetas.contains(&el.direccion) {
                estado.values_mut().for_each(|e| *e = Estado::Heredada);
            }
            let Contenido::Instruccion { forma, .. } = &el.contenido else {
                continue;
            };
            let ins = &m.instrucciones[self.formas[*forma].indice];
            let (mut destinos, mut condiciones) = (Vec::new(), Vec::new());
            recorrer(&ins.programa.sentencias, &mut destinos, &mut condiciones);
            for condicion in &condiciones {
                for n in condicion {
                    let Some(e) = estado.get(n.as_str()).copied() else {
                        continue;
                    };
                    let porque = match e {
                        Estado::SinFijar => "ninguna instruccion la ha fijado todavia",
                        Estado::Alterada => "una llamada anterior pudo cambiarla",
                        _ => continue,
                    };
                    avisos.push(Aviso {
                        archivo: el.origen.archivo.clone(),
                        linea: el.origen.linea,
                        mensaje: format!(
                            "`{}` salta segun la bandera {n}, que es indeterminada: {porque}",
                            ins.mnemonico
                        ),
                    });
                }
            }
            let escribe = |n: &str| destinos.iter().any(|d| d == n);
            let es_llamada = escribe("pc") && escribe("sp");
            for b in &m.banderas {
                let fija = ins.afecta.contains(&b.nombre) || escribe(&b.nombre);
                if let Some(e) = estado.get_mut(b.nombre.as_str()) {
                    if fija {
                        *e = Estado::Fijada;
                    } else if es_llamada {
                        *e = Estado::Alterada;
                    }
                }
            }
        }
        avisos
    }

    /// Los simbolos externos que un elemento necesita, sin repetir.
    fn externos_usados(el: &Elemento, externos: &BTreeMap<String, Externo>) -> Vec<String> {
        let mut usados: Vec<String> = Vec::new();
        let mut mirar = |e: &Expr| {
            for s in e.simbolos() {
                if externos.contains_key(s) && !usados.iter().any(|u| u == s) {
                    usados.push(s.to_string());
                }
            }
        };
        match &el.contenido {
            Contenido::Instruccion { operandos, .. } => {
                for (_, _, e) in operandos {
                    if let Some(e) = e {
                        mirar(e);
                    }
                }
            }
            Contenido::Datos(ds) => {
                for d in ds {
                    if let DatoFuente::Valor(_, e) = d {
                        mirar(e);
                    }
                }
            }
            Contenido::Direcciones(items) => {
                for (_, e) in items {
                    mirar(e);
                }
            }
            Contenido::Espacio { relleno, .. } => {
                if let Some((_, e)) = relleno {
                    mirar(e);
                }
            }
        }
        usados
    }

    /// El codigo solo puede caer en regiones que existan, que se puedan leer y que no sean un
    /// dispositivo (RF-ENS-013). Una region de solo lectura es valida: es donde vive una ROM.
    fn comprobar_region(&self, el: &Elemento, n: usize, errores: &mut Vec<ErrorEns>) -> bool {
        let m = self.maquina;
        let ultima = el.direccion + n as u64 - 1;
        for a in [el.direccion, ultima] {
            match m.region_de(a) {
                None => {
                    errores.push(Self::error(
                        &el.origen,
                        1,
                        Causa::SinRegion { direccion: a },
                    ));
                    return false;
                }
                Some(r) if r.dispositivo.is_some() || !r.acceso.lectura => {
                    errores.push(Self::error(
                        &el.origen,
                        1,
                        Causa::RegionNoEscribible {
                            direccion: a,
                            region: r.nombre.clone(),
                        },
                    ));
                    return false;
                }
                Some(_) => {}
            }
        }
        match (m.region_de(el.direccion), m.region_de(ultima)) {
            (Some(a), Some(b)) if a.nombre == b.nombre => true,
            _ => {
                errores.push(Self::error(
                    &el.origen,
                    1,
                    Causa::Otra("el elemento cruza el limite entre dos regiones de memoria".into()),
                ));
                false
            }
        }
    }

    fn emitir(
        &self,
        el: &Elemento,
        tabla: &Tabla<'_>,
        errores: &mut Vec<ErrorEns>,
    ) -> Option<Vec<u64>> {
        let m = self.maquina;
        match &el.contenido {
            Contenido::Instruccion {
                forma, operandos, ..
            } => {
                let ins: &Instruccion = &m.instrucciones[self.formas[*forma].indice];
                let mut valores: BTreeMap<String, i128> = BTreeMap::new();
                let mut hubo_error = false;
                for (hueco, texto, expr) in operandos {
                    match expr {
                        None => {
                            let TipoOperando::Registro { permitidos } = &ins
                                .operandos
                                .iter()
                                .find(|o| &o.nombre == hueco)
                                .expect("operando declarado")
                                .tipo
                            else {
                                unreachable!("solo los registros no llevan expresion");
                            };
                            let idx = permitidos
                                .iter()
                                .position(|r| r.eq_ignore_ascii_case(&texto.texto))
                                .expect("ya comprobado en la primera pasada");
                            valores.insert(hueco.clone(), idx as i128);
                        }
                        Some(e) => match e.evaluar(tabla) {
                            Ok(v) => {
                                valores.insert(hueco.clone(), v);
                            }
                            Err(err) => {
                                errores.push(Self::error(
                                    &el.origen,
                                    texto.columna,
                                    Causa::Expresion(err),
                                ));
                                hubo_error = true;
                            }
                        },
                    }
                }
                if hubo_error {
                    return None;
                }
                match codificar(m, ins, &valores, el.direccion) {
                    Ok(p) => Some(p),
                    Err((operando, causa)) => {
                        let columna = operandos
                            .iter()
                            .find(|(h, _, _)| *h == operando)
                            .map_or(1, |(_, t, _)| t.columna);
                        errores.push(Self::error(&el.origen, columna, causa));
                        None
                    }
                }
            }
            Contenido::Datos(datos) => {
                let mut salida = Vec::new();
                let mut ok = true;
                for d in datos {
                    match d {
                        DatoFuente::Cadena(p) => salida.extend(p.iter().copied()),
                        DatoFuente::Valor(t, e) => match e.evaluar(tabla) {
                            Ok(v) => match self.palabra(v) {
                                Some(p) => salida.push(p),
                                None => {
                                    errores.push(Self::error(
                                        &el.origen,
                                        t.columna,
                                        Causa::DatoFueraDeRango {
                                            valor: v,
                                            bits: m.bits_palabra,
                                        },
                                    ));
                                    ok = false;
                                }
                            },
                            Err(err) => {
                                errores.push(Self::error(
                                    &el.origen,
                                    t.columna,
                                    Causa::Expresion(err),
                                ));
                                ok = false;
                            }
                        },
                    }
                }
                ok.then_some(salida)
            }
            Contenido::Direcciones(items) => {
                let mut salida = Vec::new();
                let mut ok = true;
                for (t, e) in items {
                    match e.evaluar(tabla) {
                        Ok(v) if v >= 0 && v <= m.direccion_maxima() as i128 => {
                            let mut trozos: Vec<u64> = (0..palabras_por_direccion(m))
                                .map(|i| {
                                    ((v as u64) >> (i as u32 * m.bits_palabra)) & m.palabra_maxima()
                                })
                                .collect();
                            if m.endianness == Endianness::Big {
                                trozos.reverse();
                            }
                            salida.extend(trozos);
                        }
                        Ok(v) => {
                            errores.push(Self::error(
                                &el.origen,
                                t.columna,
                                Causa::DireccionFueraDeRango {
                                    valor: v,
                                    maximo: m.direccion_maxima(),
                                },
                            ));
                            ok = false;
                        }
                        Err(err) => {
                            errores.push(Self::error(&el.origen, t.columna, Causa::Expresion(err)));
                            ok = false;
                        }
                    }
                }
                ok.then_some(salida)
            }
            Contenido::Espacio { cantidad, relleno } => {
                let valor = match relleno {
                    None => 0,
                    Some((t, e)) => match e.evaluar(tabla) {
                        Ok(v) => match self.palabra(v) {
                            Some(p) => p,
                            None => {
                                errores.push(Self::error(
                                    &el.origen,
                                    t.columna,
                                    Causa::DatoFueraDeRango {
                                        valor: v,
                                        bits: m.bits_palabra,
                                    },
                                ));
                                return None;
                            }
                        },
                        Err(err) => {
                            errores.push(Self::error(&el.origen, t.columna, Causa::Expresion(err)));
                            return None;
                        }
                    },
                };
                Some(vec![valor; *cantidad])
            }
        }
    }

    /// Un dato cabe en una palabra si es un natural de `bits_palabra` bits o un entero negativo que
    /// cabe en complemento a dos.
    fn palabra(&self, v: i128) -> Option<u64> {
        let b = self.maquina.bits_palabra;
        let maximo = self.maquina.palabra_maxima() as i128;
        let minimo = -(1i128 << (b - 1));
        if v >= minimo && v <= maximo {
            Some((v as u64) & self.maquina.palabra_maxima())
        } else {
            None
        }
    }
}

/// Si el argumento es una cadena entre comillas dobles, devuelve sus palabras.
fn leer_cadena(texto: &str) -> Option<Result<Vec<u64>, String>> {
    let t = texto.trim();
    if !t.starts_with('"') {
        return None;
    }
    if t.len() < 2 || !t.ends_with('"') {
        return Some(Err("falta cerrar las comillas de la cadena".to_string()));
    }
    let cuerpo = &t[1..t.len() - 1];
    let mut salida = Vec::new();
    let mut chars = cuerpo.chars();
    while let Some(c) = chars.next() {
        let v = if c == '\\' {
            match chars.next() {
                Some('n') => 10,
                Some('t') => 9,
                Some('r') => 13,
                Some('0') => 0,
                Some('\\') => 92,
                Some('"') => 34,
                Some(o) => return Some(Err(format!("escape desconocido `\\{o}` en la cadena"))),
                None => return Some(Err("la cadena termina en una barra".to_string())),
            }
        } else {
            c as u64
        };
        salida.push(v);
    }
    Some(Ok(salida))
}
