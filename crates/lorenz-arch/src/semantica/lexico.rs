//! Analizador lexico: convierte el texto de una semantica en una secuencia de lexemas.

/// Una pieza minima del texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Numero(u64),
    Ident(String),
    // palabras reservadas
    Si,
    Detener,
    // aritmeticos
    Mas,
    Menos,
    Estrella,
    Barra,
    Porcentaje,
    // bits
    DesplazaIzq,
    DesplazaDer,
    Ampersand,
    Tuberia,
    Circunflejo,
    Tilde,
    // logicos y comparacion
    Admiracion,
    AmpAmp,
    TuberiaTuberia,
    IgualIgual,
    Distinto,
    Menor,
    MenorIgual,
    Mayor,
    MayorIgual,
    // puntuacion
    Igual,
    PuntoYComa,
    DosPuntos,
    ParenAbre,
    ParenCierra,
    CorcheteAbre,
    CorcheteCierra,
    LlaveAbre,
    LlaveCierra,
    /// Fin del texto.
    Fin,
}

/// Un token con el caracter donde empieza.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lexema {
    pub token: Token,
    pub posicion: usize,
}
