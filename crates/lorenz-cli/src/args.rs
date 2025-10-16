//! Interpretacion de los argumentos de la linea de comandos.

use thiserror::Error;

/// Limite de instrucciones por omision de `lorenz ejecutar`.
pub const LIMITE_POR_OMISION: u64 = 1_000_000;

/// Un rango de memoria que se vuelca al terminar (RF-EMU-024).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volcado {
    pub desde: u64,
    pub cantidad: u64,
    /// Archivo donde escribirlo; si falta, se imprime.
    pub archivo: Option<String>,
}

/// Una interrupcion que se dispara a mano durante la ejecucion (RF-EMU-040).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disparo {
    pub linea: u32,
    /// Se levanta justo antes de ejecutar la instruccion con este numero de orden (0 es la primera).
    pub instruccion: u64,
}

/// Lo que el usuario pidio hacer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Orden {
    Ayuda,
    Version,
    /// Cargar y validar una descripcion de arquitectura.
    Validar {
        maquina: String,
    },
    /// Ejecutar el mismo programa en dos descripciones de maquina y comparar paso a paso.
    Comparar {
        maquina_a: String,
        maquina_b: String,
        programa: String,
        limite: u64,
        direccion: Option<u64>,
    },
    /// Ensamblar un fuente y escribir el programa en hexadecimal.
    Ensamblar {
        maquina: String,
        fuente: String,
        opciones: crate::ensamblar::OpcionesEnsamblar,
    },
    /// Desensamblar un programa en hexadecimal.
    Desensamblar {
        maquina: String,
        programa: String,
        direccion: Option<u64>,
        simbolos: Option<String>,
    },
    /// Ejecutar las pruebas de un proyecto.
    Probar {
        pruebas: String,
        nombre: Option<String>,
        etiqueta: Option<String>,
        hilos: usize,
        /// Escribir el informe en JSON por la salida estandar en lugar del texto.
        json: bool,
        /// Guardar ademas el informe JSON en este archivo.
        salida: Option<String>,
        /// Abrir el depurador en el estado en que fallo esta prueba.
        depurar: Option<String>,
        /// Medir la cobertura de instrucciones y ramas.
        cobertura: bool,
    },
    /// Comprobar el emulador contra la semantica de la arquitectura.
    Conformidad {
        maquina: String,
        instruccion: Option<String>,
        /// Combinaciones de operandos por instruccion como maximo.
        casos: Option<usize>,
        repeticiones: Option<usize>,
        json: bool,
        salida: Option<String>,
    },
    /// Depurar un programa desde la linea de comandos.
    Depurar {
        maquina: String,
        programa: String,
        /// Archivo con las ordenes a ejecutar en lugar de leerlas de la entrada.
        guion: Option<String>,
    },
    /// Cargar un programa y ejecutarlo.
    Ejecutar {
        maquina: String,
        programa: String,
        limite: u64,
        traza: bool,
        /// La traza sale en formato de maquina (`clave=valor`), pensado para compararla con otra.
        traza_maquina: bool,
        /// Dibujar las pantallas de la maquina al terminar.
        pantalla: bool,
        /// Avisar de las lecturas de memoria que nadie habia escrito.
        vigilar: bool,
        /// Teclas que se pulsan en el primer teclado antes de empezar, una por caracter.
        teclas: Option<String>,
        /// Texto que se entrega a la primera UART antes de empezar.
        serie: Option<String>,
        /// Rangos de memoria que se vuelcan al terminar.
        volcados: Vec<Volcado>,
        /// Interrupciones que se disparan a mano.
        disparos: Vec<Disparo>,
        /// Guion de eventos de entrada que se reproducen durante la ejecucion.
        guion: Option<String>,
        /// Direccion donde se carga el programa; por omision, la de arranque de la maquina.
        direccion: Option<u64>,
    },
}

/// Argumentos mal escritos.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorUso {
    #[error("falta la orden; prueba 'lorenz ayuda'")]
    SinOrden,
    #[error("la orden '{0}' no existe; prueba 'lorenz ayuda'")]
    OrdenDesconocida(String),
    #[error("falta el argumento <{0}>")]
    Falta(&'static str),
    #[error("sobra el argumento '{0}'")]
    Sobra(String),
    #[error("la opcion '{0}' no existe")]
    OpcionDesconocida(String),
    #[error("la opcion '{0}' necesita un valor")]
    OpcionSinValor(String),
    #[error("'{0}' no es un volcado valido; se espera DESDE:CANTIDAD o DESDE:CANTIDAD:ARCHIVO")]
    VolcadoInvalido(String),
    #[error("'{0}' no es un disparo valido; se espera LINEA@INSTRUCCION")]
    DisparoInvalido(String),
    #[error("'{valor}' no es un numero valido para {opcion}")]
    NumeroInvalido { opcion: String, valor: String },
}

