//! El lenguaje de campos de bits que describe como se codifica una instruccion.
//!
//! Una codificacion se escribe de izquierda a derecha, del bit mas significativo al menos
//! significativo. Los espacios solo agrupan visualmente y no significan nada.
//!
//! | Elemento | Escritura | Significado |
//! | --- | --- | --- |
//! | Bit fijo | `0`, `1` | Forma el codigo de operacion |
//! | Campo | `nombre:n` | `n` bits que reciben el operando `nombre` |
//! | Relleno | `.` | Bit ignorado al decodificar, cero al ensamblar |
//! | Palabra extra | `\|` | Lo que sigue ocupa la siguiente palabra |
//!
//! Un campo puede repetirse en palabras distintas, nunca dentro de la misma: sus fragmentos se
//! concatenan para formar un solo operando. Es lo que permite una direccion de 16 bits repartida en
//! dos palabras de 8 (`dir:8 | dir:8`). El primer fragmento es el menos significativo en una maquina
//! little endian y el mas significativo en una big endian.
//!
//! ```
//! use lorenz_arch::bits::Codificacion;
//!
//! let c = Codificacion::analizar("0011 rd:2 .. | imm:8").unwrap();
//! assert_eq!(c.palabras().len(), 2);
//! assert_eq!(c.palabras()[0].ancho(), 8);
//! assert!(c.campo("imm").is_some());
//! ```

use std::fmt;

/// Un trozo de una palabra de instruccion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pieza {
    /// Un bit con valor literal.
    Fijo(bool),
    /// Un bit ignorado al decodificar y puesto a cero al ensamblar.
    Relleno,
    /// Un campo con nombre que recibe un operando.
    Campo { nombre: String, bits: u32 },
}

impl Pieza {
    /// Cuantos bits ocupa la pieza.
    pub fn bits(&self) -> u32 {
        match self {
            Pieza::Fijo(_) | Pieza::Relleno => 1,
            Pieza::Campo { bits, .. } => *bits,
        }
    }
}

/// Una palabra de instruccion, como secuencia de piezas de mas a menos significativo.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Palabra {
    piezas: Vec<Pieza>,
}

impl Palabra {
    pub fn piezas(&self) -> &[Pieza] {
        &self.piezas
    }

    /// Suma de los bits de todas sus piezas.
    pub fn ancho(&self) -> u32 {
        self.piezas.iter().map(Pieza::bits).sum()
    }

    /// Mascara con un 1 en cada bit fijo o de relleno, y 0 en los bits de campo.
    ///
    /// Es la mascara con la que se comparan dos codificaciones para detectar ambiguedad.
    pub fn mascara(&self) -> u64 {
        let mut mascara = 0u64;
        let mut restantes = self.ancho();
        for pieza in &self.piezas {
            restantes -= pieza.bits();
            if matches!(pieza, Pieza::Fijo(_)) {
                mascara |= 1u64 << restantes;
            }
        }
        mascara
    }

    /// Valor formado solo por los bits fijos; los demas quedan en cero.
    pub fn valor_fijo(&self) -> u64 {
        let mut valor = 0u64;
        let mut restantes = self.ancho();
        for pieza in &self.piezas {
            restantes -= pieza.bits();
            if let Pieza::Fijo(true) = pieza {
                valor |= 1u64 << restantes;
            }
        }
        valor
    }

    /// Posicion del bit menos significativo de un campo dentro de la palabra.
    pub fn desplazamiento_de(&self, nombre: &str) -> Option<u32> {
        let mut restantes = self.ancho();
        for pieza in &self.piezas {
            restantes -= pieza.bits();
            if let Pieza::Campo { nombre: n, .. } = pieza {
                if n == nombre {
                    return Some(restantes);
                }
            }
        }
        None
    }
}

/// La codificacion completa de una instruccion: una o varias palabras.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Codificacion {
    palabras: Vec<Palabra>,
    texto: String,
}

