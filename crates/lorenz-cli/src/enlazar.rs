//! La orden `enlazar` (RF-ENS-019, RF-ENS-020).

use std::io::Write;

use lorenz_ens::enlazador::enlazar;
use lorenz_ens::ensamblador::{Ensamblador, Objeto};
use lorenz_ens::salidas::{mapa, tabla_de_simbolos};

use crate::ensamblar::{ensamblar_archivo, es_fuente, imagen_hex, OpcionesEnsamblar};
use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};

fn escribir(salida: &mut dyn Write, texto: &str) {
    let _ = salida.write_all(texto.as_bytes());
}

/// Un objeto guardado, o un fuente que se ensambla al vuelo.
fn cargar(maquina: &lorenz_arch::modelo::Maquina, ruta: &str) -> Result<(String, Objeto), String> {
    if es_fuente(ruta) {
        return ensamblar_archivo(maquina, ruta).map(|(o, _)| (ruta.to_string(), o));
    }
    let texto =
        std::fs::read_to_string(ruta).map_err(|e| format!("{ruta}: no se pudo leer: {e}"))?;
    let (nombre, objeto) = Objeto::desde_toml(&texto).map_err(|e| format!("{ruta}: {e}"))?;
    if nombre != maquina.nombre {
        return Err(format!(
            "{ruta}: se ensamblo para la maquina `{nombre}`, no para `{}`",
            maquina.nombre
        ));
    }
    Ok((ruta.to_string(), objeto))
}

pub fn enlazar_cli(
    ruta_maquina: &str,
    entradas: &[String],
    opciones: &OpcionesEnsamblar,
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
    let mut objetos = Vec::new();
    for ruta in entradas {
        match cargar(&maquina, ruta) {
            Ok(o) => objetos.push(o),
            Err(e) => {
                escribir(errores, &format!("{e}\n"));
                return SALIDA_ERROR_USO;
            }
        }
    }
    let ensamblador = match Ensamblador::nuevo(&maquina) {
        Ok(e) => e,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let enlazado = match enlazar(&ensamblador, &objetos) {
        Ok(o) => o,
        Err(es) => {
            for e in &es {
                escribir(errores, &format!("{e}\n"));
            }
            return SALIDA_ERROR_EJECUCION;
        }
    };
    let hex = imagen_hex(&enlazado, &maquina);
    let mut escribir_en = |ruta: &str, texto: &str| -> bool {
        match std::fs::write(ruta, texto) {
            Ok(()) => true,
            Err(e) => {
                escribir(errores, &format!("{ruta}: no se pudo escribir: {e}\n"));
                false
            }
        }
    };
    match &opciones.salida {
        Some(ruta) => {
            if !escribir_en(ruta, &hex) {
                return SALIDA_ERROR_USO;
            }
        }
        None => escribir(salida, &hex),
    }
    for (ruta, texto) in [
        (&opciones.mapa, mapa(&enlazado)),
        (&opciones.simbolos, tabla_de_simbolos(&enlazado)),
        (&opciones.objeto, enlazado.a_toml(&maquina.nombre)),
    ] {
        if let Some(ruta) = ruta {
            if !escribir_en(ruta, &texto) {
                return SALIDA_ERROR_USO;
            }
        }
    }
    if opciones.salida.is_some() {
        escribir(
            salida,
            &format!(
                "{} objeto(s) enlazados: {} palabras en {} segmento(s)\n",
                objetos.len(),
                enlazado.tamano(),
                enlazado.segmentos.len()
            ),
        );
    }
    SALIDA_OK
}