fn numero(opcion: &str, valor: &str) -> Result<u64, ErrorUso> {
    let invalido = || ErrorUso::NumeroInvalido {
        opcion: opcion.to_string(),
        valor: valor.to_string(),
    };
    let sin_guiones = valor.replace('_', "");
    if let Some(h) = sin_guiones
        .strip_prefix("0x")
        .or_else(|| sin_guiones.strip_prefix("0X"))
    {
        u64::from_str_radix(h, 16).map_err(|_| invalido())
    } else if let Some(b) = sin_guiones
        .strip_prefix("0b")
        .or_else(|| sin_guiones.strip_prefix("0B"))
    {
        u64::from_str_radix(b, 2).map_err(|_| invalido())
    } else {
        sin_guiones.parse().map_err(|_| invalido())
    }
}

/// Interpreta los argumentos (sin el nombre del programa).
pub fn interpretar(args: &[String]) -> Result<Orden, ErrorUso> {
    let (orden, resto) = args.split_first().ok_or(ErrorUso::SinOrden)?;
    match orden.as_str() {
        "ayuda" | "--ayuda" | "-h" | "--help" => Ok(Orden::Ayuda),
        "version" | "--version" | "-V" => Ok(Orden::Version),
        "validar" => {
            let mut posicionales = Vec::new();
            for a in resto {
                if a.starts_with("--") {
                    return Err(ErrorUso::OpcionDesconocida(a.clone()));
                }
                posicionales.push(a.clone());
            }
            let mut it = posicionales.into_iter();
            let maquina = it.next().ok_or(ErrorUso::Falta("maquina"))?;
            if let Some(extra) = it.next() {
                return Err(ErrorUso::Sobra(extra));
            }
            Ok(Orden::Validar { maquina })
        }
        "ejecutar" => interpretar_ejecutar(resto),
        "comparar" => interpretar_comparar(resto),
        "ensamblar" => interpretar_ensamblar(resto),
        "desensamblar" => interpretar_desensamblar(resto),
        "depurar" => interpretar_depurar(resto),
        "probar" => interpretar_probar(resto),
        "conformidad" => interpretar_conformidad(resto),
        otra => Err(ErrorUso::OrdenDesconocida(otra.to_string())),
    }
}

fn interpretar_ejecutar(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let mut limite = LIMITE_POR_OMISION;
    let mut traza = false;
    let mut traza_maquina = false;
    let mut pantalla = false;
    let mut vigilar = false;
    let mut teclas = None;
    let mut serie = None;
    let mut volcados = Vec::new();
    let mut disparos = Vec::new();
    let mut guion = None;
    let mut direccion = None;
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--traza" => traza = true,
            "--traza-maquina" => {
                traza = true;
                traza_maquina = true;
            }
            "--pantalla" => pantalla = true,
            "--vigilar" => vigilar = true,
            "--guion" => {
                i += 1;
                let valor = resto
                    .get(i)
                    .ok_or_else(|| ErrorUso::OpcionSinValor(a.clone()))?;
                guion = Some(valor.clone());
            }
            "--interrumpir" => {
                i += 1;
                let valor = resto
                    .get(i)
                    .ok_or_else(|| ErrorUso::OpcionSinValor(a.clone()))?;
                disparos.push(interpretar_disparo(valor)?);
            }
            "--volcar" => {
                i += 1;
                let valor = resto
                    .get(i)
                    .ok_or_else(|| ErrorUso::OpcionSinValor(a.clone()))?;
                volcados.push(interpretar_volcado(valor)?);
            }
            "--serie" => {
                i += 1;
                let valor = resto
                    .get(i)
                    .ok_or_else(|| ErrorUso::OpcionSinValor(a.clone()))?;
                serie = Some(valor.clone());
            }
            "--teclas" => {
                i += 1;
                let valor = resto
                    .get(i)
                    .ok_or_else(|| ErrorUso::OpcionSinValor(a.clone()))?;
                teclas = Some(valor.clone());
            }
            "--limite" | "--direccion" => {
                i += 1;
                let valor = resto
                    .get(i)
                    .ok_or_else(|| ErrorUso::OpcionSinValor(a.clone()))?;
                let n = numero(a, valor)?;
                if a == "--limite" {
                    limite = n;
                } else {
                    direccion = Some(n);
                }
            }
            _ if a.starts_with("--") => return Err(ErrorUso::OpcionDesconocida(a.clone())),
            _ => posicionales.push(a.clone()),
        }
        i += 1;
    }
    let mut it = posicionales.into_iter();
    let maquina = it.next().ok_or(ErrorUso::Falta("maquina"))?;
    let programa = it.next().ok_or(ErrorUso::Falta("programa"))?;
    if let Some(extra) = it.next() {
        return Err(ErrorUso::Sobra(extra));
    }
    Ok(Orden::Ejecutar {
        maquina,
        programa,
        limite,
        traza,
        traza_maquina,
        pantalla,
        vigilar,
        teclas,
        serie,
        volcados,
        disparos,
        guion,
        direccion,
    })
}

