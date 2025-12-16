//! Recarga del ensamblador cuando la descripcion de arquitectura cambia (RF-ENS-030).
//!
//! El ensamblador se genera a partir de la descripcion, asi que si esta cambia hay que volver a
//! generarlo. [`Recargable`] guarda la maquina cargada y, cada vez que se le pide, mira si el
//! archivo cambio. Si la version nueva es valida la sustituye; si no lo es se queda con la anterior
//! y devuelve el error, de modo que una edicion a medias nunca deja al usuario sin ensamblador.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use lorenz_arch::modelo::Maquina;
use lorenz_arch::ErrorArquitectura;

use crate::ensamblador::Ensamblador;

/// Una descripcion de arquitectura cargada de un archivo que se puede volver a leer.
#[derive(Debug)]
pub struct Recargable {
    ruta: PathBuf,
    maquina: Maquina,
    /// Huella del ultimo texto leido, valido o no, para no repetir el mismo error.
    huella: u64,
    version: u64,
}

/// Lo que paso al pedir una recarga.
#[derive(Debug)]
pub enum Recarga {
    /// El archivo no cambio desde la ultima lectura.
    SinCambios,
    /// El archivo cambio y la maquina nueva ya esta en uso.
    Recargada,
    /// El archivo cambio pero no es valido; se sigue con la maquina anterior.
    Rechazada(ErrorArquitectura),
}

/// Huella del texto de la descripcion y de todas las que hereda: si cambia una, cambia la huella.
fn huella_de(ruta: &Path) -> Result<u64, std::io::Error> {
    let mut h = DefaultHasher::new();
    for archivo in lorenz_arch::cadena_de_herencia(ruta) {
        std::fs::read_to_string(&archivo)?.hash(&mut h);
    }
    Ok(h.finish())
}

impl Recargable {
    /// Carga la descripcion por primera vez; aqui un error si es definitivo.
    pub fn abrir(ruta: impl AsRef<Path>) -> Result<Self, ErrorArquitectura> {
        let ruta = ruta.as_ref().to_path_buf();
        let huella = huella_de(&ruta)?;
        let maquina = lorenz_arch::cargar_archivo(&ruta)?;
        Ok(Self {
            ruta,
            maquina,
            huella,
            version: 1,
        })
    }

    /// La maquina vigente.
    pub fn maquina(&self) -> &Maquina {
        &self.maquina
    }

    /// Cuantas veces se ha cargado una maquina nueva; empieza en 1.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Ensamblador generado para la maquina vigente.
    pub fn ensamblador(&self) -> Result<Ensamblador<'_>, String> {
        Ensamblador::nuevo(&self.maquina)
    }

    /// Vuelve a leer el archivo y, si cambio y es valido, cambia de maquina.
    pub fn recargar(&mut self) -> Recarga {
        let huella = match huella_de(&self.ruta) {
            Ok(h) => h,
            Err(e) => return Recarga::Rechazada(e.into()),
        };
        if huella == self.huella {
            return Recarga::SinCambios;
        }
        self.huella = huella;
        match lorenz_arch::cargar_archivo(&self.ruta) {
            Ok(m) if Ensamblador::nuevo(&m).is_ok() => {
                self.maquina = m;
                self.version += 1;
                Recarga::Recargada
            }
            Ok(_) => Recarga::Rechazada(ErrorArquitectura::Codificacion {
                mnemonico: String::new(),
                causa: "el ensamblador no se pudo generar con la descripcion nueva".into(),
            }),
            Err(e) => Recarga::Rechazada(e),
        }
    }
}