/// Fallo al analizar una cadena de codificacion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorBits {
    /// Un caracter que el lenguaje no reconoce.
    CaracterInesperado { caracter: char, posicion: usize },
    /// Un campo escrito sin su numero de bits.
    CampoSinAncho { nombre: String },
    /// Un campo con un ancho que no es un numero valido.
    AnchoInvalido { nombre: String, texto: String },
    /// Un campo de cero bits.
    AnchoCero { nombre: String },
    /// Un campo de mas de 64 bits, que no cabe en el tipo que lo transporta.
    AnchoExcesivo { nombre: String, bits: u32 },
    /// El mismo nombre de campo dos veces en la misma codificacion.
    CampoDuplicado { nombre: String },
    /// Una codificacion vacia o una palabra vacia entre dos separadores.
    PalabraVacia,
}

impl fmt::Display for ErrorBits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorBits::CaracterInesperado { caracter, posicion } => write!(
                f,
                "caracter '{caracter}' inesperado en la posicion {posicion}; se esperaba 0, 1, '.', '|' o un campo nombre:bits"
            ),
            ErrorBits::CampoSinAncho { nombre } => {
                write!(f, "el campo '{nombre}' no declara cuantos bits ocupa")
            }
            ErrorBits::AnchoInvalido { nombre, texto } => {
                write!(f, "el ancho '{texto}' del campo '{nombre}' no es un numero")
            }
            ErrorBits::AnchoCero { nombre } => write!(f, "el campo '{nombre}' ocupa cero bits"),
            ErrorBits::AnchoExcesivo { nombre, bits } => {
                write!(f, "el campo '{nombre}' ocupa {bits} bits; el maximo es 64")
            }
            ErrorBits::CampoDuplicado { nombre } => {
                write!(f, "el campo '{nombre}' aparece dos veces en la misma codificacion")
            }
            ErrorBits::PalabraVacia => write!(f, "hay una palabra de instruccion sin ningun bit"),
        }
    }
}

impl std::error::Error for ErrorBits {}

impl Codificacion {
    /// Analiza una cadena del lenguaje de campos de bits.
    pub fn analizar(texto: &str) -> Result<Self, ErrorBits> {
        let mut palabras = Vec::new();
        let mut actual = Palabra::default();
        // nombres ya usados dentro de la palabra en curso: un campo puede repetirse en otra palabra
        let mut vistos: Vec<String> = Vec::new();

        let caracteres: Vec<char> = texto.chars().collect();
        let mut i = 0usize;

        while i < caracteres.len() {
            let c = caracteres[i];
            match c {
                ' ' | '\t' | '_' => i += 1,
                '0' | '1' => {
                    actual.piezas.push(Pieza::Fijo(c == '1'));
                    i += 1;
                }
                '.' => {
                    actual.piezas.push(Pieza::Relleno);
                    i += 1;
                }
                '|' => {
                    if actual.piezas.is_empty() {
                        return Err(ErrorBits::PalabraVacia);
                    }
                    palabras.push(std::mem::take(&mut actual));
                    vistos.clear();
                    i += 1;
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    let inicio = i;
                    while i < caracteres.len()
                        && (caracteres[i].is_ascii_alphanumeric() || caracteres[i] == '_')
                    {
                        i += 1;
                    }
                    let nombre: String = caracteres[inicio..i].iter().collect();

                    if i >= caracteres.len() || caracteres[i] != ':' {
                        return Err(ErrorBits::CampoSinAncho { nombre });
                    }
                    i += 1;

                    let inicio_ancho = i;
                    while i < caracteres.len() && caracteres[i].is_ascii_digit() {
                        i += 1;
                    }
                    let texto_ancho: String = caracteres[inicio_ancho..i].iter().collect();
                    if texto_ancho.is_empty() {
                        return Err(ErrorBits::CampoSinAncho { nombre });
                    }
                    let bits: u32 = texto_ancho.parse().map_err(|_| ErrorBits::AnchoInvalido {
                        nombre: nombre.clone(),
                        texto: texto_ancho.clone(),
                    })?;
                    if bits == 0 {
                        return Err(ErrorBits::AnchoCero { nombre });
                    }
                    if bits > 64 {
                        return Err(ErrorBits::AnchoExcesivo { nombre, bits });
                    }
                    if vistos.iter().any(|n| n == &nombre) {
                        return Err(ErrorBits::CampoDuplicado { nombre });
                    }
                    vistos.push(nombre.clone());
                    actual.piezas.push(Pieza::Campo { nombre, bits });
                }
                otro => {
                    return Err(ErrorBits::CaracterInesperado {
                        caracter: otro,
                        posicion: i,
                    })
                }
            }
        }

        if actual.piezas.is_empty() {
            return Err(ErrorBits::PalabraVacia);
        }
        palabras.push(actual);

        Ok(Self {
            palabras,
            texto: texto.to_string(),
        })
    }

