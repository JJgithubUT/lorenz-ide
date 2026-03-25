//! Errores al leer o validar un archivo de pruebas.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorPruebas {
    #[error("{ruta}: no se pudo leer: {motivo}")]
    Lectura { ruta: String, motivo: String },

    #[error("{ruta}: {motivo}")]
    Sintaxis { ruta: String, motivo: String },

    #[error("el formato `{encontrado}` no se reconoce; esta version lee `{esperado}`")]
    Formato {
        encontrado: String,
        esperado: &'static str,
    },

    #[error("la arquitectura `{ruta}` no se pudo cargar: {motivo}")]
    Maquina { ruta: String, motivo: String },

    #[error("prueba `{prueba}`: {motivo}")]
    Prueba { prueba: String, motivo: String },

    #[error("hay dos pruebas con el nombre `{0}`")]
    NombreRepetido(String),
}
