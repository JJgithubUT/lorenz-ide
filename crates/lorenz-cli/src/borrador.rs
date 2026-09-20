//! `lorenz borrador`: un borrador de arquitectura a partir de una descripcion en lenguaje natural
//! (RF-ECO-045). Sin modelos ni red: reglas fijas, ver `lorenz_arch::borrador`.

use std::io::Write;

use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};

pub fn borrador_cli(
    descripcion: &str,
    archivo: Option<&str>,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let b = match lorenz_arch::borrador::desde_descripcion(descripcion) {
        Ok(b) => b,
        Err(e) => {
            let _ = writeln!(errores, "borrador: {e}");
            return SALIDA_ERROR_USO;
        }
    };
    match archivo {
        Some(ruta) => {
            if let Err(e) = std::fs::write(ruta, &b.texto) {
                let _ = writeln!(errores, "{ruta}: no se pudo escribir: {e}");
                return SALIDA_ERROR_EJECUCION;
            }
            let _ = writeln!(salida, "Borrador escrito en {ruta}");
        }
        None => {
            let _ = write!(salida, "{}", b.texto);
        }
    }
    let _ = writeln!(errores, "Supuestos (revisalos):");
    for s in &b.supuestos {
        let _ = writeln!(errores, "  - {s}");
    }
    SALIDA_OK
}
