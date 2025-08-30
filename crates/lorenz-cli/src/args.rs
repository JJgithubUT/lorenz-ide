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
    /// Cargar un programa y ejecutarlo.
    Ejecutar {
        maquina: String,
        programa: String,
        limite: u64,
        traza: bool,
        /// Dibujar las pantallas de la maquina al terminar.
        pantalla: bool,
        /// Teclas que se pulsan en el primer teclado antes de empezar, una por caracter.
        teclas: Option<String>,
        /// Texto que se entrega a la primera UART antes de empezar.
        serie: Option<String>,
        /// Rangos de memoria que se vuelcan al terminar.
        volcados: Vec<Volcado>,
        /// Interrupciones que se disparan a mano.
        disparos: Vec<Disparo>,
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
        otra => Err(ErrorUso::OrdenDesconocida(otra.to_string())),
    }
}

fn interpretar_ejecutar(resto: &[String]) -> Result<Orden, ErrorUso> {
    let mut posicionales: Vec<String> = Vec::new();
    let mut limite = LIMITE_POR_OMISION;
    let mut traza = false;
    let mut pantalla = false;
    let mut teclas = None;
    let mut serie = None;
    let mut volcados = Vec::new();
    let mut disparos = Vec::new();
    let mut direccion = None;
    let mut i = 0;
    while i < resto.len() {
        let a = &resto[i];
        match a.as_str() {
            "--traza" => traza = true,
            "--pantalla" => pantalla = true,
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
        pantalla,
        teclas,
        serie,
        volcados,
        disparos,
        direccion,
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
