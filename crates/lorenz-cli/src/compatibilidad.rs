//! `lorenz compatibilidad`: comprobar que un binario hecho con una arquitectura sigue sirviendo
//! con la version nueva (RF-TST-029).

use std::io::Write;

use lorenz_tst::compatibilidad::comprobar_binario;

use crate::ensamblar::cargar_segmentos;
use crate::ordenes::SALIDA_ERROR_USO;

fn escribir(salida: &mut dyn Write, texto: &str) {
    let _ = salida.write_all(texto.as_bytes());
}

/// `compatibilidad` encontro instrucciones que la arquitectura nueva ya no entiende igual.
pub const SALIDA_INCOMPATIBLE: i32 = 1;

pub fn compatibilidad_cli(
    ruta_anterior: &str,
    ruta_nueva: &str,
    programa: &str,
    direccion: Option<u64>,
    json: bool,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let cargar = |ruta: &str, errores: &mut dyn Write| match lorenz_arch::cargar_archivo(ruta) {
        Ok(m) => Some(m),
        Err(e) => {
            escribir(errores, &format!("{ruta}: {e}\n"));
            None
        }
    };
    let (Some(anterior), Some(nueva)) =
        (cargar(ruta_anterior, errores), cargar(ruta_nueva, errores))
    else {
        return SALIDA_ERROR_USO;
    };
    // El binario se lee con la arquitectura con que se hizo.
    let tramos = match cargar_segmentos(&anterior, programa, direccion.unwrap_or(anterior.arranque))
    {
        Ok(t) => t,
        Err(e) => {
            escribir(errores, &format!("{e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let informe = match comprobar_binario(&anterior, &nueva, &tramos) {
        Ok(i) => i,
        Err(e) => {
            escribir(errores, &format!("error: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    if json {
        escribir(salida, &informe.a_json());
    } else {
        escribir(salida, &informe.a_texto());
    }
    if informe.es_compatible() {
        0
    } else {
        SALIDA_INCOMPATIBLE
    }
}