    pub fn palabras(&self) -> &[Palabra] {
        &self.palabras
    }

    /// La cadena original, tal como la escribio el usuario.
    pub fn texto(&self) -> &str {
        &self.texto
    }

    /// Nombres de todos los campos, en el orden en que aparecen.
    pub fn campos(&self) -> Vec<&str> {
        let mut nombres = Vec::new();
        for palabra in &self.palabras {
            for pieza in &palabra.piezas {
                if let Pieza::Campo { nombre, .. } = pieza {
                    nombres.push(nombre.as_str());
                }
            }
        }
        nombres
    }

    /// Busca un campo por nombre y devuelve en que palabra esta y cuantos bits ocupa.
    pub fn campo(&self, nombre: &str) -> Option<UbicacionCampo> {
        for (indice, palabra) in self.palabras.iter().enumerate() {
            for pieza in &palabra.piezas {
                if let Pieza::Campo { nombre: n, bits } = pieza {
                    if n == nombre {
                        return Some(UbicacionCampo {
                            palabra: indice,
                            desplazamiento: palabra.desplazamiento_de(nombre).unwrap_or(0),
                            bits: *bits,
                        });
                    }
                }
            }
        }
        None
    }

    /// Todos los fragmentos de un campo, en el orden en que aparecen en la codificacion.
    ///
    /// Un campo que no se reparte devuelve un solo fragmento.
    pub fn fragmentos(&self, nombre: &str) -> Vec<UbicacionCampo> {
        let mut encontrados = Vec::new();
        for (indice, palabra) in self.palabras.iter().enumerate() {
            for pieza in &palabra.piezas {
                if let Pieza::Campo { nombre: n, bits } = pieza {
                    if n == nombre {
                        encontrados.push(UbicacionCampo {
                            palabra: indice,
                            desplazamiento: palabra.desplazamiento_de(nombre).unwrap_or(0),
                            bits: *bits,
                        });
                    }
                }
            }
        }
        encontrados
    }

    /// Bits totales de un campo sumando todos sus fragmentos; 0 si no existe.
    pub fn bits_totales(&self, nombre: &str) -> u32 {
        self.fragmentos(nombre).iter().map(|f| f.bits).sum()
    }

    /// Nombres de campo sin repetir, en el orden de su primera aparicion.
    pub fn campos_unicos(&self) -> Vec<&str> {
        let mut unicos: Vec<&str> = Vec::new();
        for nombre in self.campos() {
            if !unicos.contains(&nombre) {
                unicos.push(nombre);
            }
        }
        unicos
    }

    /// Cuantas palabras ocupa la instruccion.
    pub fn longitud_en_palabras(&self) -> usize {
        self.palabras.len()
    }

    /// Dos codificaciones son ambiguas si existe alguna palabra que ambas aceptarian.
    ///
    /// Se comparan solo los bits que ambas fijan: si en todos ellos coinciden, una secuencia de
    /// bits podria decodificarse como cualquiera de las dos.
    pub fn es_ambigua_con(&self, otra: &Codificacion) -> bool {
        let primera = match (self.palabras.first(), otra.palabras.first()) {
            (Some(a), Some(b)) => (a, b),
            _ => return false,
        };
        let (a, b) = primera;
        if a.ancho() != b.ancho() {
            return false;
        }
        let comun = a.mascara() & b.mascara();
        (a.valor_fijo() & comun) == (b.valor_fijo() & comun)
    }
}

impl fmt::Display for Codificacion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.texto)
    }
}

/// Donde vive un campo dentro de la codificacion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UbicacionCampo {
    /// Indice de la palabra que lo contiene.
    pub palabra: usize,
    /// Posicion de su bit menos significativo dentro de esa palabra.
    pub desplazamiento: u32,
    /// Cuantos bits ocupa.
    pub bits: u32,
}