fn interpretar_comparar(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let mut limite = LIMITE_POR_OMISION;
    let mut direccion = None;
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--limite" | "--direccion" => {
                i += 1;
                let valor = resto
                    .get(i)
                    .ok_or_else(|| ErrorUso::OpcionSinValor(a.clone()))?;
                let n = numero(a, valor)?;
                if a == "--limite" {
                    limite = n;
                } else {
                    direccion = Some(n);
                }
            }
            _ if a.starts_with("--") => return Err(ErrorUso::OpcionDesconocida(a.clone())),
            _ => posicionales.push(a.clone()),
        }
        i += 1;
    }
    let mut it = posicionales.into_iter();
    let maquina_a = it.next().ok_or(ErrorUso::Falta("maquina-a"))?;
    let maquina_b = it.next().ok_or(ErrorUso::Falta("maquina-b"))?;
    let programa = it.next().ok_or(ErrorUso::Falta("programa"))?;
    if let Some(extra) = it.next() {
        return Err(ErrorUso::Sobra(extra));
    }
    Ok(Orden::Comparar {
        maquina_a,
        maquina_b,
        programa,
        limite,
        direccion,
    })
}

/// Valor de una opcion que lo necesita: avanza `i` hasta el argumento siguiente.
fn valor_de<'a>(resto: &'a [String], i: &mut usize, opcion: &str) -> Result<&'a String, ErrorUso> {
    *i += 1;
    resto
        .get(*i)
        .ok_or_else(|| ErrorUso::OpcionSinValor(opcion.to_string()))
}

fn interpretar_ensamblar(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let mut opciones = crate::ensamblar::OpcionesEnsamblar::default();
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "-o" | "--salida" => opciones.salida = Some(valor_de(resto, &mut i, a)?.clone()),
            "--mapa" => opciones.mapa = Some(valor_de(resto, &mut i, a)?.clone()),
            "--listado" => opciones.listado = Some(valor_de(resto, &mut i, a)?.clone()),
            "--simbolos" => opciones.simbolos = Some(valor_de(resto, &mut i, a)?.clone()),
            "--objeto" => opciones.objeto = Some(valor_de(resto, &mut i, a)?.clone()),
            _ if a.starts_with('-') => return Err(ErrorUso::OpcionDesconocida(a.clone())),
            _ => posicionales.push(a.clone()),
        }
        i += 1;
    }
    let mut it = posicionales.into_iter();
    let maquina = it.next().ok_or(ErrorUso::Falta("maquina"))?;
    let fuente = it.next().ok_or(ErrorUso::Falta("fuente"))?;
    if let Some(extra) = it.next() {
        return Err(ErrorUso::Sobra(extra));
    }
    Ok(Orden::Ensamblar {
        maquina,
        fuente,
        opciones,
    })
}

fn interpretar_probar(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let (mut nombre, mut etiqueta, mut salida, mut depurar) = (None, None, None, None);
    let (mut hilos, mut json, mut cobertura) = (0usize, false, false);
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--cobertura" => cobertura = true,
            "--nombre" => nombre = Some(valor_de(resto, &mut i, a)?.clone()),
            "--etiqueta" => etiqueta = Some(valor_de(resto, &mut i, a)?.clone()),
            "--salida" => salida = Some(valor_de(resto, &mut i, a)?.clone()),
            "--depurar" => depurar = Some(valor_de(resto, &mut i, a)?.clone()),
            "--json" => json = true,
            "--hilos" => {
                let v = valor_de(resto, &mut i, a)?;
                hilos = numero("--hilos", v)? as usize;
            }
            _ if a.starts_with('-') => return Err(ErrorUso::OpcionDesconocida(a.clone())),
            _ => posicionales.push(a.clone()),
        }
        i += 1;
    }
    let mut it = posicionales.into_iter();
    let pruebas = it.next().ok_or(ErrorUso::Falta("archivo de pruebas"))?;
    if let Some(extra) = it.next() {
        return Err(ErrorUso::Sobra(extra));
    }
    Ok(Orden::Probar {
        pruebas,
        nombre,
        etiqueta,
        hilos,
        json,
        salida,
        depurar,
        cobertura,
    })
}

