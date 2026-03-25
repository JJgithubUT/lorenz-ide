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
#[derive(Debug, Clone, PartialEq)]
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
    /// Compilar un fuente de SGT y escribir el programa en hexadecimal.
    Compilar {
        maquina: String,
        fuente: String,
        opciones: crate::compilar::OpcionesCompilar,
    },
    /// Ensamblar un fuente y escribir el programa en hexadecimal.
    Ensamblar {
        maquina: String,
        fuente: String,
        opciones: crate::ensamblar::OpcionesEnsamblar,
    },
    /// Compilar y ejecutar el mismo programa SGT en varias maquinas y compararlas.
    Cotejar {
        fuente: String,
        maquinas: Vec<String>,
        opciones: crate::cotejar::OpcionesCotejar,
    },
    /// Ejecutar un programa contando ciclos con la temporizacion de la descripcion.
    Ciclos {
        maquina: String,
        programa: String,
        opciones: crate::ciclos::OpcionesCiclos,
    },
    /// El mismo programa en dos descripciones, con sus cifras de ciclos lado a lado.
    CompararCiclos {
        maquina_a: String,
        maquina_b: String,
        programa: String,
        opciones: crate::ciclos::OpcionesCiclos,
    },
    /// Proponer instrucciones nuevas a partir de lo que ejecutan los programas.
    Sugerir {
        maquina: String,
        programas: Vec<String>,
        opciones: crate::sugerir::OpcionesSugerir,
    },
    /// Listar el historico de mediciones de ciclos.
    HistorialCiclos {
        archivo: String,
    },
    /// Crear un proyecto nuevo copiando una carpeta plantilla.
    Nuevo {
        plantilla: String,
        destino: String,
        nombre: Option<String>,
    },
    /// Escribir la convencion de llamada de SGT en una maquina.
    Convencion {
        maquina: String,
        salida: Option<String>,
    },
    /// Generar la documentacion de una arquitectura.
    Documentar {
        maquina: String,
        salida: Option<String>,
        vigilar: bool,
    },
    /// Enlazar varios objetos (o fuentes) en un programa.
    Enlazar {
        maquina: String,
        entradas: Vec<String>,
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
        /// Comparar con los resultados guardados en este archivo.
        comparar: Option<String>,
        /// Comparar con la ultima ejecucion de este proyecto y guardar esta para la proxima.
        historial: bool,
        /// Aumento de ciclos tolerado antes de contar como lentitud, en porcentaje.
        umbral: f64,
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
    /// Comparar el emulador con la referencia sobre programas aleatorios.
    Aleatorio {
        maquina: String,
        semilla: u64,
        programas: Option<usize>,
        longitud: Option<usize>,
        limite: Option<u64>,
        sin_reducir: bool,
        json: bool,
        salida: Option<String>,
    },
    /// Avisar de los binarios anteriores que una arquitectura nueva deja de entender.
    Compatibilidad {
        anterior: String,
        nueva: String,
        programa: String,
        direccion: Option<u64>,
        json: bool,
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
        /// Informar de las instrucciones y ramas que la ejecucion ejercito (RF-TST-021).
        cobertura: bool,
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
        "enlazar" => interpretar_enlazar(resto),
        "documentar" => interpretar_documentar(resto),
        "compilar" => interpretar_compilar(resto),
        "convencion" => interpretar_convencion(resto),
        "nuevo" => interpretar_nuevo(resto),
        "ciclos" => interpretar_ciclos(resto),
        "comparar-ciclos" => interpretar_comparar_ciclos(resto),
        "sugerir" => interpretar_sugerir(resto),
        "historial-ciclos" => match resto {
            [archivo] => Ok(Orden::HistorialCiclos {
                archivo: archivo.clone(),
            }),
            [] => Err(ErrorUso::Falta("archivo")),
            [_, extra, ..] => Err(ErrorUso::Sobra(extra.clone())),
        },
        "cotejar" => interpretar_cotejar(resto),
        "desensamblar" => interpretar_desensamblar(resto),
        "depurar" => interpretar_depurar(resto),
        "probar" => interpretar_probar(resto),
        "conformidad" => interpretar_conformidad(resto),
        "aleatorio" => interpretar_aleatorio(resto),
        "compatibilidad" => interpretar_compatibilidad(resto),
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
    let mut cobertura = false;
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
            "--cobertura" => cobertura = true,
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
        cobertura,
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

fn interpretar_compilar(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let mut opciones = crate::compilar::OpcionesCompilar::default();
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "-o" | "--salida" => opciones.salida = Some(valor_de(resto, &mut i, a)?.clone()),
            "--asm" => opciones.ensamblador = Some(valor_de(resto, &mut i, a)?.clone()),
            "--lineas" => opciones.lineas = Some(valor_de(resto, &mut i, a)?.clone()),
            "--costes" => opciones.costes = Some(valor_de(resto, &mut i, a)?.clone()),
            "-O0" | "-O1" | "-O2" | "-O3" => opciones.nivel = Some(a.as_bytes()[2] - b'0'),
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
    Ok(Orden::Compilar {
        maquina,
        fuente,
        opciones,
    })
}

fn interpretar_cotejar(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let mut opciones = crate::cotejar::OpcionesCotejar::default();
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--limite" => opciones.limite = Some(numero(a, valor_de(resto, &mut i, a)?)?),
            "-O0" | "-O1" | "-O2" | "-O3" => opciones.nivel = Some(a.as_bytes()[2] - b'0'),
            _ if a.starts_with('-') => return Err(ErrorUso::OpcionDesconocida(a.clone())),
            _ => posicionales.push(a.clone()),
        }
        i += 1;
    }
    let mut it = posicionales.into_iter();
    let fuente = it.next().ok_or(ErrorUso::Falta("fuente"))?;
    let maquinas: Vec<String> = it.collect();
    if maquinas.len() < 2 {
        return Err(ErrorUso::Falta("al menos dos maquinas"));
    }
    Ok(Orden::Cotejar {
        fuente,
        maquinas,
        opciones,
    })
}

