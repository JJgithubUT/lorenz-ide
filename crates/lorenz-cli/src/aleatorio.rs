//! `lorenz aleatorio`: comparar el emulador con el evaluador de referencia sobre programas
//! generados al azar (RF-TST-024, RF-TST-025, RF-TST-026, RF-TST-035).

use std::io::Write;

use lorenz_tst::aleatorio::{ejecutar_aleatorio, Opciones};

use crate::ordenes::SALIDA_ERROR_USO;

fn escribir(salida: &mut dyn Write, texto: &str) {
    let _ = salida.write_all(texto.as_bytes());
}

/// Argumentos de `aleatorio` ya interpretados.
#[derive(Debug, Clone, Default)]
pub struct OpcionesAleatorio {
    pub semilla: u64,
    pub programas: Option<usize>,
    pub longitud: Option<usize>,
    pub limite: Option<u64>,
    pub sin_reducir: bool,
    pub json: bool,
    pub salida: Option<String>,
}

pub fn aleatorio_cli(
    ruta_maquina: &str,
    o: &OpcionesAleatorio,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let maquina = match lorenz_arch::cargar_archivo(ruta_maquina) {
        Ok(m) => m,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let por_omision = Opciones::default();
    let opciones = Opciones {
        semilla: o.semilla,
        programas: o.programas.unwrap_or(por_omision.programas),
        longitud: o.longitud.unwrap_or(por_omision.longitud),
        limite: o.limite.unwrap_or(por_omision.limite),
        reducir: !o.sin_reducir,
    };
    let informe = match ejecutar_aleatorio(&maquina, &opciones) {
        Ok(i) => i,
        Err(e) => {
            escribir(errores, &format!("error: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    if let Some(archivo) = &o.salida {
        if let Err(e) = std::fs::write(archivo, informe.a_json()) {
            escribir(errores, &format!("{archivo}: no se pudo escribir: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    }
    if o.json {
        escribir(salida, &informe.a_json());
    } else {
        escribir(salida, &informe.a_texto());
    }
    informe.codigo_de_salida()
}
