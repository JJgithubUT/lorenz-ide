//! El modelo en memoria de una maquina descrita.
//!
//! Es lo que el cargador entrega y lo que consultan el ensamblador, el emulador y el generador de
//! documentacion. Todo aqui esta ya validado: quien lo consume no vuelve a comprobar invariantes.

use std::collections::BTreeMap;

use crate::bits::Codificacion;

/// Orden de bytes de la maquina.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
}

/// Sentido de crecimiento de la pila.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentidoPila {
    Descendente,
    Ascendente,
}

/// Papel que un registro cumple para las herramientas.
///
/// El rol es lo que permite al depurador saber cual registro es el contador de programa sin que el
/// usuario tenga que llamarlo `PC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolRegistro {
    General,
    ContadorPrograma,
    PunteroPila,
    Banderas,
    Indirecto,
    Fijo,
}

impl RolRegistro {
    /// Un rol unico no puede aparecer en dos registros de la misma maquina.
    pub fn es_unico(self) -> bool {
        matches!(
            self,
            RolRegistro::ContadorPrograma | RolRegistro::PunteroPila | RolRegistro::Banderas
        )
    }
}

/// Un registro de la maquina.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registro {
    pub nombre: String,
    pub bits: u32,
    pub rol: RolRegistro,
    pub inicial: u64,
}

/// Un bit con nombre dentro del registro de banderas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bandera {
    pub nombre: String,
    pub bit: u32,
    pub descripcion: String,
}

/// Que acepta un operando de una instruccion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TipoOperando {
    /// Un registro, limitado a la lista dada y codificado por su posicion en ella.
    Registro { permitidos: Vec<String> },
    /// Una constante.
    Inmediato { signo: bool, rango: (i64, i64) },
    /// Una direccion absoluta.
    Direccion,
    /// Un desplazamiento relativo al contador de programa, siempre con signo.
    Desplazamiento,
    /// Un puerto de entrada y salida.
    Puerto,
}

/// Un operando declarado por una instruccion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operando {
    pub nombre: String,
    pub tipo: TipoOperando,
}

/// Coste en ciclos de una instruccion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ciclos {
    /// Siempre el mismo coste.
    Fijo(u32),
    /// Coste distinto segun el salto se tome o no.
    SegunSalto { tomado: u32, no_tomado: u32 },
}

impl Ciclos {
    /// Coste que se usa cuando todavia no se sabe el resultado del salto.
    pub fn peor_caso(self) -> u32 {
        match self {
            Ciclos::Fijo(n) => n,
            Ciclos::SegunSalto { tomado, no_tomado } => tomado.max(no_tomado),
        }
    }
}

/// Una instruccion de la ISA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruccion {
    pub mnemonico: String,
    pub sintaxis: String,
    pub codificacion: Codificacion,
    pub operandos: Vec<Operando>,
    pub semantica: String,
    pub afecta: Vec<String>,
    pub ciclos: Ciclos,
    pub descripcion: String,
}

/// Permisos de una region de memoria.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Acceso {
    pub lectura: bool,
    pub escritura: bool,
}

impl Acceso {
    pub fn solo_lectura() -> Self {
        Self { lectura: true, escritura: false }
    }

    pub fn lectura_escritura() -> Self {
        Self { lectura: true, escritura: true }
    }
}

/// Un tramo del mapa de direcciones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub nombre: String,
    pub desde: u64,
    pub hasta: u64,
    pub acceso: Acceso,
    /// Dispositivo que responde a esta region, si no es memoria normal.
    pub dispositivo: Option<String>,
    /// Archivo a precargar al arrancar.
    pub contenido: Option<String>,
}

impl Region {
    /// Numero de direcciones que cubre la region, con ambos extremos incluidos.
    pub fn tamano(&self) -> u64 {
        self.hasta - self.desde + 1
    }

    pub fn contiene(&self, direccion: u64) -> bool {
        direccion >= self.desde && direccion <= self.hasta
    }

    pub fn se_solapa_con(&self, otra: &Region) -> bool {
        self.desde <= otra.hasta && otra.desde <= self.hasta
    }
}

/// Clase de dispositivo que el emulador instancia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TipoDispositivo {
    Pantalla { ancho: u32, alto: u32, formato: String },
    Teclado,
    Uart,
    Temporizador,
    Bloques { tamano_bloque: u32 },
    Generico { script: String },
}

/// Un periferico conectado a la maquina.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dispositivo {
    pub nombre: String,
    pub tipo: TipoDispositivo,
    /// Puertos con nombre, por ejemplo `datos` y `estado`.
    pub puertos: BTreeMap<String, u64>,
    /// Linea de interrupcion que levanta, si levanta alguna.
    pub interrupcion: Option<u32>,
}

/// Configuracion de interrupciones de la maquina.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Interrupciones {
    /// Linea a direccion del vector.
    pub vectores: BTreeMap<u32, u64>,
    pub habilitadas_al_arrancar: bool,
}

/// Una maquina completa, ya validada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Maquina {
    pub nombre: String,
    pub formato: String,
    pub bits_palabra: u32,
    pub bits_direccion: u32,
    pub endianness: Endianness,
    pub arranque: u64,
    pub pila: SentidoPila,
    pub registros: Vec<Registro>,
    pub banderas: Vec<Bandera>,
    pub instrucciones: Vec<Instruccion>,
    pub regiones: Vec<Region>,
    pub dispositivos: Vec<Dispositivo>,
    pub interrupciones: Option<Interrupciones>,
}

impl Maquina {
    /// Registro que cumple un rol unico, si existe.
    pub fn registro_con_rol(&self, rol: RolRegistro) -> Option<&Registro> {
        self.registros.iter().find(|r| r.rol == rol)
    }

    pub fn registro(&self, nombre: &str) -> Option<&Registro> {
        self.registros.iter().find(|r| r.nombre == nombre)
    }

    pub fn bandera(&self, nombre: &str) -> Option<&Bandera> {
        self.banderas.iter().find(|b| b.nombre == nombre)
    }

    pub fn instruccion(&self, mnemonico: &str) -> Option<&Instruccion> {
        self.instrucciones.iter().find(|i| i.mnemonico == mnemonico)
    }

    pub fn dispositivo(&self, nombre: &str) -> Option<&Dispositivo> {
        self.dispositivos.iter().find(|d| d.nombre == nombre)
    }

    /// Region que cubre una direccion dada.
    pub fn region_de(&self, direccion: u64) -> Option<&Region> {
        self.regiones.iter().find(|r| r.contiene(direccion))
    }

    /// Mayor direccion representable con el ancho de direccion declarado.
    pub fn direccion_maxima(&self) -> u64 {
        if self.bits_direccion >= 64 {
            u64::MAX
        } else {
            (1u64 << self.bits_direccion) - 1
        }
    }

    /// Mayor valor representable en una palabra.
    pub fn palabra_maxima(&self) -> u64 {
        if self.bits_palabra >= 64 {
            u64::MAX
        } else {
            (1u64 << self.bits_palabra) - 1
        }
    }
}