fn interpretar_sugerir(resto: &[String]) -> Result<Orden, ErrorUso> {
    let (mut posicionales, mut opciones) = (Vec::new(), crate::sugerir::OpcionesSugerir::default());
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--limite" => opciones.limite = numero(a, valor_de(resto, &mut i, a)?)?,
            "--direccion" => opciones.direccion = numero(a, valor_de(resto, &mut i, a)?)?,
            "--minima" => opciones.minima = numero(a, valor_de(resto, &mut i, a)?)? as usize,
            "--maxima" => opciones.maxima = numero(a, valor_de(resto, &mut i, a)?)? as usize,
            "--cuantas" => opciones.cuantas = numero(a, valor_de(resto, &mut i, a)?)? as usize,
            _ if a.starts_with('-') => return Err(ErrorUso::OpcionDesconocida(a.clone())),
            _ => posicionales.push(a.clone()),
        }
        i += 1;
    }
    let mut it = posicionales.into_iter();
    let maquina = it.next().ok_or(ErrorUso::Falta("maquina"))?;
    let programas: Vec<String> = it.collect();
    if programas.is_empty() {
        return Err(ErrorUso::Falta("programa"));
    }
    Ok(Orden::Sugerir {
        maquina,
        programas,
        opciones,
    })
}

fn interpretar_ciclos(resto: &[String]) -> Result<Orden, ErrorUso> {
    let (mut posicionales, mut opciones) = (Vec::new(), crate::ciclos::OpcionesCiclos::default());
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--limite" => opciones.limite = numero(a, valor_de(resto, &mut i, a)?)?,
            "--direccion" => opciones.direccion = numero(a, valor_de(resto, &mut i, a)?)?,
            "--vista" => opciones.vista = Some(numero(a, valor_de(resto, &mut i, a)?)?),
            "--diagrama" => {
                opciones.diagrama = Some(numero(a, valor_de(resto, &mut i, a)?)? as usize)
            }
            "--perfil" => opciones.perfil = true,
            "--cache" => opciones.volcar_cache = true,
            "--calor" => opciones.calor = true,
            "--calor-html" => opciones.calor_html = Some(valor_de(resto, &mut i, a)?.clone()),
            "--historial" => opciones.historial = Some(valor_de(resto, &mut i, a)?.clone()),
            "--etiqueta" => opciones.etiqueta = Some(valor_de(resto, &mut i, a)?.clone()),
            "--vigilar" => opciones.vigilar = true,
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
    Ok(Orden::Ciclos {
        maquina,
        programa,
        opciones,
    })
}

fn interpretar_comparar_ciclos(resto: &[String]) -> Result<Orden, ErrorUso> {
    let (mut posicionales, mut opciones) = (Vec::new(), crate::ciclos::OpcionesCiclos::default());
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--limite" => opciones.limite = numero(a, valor_de(resto, &mut i, a)?)?,
            "--direccion" => opciones.direccion = numero(a, valor_de(resto, &mut i, a)?)?,
            _ if a.starts_with('-') => return Err(ErrorUso::OpcionDesconocida(a.clone())),
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
    Ok(Orden::CompararCiclos {
        maquina_a,
        maquina_b,
        programa,
        opciones,
    })
}

fn interpretar_nuevo(resto: &[String]) -> Result<Orden, ErrorUso> {
    let (mut posicionales, mut nombre) = (Vec::new(), None);
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--nombre" => nombre = Some(valor_de(resto, &mut i, a)?.clone()),
            _ if a.starts_with('-') => return Err(ErrorUso::OpcionDesconocida(a.clone())),
            _ => posicionales.push(a.clone()),
        }
        i += 1;
    }
    let mut it = posicionales.into_iter();
    let plantilla = it.next().ok_or(ErrorUso::Falta("plantilla"))?;
    let destino = it.next().ok_or(ErrorUso::Falta("destino"))?;
    if let Some(extra) = it.next() {
        return Err(ErrorUso::Sobra(extra));
    }
    Ok(Orden::Nuevo {
        plantilla,
        destino,
        nombre,
    })
}