fn interpretar_conformidad(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let (mut instruccion, mut salida) = (None, None);
    let (mut casos, mut repeticiones, mut json) = (None, None, false);
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--instruccion" => instruccion = Some(valor_de(resto, &mut i, a)?.clone()),
            "--salida" => salida = Some(valor_de(resto, &mut i, a)?.clone()),
            "--json" => json = true,
            "--casos" => {
                let v = valor_de(resto, &mut i, a)?;
                casos = Some(numero("--casos", v)? as usize);
            }
            "--repeticiones" => {
                let v = valor_de(resto, &mut i, a)?;
                repeticiones = Some(numero("--repeticiones", v)? as usize);
            }
            _ if a.starts_with('-') => return Err(ErrorUso::OpcionDesconocida(a.clone())),
            _ => posicionales.push(a.clone()),
        }
        i += 1;
    }
    let mut it = posicionales.into_iter();
    let maquina = it.next().ok_or(ErrorUso::Falta("maquina"))?;
    if let Some(extra) = it.next() {
        return Err(ErrorUso::Sobra(extra));
    }
    Ok(Orden::Conformidad {
        maquina,
        instruccion,
        casos,
        repeticiones,
        json,
        salida,
    })
}

fn interpretar_depurar(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let mut guion = None;
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--guion" => guion = Some(valor_de(resto, &mut i, a)?.clone()),
            _ if a.starts_with('-') => return Err(ErrorUso::OpcionDesconocida(a.clone())),
            _ => posicionales.push(a.clone()),
        }
        i += 1;
    }
    let mut it = posicionales.into_iter();
    let maquina = it.next().ok_or(ErrorUso::Falta("maquina"))?;
    let programa = it.next().ok_or(ErrorUso::Falta("programa"))?;
    if let Some(extra) = it.next() {
        return Err(ErrorUso::Sobra(extra));
    }
    Ok(Orden::Depurar {
        maquina,
        programa,
        guion,
    })
}

fn interpretar_desensamblar(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let mut direccion = None;
    let mut simbolos = None;
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--direccion" => {
                let v = valor_de(resto, &mut i, a)?;
                direccion = Some(numero(a, v)?);
            }
            "--simbolos" => simbolos = Some(valor_de(resto, &mut i, a)?.clone()),
            _ if a.starts_with("--") => return Err(ErrorUso::OpcionDesconocida(a.clone())),
            _ => posicionales.push(a.clone()),
        }
        i += 1;
    }
    let mut it = posicionales.into_iter();
    let maquina = it.next().ok_or(ErrorUso::Falta("maquina"))?;
    let programa = it.next().ok_or(ErrorUso::Falta("programa"))?;
    if let Some(extra) = it.next() {
        return Err(ErrorUso::Sobra(extra));
    }
    Ok(Orden::Desensamblar {
        maquina,
        programa,
        direccion,
        simbolos,
    })
}

fn interpretar_volcado(texto: &str) -> Result<Volcado, ErrorUso> {
    let invalido = || ErrorUso::VolcadoInvalido(texto.to_string());
    let mut partes = texto.splitn(3, ':');
    let desde = partes
        .next()
        .filter(|p| !p.is_empty())
        .ok_or_else(invalido)?;
    let cantidad = partes
        .next()
        .filter(|p| !p.is_empty())
        .ok_or_else(invalido)?;
    let archivo = partes.next().map(str::to_string);
    if archivo.as_deref() == Some("") {
        return Err(invalido());
    }
    Ok(Volcado {
        desde: numero("--volcar", desde)?,
        cantidad: numero("--volcar", cantidad)?,
        archivo,
    })
}

fn interpretar_disparo(texto: &str) -> Result<Disparo, ErrorUso> {
    let invalido = || ErrorUso::DisparoInvalido(texto.to_string());
    let (linea, instruccion) = texto.split_once('@').ok_or_else(invalido)?;
    if linea.is_empty() || instruccion.is_empty() {
        return Err(invalido());
    }
    let linea = numero("--interrumpir", linea)?;
    Ok(Disparo {
        linea: u32::try_from(linea).map_err(|_| invalido())?,
        instruccion: numero("--interrumpir", instruccion)?,
    })
}
