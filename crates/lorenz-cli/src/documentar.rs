//! La orden `documentar` (RF-ECO-017 a RF-ECO-023, RF-ECO-026, RF-ARQ-036).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::time::Duration;

use lorenz_arch::documentar::{documentar, Comentarios};

use crate::ordenes::{SALIDA_ERROR_USO, SALIDA_OK};

fn escribir(salida: &mut dyn Write, texto: &str) {
    let _ = salida.write_all(texto.as_bytes());
}

/// Cada cuanto mira `--vigilar` si la descripcion cambio.
const PERIODO_DE_VIGILANCIA: Duration = Duration::from_millis(500);

/// Genera la documentacion de una descripcion a partir de su texto.
///
/// Si la descripcion hereda de otras, los comentarios que se incorporan son los de su propio archivo.
pub fn generar(ruta: &str, texto: &str) -> Result<String, String> {
    let maquina = lorenz_arch::cargar_archivo(ruta).map_err(|e| format!("{ruta}: {e}"))?;
    Ok(documentar(&maquina, &Comentarios::leer(texto)))
}

/// Huella de la descripcion y de todas las que hereda.
fn huella(ruta: &str) -> Option<u64> {
    let mut h = DefaultHasher::new();
    for archivo in lorenz_arch::cadena_de_herencia(std::path::Path::new(ruta)) {
        std::fs::read_to_string(archivo).ok()?.hash(&mut h);
    }
    Some(h.finish())
}

/// Regenera la documentacion cuando la descripcion cambia (RF-ECO-026).
///
/// Recuerda la huella del ultimo texto visto, valido o no, para no repetir el mismo error en cada
/// vuelta; una descripcion rota no borra la documentacion anterior.
#[derive(Debug)]
pub struct Vigilante {
    maquina: String,
    salida: String,
    ultima: Option<u64>,
}

/// Lo que paso en una pasada del vigilante.
#[derive(Debug, PartialEq, Eq)]
pub enum Pasada {
    SinCambios,
    Regenerada,
    Fallo(String),
}

impl Vigilante {
    pub fn nuevo(maquina: &str, salida: &str) -> Self {
        Self {
            maquina: maquina.to_string(),
            salida: salida.to_string(),
            ultima: None,
        }
    }

    pub fn pasar(&mut self) -> Pasada {
        let texto = match std::fs::read_to_string(&self.maquina) {
            Ok(t) => t,
            Err(e) => return Pasada::Fallo(format!("{}: no se pudo leer: {e}", self.maquina)),
        };
        let h = huella(&self.maquina);
        if h.is_some() && self.ultima == h {
            return Pasada::SinCambios;
        }
        self.ultima = h;
        match generar(&self.maquina, &texto) {
            Ok(doc) => match std::fs::write(&self.salida, doc) {
                Ok(()) => Pasada::Regenerada,
                Err(e) => Pasada::Fallo(format!("{}: no se pudo escribir: {e}", self.salida)),
            },
            Err(e) => Pasada::Fallo(e),
        }
    }
}

pub fn documentar_cli(
    maquina: &str,
    salida_ruta: Option<&str>,
    vigilar: bool,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    if vigilar {
        let Some(destino) = salida_ruta else {
            escribir(errores, "--vigilar necesita -o ARCHIVO\n");
            return SALIDA_ERROR_USO;
        };
        let mut v = Vigilante::nuevo(maquina, destino);
        loop {
            match v.pasar() {
                Pasada::Regenerada => escribir(salida, &format!("{destino}: regenerada\n")),
                Pasada::Fallo(e) => escribir(errores, &format!("{e}\n")),
                Pasada::SinCambios => {}
            }
            let _ = salida.flush();
            std::thread::sleep(PERIODO_DE_VIGILANCIA);
        }
    }
    let texto = match std::fs::read_to_string(maquina) {
        Ok(t) => t,
        Err(e) => {
            escribir(errores, &format!("{maquina}: no se pudo leer: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let doc = match generar(maquina, &texto) {
        Ok(d) => d,
        Err(e) => {
            escribir(errores, &format!("{e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    match salida_ruta {
        Some(destino) => {
            if let Err(e) = std::fs::write(destino, doc) {
                escribir(errores, &format!("{destino}: no se pudo escribir: {e}\n"));
                return SALIDA_ERROR_USO;
            }
        }
        None => escribir(salida, &doc),
    }
    SALIDA_OK
}
