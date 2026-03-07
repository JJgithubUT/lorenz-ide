//! Errores de carga y validacion de una descripcion de arquitectura.
//!
//! Todo error que el usuario pueda ver se expresa en terminos de la maquina que describio, no del
//! analizador de TOML (RNF-USA-003).

use std::fmt;

use thiserror::Error;

/// Lugar del archivo donde se detecto el problema.
///
/// `linea` es 1 cuando no se pudo determinar una posicion mas precisa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Posicion {
    pub linea: usize,
    pub columna: usize,
}

impl Posicion {
    pub fn nueva(linea: usize, columna: usize) -> Self {
        Self { linea, columna }
    }
}

impl fmt::Display for Posicion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.linea, self.columna)
    }
}

/// Un fallo al cargar o validar una descripcion de arquitectura.
#[derive(Debug, Error)]
pub enum ErrorArquitectura {
    #[error("no se pudo leer la descripcion: {0}")]
    Lectura(#[from] std::io::Error),

    #[error("el archivo no es TOML valido: {0}")]
    Sintaxis(#[from] toml::de::Error),

    #[error("version de formato no soportada: la descripcion pide {pedida}, esta version entiende {soportada}")]
    VersionFormato { pedida: String, soportada: String },

    #[error("codificacion de instruccion invalida en '{mnemonico}': {causa}")]
    Codificacion { mnemonico: String, causa: String },

    #[error("herencia entre descripciones: {0}")]
    Herencia(String),

    #[error("la descripcion no es valida:{}", listar_fallos(.0))]
    Validacion(Vec<FalloValidacion>),
}

/// Un fallo por linea, para que el mensaje del error diga que esta mal y no solo que algo lo esta.
fn listar_fallos(fallos: &[FalloValidacion]) -> String {
    fallos.iter().map(|f| format!("\n  {f}")).collect()
}

/// Un incumplimiento concreto de las reglas de validacion.
///
/// Se acumulan todos los fallos antes de rechazar la descripcion, para que el usuario los vea de
/// una vez y no de uno en uno (RF-ENS-015 aplicado al cargador).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FalloValidacion {
    /// Regla incumplida, citada por el requisito que la exige.
    pub regla: Regla,
    /// Que se detecto, en terminos de la maquina.
    pub detalle: String,
    /// Donde, si se conoce.
    pub posicion: Option<Posicion>,
}

impl fmt::Display for FalloValidacion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.posicion {
            Some(p) => write!(f, "[{}] {} ({})", self.regla.codigo(), self.detalle, p),
            None => write!(f, "[{}] {}", self.regla.codigo(), self.detalle),
        }
    }
}

/// Reglas que el cargador hace cumplir antes de entregar una maquina.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regla {
    /// Los bits de cada palabra suman el ancho de palabra declarado.
    SumaDeBits,
    /// Dos instrucciones no pueden admitir la misma codificacion binaria.
    CodificacionAmbigua,
    /// Cada campo de la codificacion tiene operando declarado y viceversa.
    CampoSinOperando,
    /// Los roles unicos aparecen exactamente una vez.
    RolUnico,
    /// Las regiones de memoria no se solapan.
    RegionSolapada,
    /// La direccion de arranque cae dentro de una region.
    ArranqueFueraDeRegion,
    /// Toda region que nombra un dispositivo lo tiene declarado.
    DispositivoInexistente,
    /// Toda bandera citada por una instruccion esta declarada.
    BanderaInexistente,
    /// Nombres duplicados de registro, bandera, region o dispositivo.
    NombreDuplicado,
    /// Valores fuera de los limites admitidos por el formato.
    ValorFueraDeRango,
    /// La semantica de una instruccion no se puede analizar o cita nombres que no existen.
    SemanticaInvalida,
    /// La sintaxis de una instruccion esta mal formada o no cuadra con sus operandos.
    SintaxisInvalida,
    /// Un par de registros nombra registros que no existen, repite uno o no cabe en 64 bits.
    ParInvalido,
    /// Un banco de registros nombra registros que no existen, se repite o tiene menos de dos copias.
    BancoInvalido,
    /// Una region con bancos no cumple las condiciones para conmutar por puerto.
    BancoMemoriaInvalido,
}

impl Regla {
    /// Identificador corto, el que se imprime al usuario y el que citan las pruebas.
    pub fn codigo(self) -> &'static str {
        match self {
            Regla::SumaDeBits => "E-BITS",
            Regla::CodificacionAmbigua => "E-AMBIG",
            Regla::CampoSinOperando => "E-CAMPO",
            Regla::RolUnico => "E-ROL",
            Regla::RegionSolapada => "E-SOLAPE",
            Regla::ArranqueFueraDeRegion => "E-ARRANQUE",
            Regla::DispositivoInexistente => "E-DISPOSITIVO",
            Regla::BanderaInexistente => "E-BANDERA",
            Regla::NombreDuplicado => "E-DUPLICADO",
            Regla::ValorFueraDeRango => "E-RANGO",
            Regla::SemanticaInvalida => "E-SEMANT",
            Regla::SintaxisInvalida => "E-SINTAXIS",
            Regla::ParInvalido => "E-PAR",
            Regla::BancoInvalido => "E-BANCO",
            Regla::BancoMemoriaInvalido => "E-BANCO-MEM",
        }
    }

    /// Requisito de la ERS que exige la regla.
    pub fn requisito(self) -> &'static str {
        match self {
            Regla::SumaDeBits => "RF-ARQ-016",
            Regla::CodificacionAmbigua => "RF-ARQ-017",
            Regla::CampoSinOperando => "RF-ARQ-018",
            Regla::RolUnico => "RF-ARQ-009",
            Regla::RegionSolapada => "RF-ARQ-027",
            Regla::ArranqueFueraDeRegion => "RF-ARQ-028",
            Regla::DispositivoInexistente => "RF-ARQ-031",
            Regla::BanderaInexistente => "RF-ARQ-024",
            Regla::NombreDuplicado => "RF-ARQ-007",
            Regla::ValorFueraDeRango => "RF-ARQ-003",
            Regla::SemanticaInvalida => "RF-ARQ-022",
            Regla::SintaxisInvalida => "RF-ARQ-013",
            Regla::ParInvalido => "RF-ARQ-010",
            Regla::BancoInvalido => "RF-ARQ-012",
            Regla::BancoMemoriaInvalido => "RF-ARQ-033",
        }
    }
}