fn interpretar_convencion(resto: &[String]) -> Result<Orden, ErrorUso> {
    let (mut posicionales, mut salida) = (Vec::new(), None);
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "-o" | "--salida" => salida = Some(valor_de(resto, &mut i, a)?.clone()),
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
    Ok(Orden::Convencion { maquina, salida })
}

fn interpretar_documentar(resto: &[String]) -> Result<Orden, ErrorUso> {
    let (mut posicionales, mut salida, mut vigilar) = (Vec::new(), None, false);
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "-o" | "--salida" => salida = Some(valor_de(resto, &mut i, a)?.clone()),
            "--vigilar" => vigilar = true,
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
    Ok(Orden::Documentar {
        maquina,
        salida,
        vigilar,
    })
}

fn interpretar_enlazar(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let mut opciones = crate::ensamblar::OpcionesEnsamblar::default();
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "-o" | "--salida" => opciones.salida = Some(valor_de(resto, &mut i, a)?.clone()),
            "--mapa" => opciones.mapa = Some(valor_de(resto, &mut i, a)?.clone()),
            "--simbolos" => opciones.simbolos = Some(valor_de(resto, &mut i, a)?.clone()),
            "--objeto" => opciones.objeto = Some(valor_de(resto, &mut i, a)?.clone()),
            _ if a.starts_with('-') => return Err(ErrorUso::OpcionDesconocida(a.clone())),
            _ => posicionales.push(a.clone()),
        }
        i += 1;
    }
    let mut it = posicionales.into_iter();
    let maquina = it.next().ok_or(ErrorUso::Falta("maquina"))?;
    let entradas: Vec<String> = it.collect();
    if entradas.is_empty() {
        return Err(ErrorUso::Falta("objeto"));
    }
    Ok(Orden::Enlazar {
        maquina,
        entradas,
        opciones,
    })
}

fn interpretar_probar(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let (mut nombre, mut etiqueta, mut salida, mut depurar) = (None, None, None, None);
    let (mut hilos, mut json, mut cobertura) = (0usize, false, false);
    let (mut comparar, mut historial, mut umbral) = (None, false, 0.0f64);
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--cobertura" => cobertura = true,
            "--historial" => historial = true,
            "--comparar" => comparar = Some(valor_de(resto, &mut i, a)?.clone()),
            "--umbral" => {
                let v = valor_de(resto, &mut i, a)?;
                umbral = v
                    .parse::<f64>()
                    .ok()
                    .filter(|x| x.is_finite() && *x >= 0.0)
                    .ok_or_else(|| ErrorUso::NumeroInvalido {
                        opcion: "--umbral".into(),
                        valor: v.clone(),
                    })?;
            }
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
        comparar,
        historial,
        umbral,
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

fn interpretar_aleatorio(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let mut salida = None;
    let (mut semilla, mut programas, mut longitud, mut limite) = (0u64, None, None, None);
    let (mut sin_reducir, mut json) = (false, false);
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--salida" => salida = Some(valor_de(resto, &mut i, a)?.clone()),
            "--json" => json = true,
            "--sin-reducir" => sin_reducir = true,
            "--semilla" => {
                let v = valor_de(resto, &mut i, a)?;
                semilla = numero("--semilla", v)?;
            }
            "--programas" => {
                let v = valor_de(resto, &mut i, a)?;
                programas = Some(numero("--programas", v)? as usize);
            }
            "--longitud" => {
                let v = valor_de(resto, &mut i, a)?;
                longitud = Some(numero("--longitud", v)? as usize);
            }
            "--limite" => {
                let v = valor_de(resto, &mut i, a)?;
                limite = Some(numero("--limite", v)?);
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
    Ok(Orden::Aleatorio {
        maquina,
        semilla,
        programas,
        longitud,
        limite,
        sin_reducir,
        json,
        salida,
    })
}

fn interpretar_compatibilidad(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let (mut direccion, mut json) = (None, false);
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--json" => json = true,
            "--direccion" => {
                let v = valor_de(resto, &mut i, a)?;
                direccion = Some(numero("--direccion", v)?);
            }
            _ if a.starts_with('-') => return Err(ErrorUso::OpcionDesconocida(a.clone())),
            _ => posicionales.push(a.clone()),
        }
        i += 1;
    }
    let mut it = posicionales.into_iter();
    let anterior = it.next().ok_or(ErrorUso::Falta("maquina-anterior"))?;
    let nueva = it.next().ok_or(ErrorUso::Falta("maquina-nueva"))?;
    let programa = it.next().ok_or(ErrorUso::Falta("programa"))?;
    if let Some(extra) = it.next() {
        return Err(ErrorUso::Sobra(extra));
    }
    Ok(Orden::Compatibilidad {
        anterior,
        nueva,
        programa,
        direccion,
        json,
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
