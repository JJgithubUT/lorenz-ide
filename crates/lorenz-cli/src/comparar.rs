//! La orden `lorenz comparar` (RF-EMU-046).

use std::io::Write;

use lorenz_emu::{comparar, Emulador, Veredicto};

use crate::ordenes::{SALIDA_DIFIEREN, SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_OK};
use crate::programa::leer_palabras;

fn escribir(salida: &mut dyn Write, texto: &str) {
    let _ = salida.write_all(texto.as_bytes());
}

/// Carga una maquina con el programa puesto en su direccion de arranque (o en `direccion`).
fn preparar(
    ruta_maquina: &str,
    ruta_programa: &str,
    direccion: Option<u64>,
    errores: &mut dyn Write,
) -> Result<Emulador, i32> {
    let maquina = lorenz_arch::cargar_archivo(ruta_maquina).map_err(|e| {
        escribir(errores, &format!("{ruta_maquina}: {e}\n"));
        SALIDA_ERROR_USO
    })?;
    let texto = std::fs::read_to_string(ruta_programa).map_err(|e| {
        escribir(errores, &format!("{ruta_programa}: no se pudo leer: {e}\n"));
        SALIDA_ERROR_USO
    })?;
    let palabras = leer_palabras(&texto, maquina.bits_palabra).map_err(|e| {
        escribir(errores, &format!("{ruta_programa}: {e}\n"));
        SALIDA_ERROR_USO
    })?;
    let inicio = direccion.unwrap_or(maquina.arranque);
    let mut emulador = Emulador::nuevo(maquina).map_err(|e| {
        escribir(errores, &format!("{ruta_maquina}: {e}\n"));
        SALIDA_ERROR_USO
    })?;
    emulador.cargar_programa(inicio, &palabras).map_err(|e| {
        escribir(errores, &format!("{ruta_programa}: {e}\n"));
        SALIDA_ERROR_USO
    })?;
    emulador.estado_mut().fijar_pc(inicio);
    Ok(emulador)
}

/// Ejecuta el mismo programa en dos maquinas y dice si se comportan igual.
pub fn comparar_cli(
    rutas: [&str; 2],
    ruta_programa: &str,
    limite: u64,
    direccion: Option<u64>,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let mut a = match preparar(rutas[0], ruta_programa, direccion, errores) {
        Ok(e) => e,
        Err(codigo) => return codigo,
    };
    let mut b = match preparar(rutas[1], ruta_programa, direccion, errores) {
        Ok(e) => e,
        Err(codigo) => return codigo,
    };
    match comparar(&mut a, &mut b, limite) {
        Veredicto::Iguales { instrucciones } => {
            escribir(
                salida,
                &format!("iguales: {instrucciones} instrucciones sin diferencias\n"),
            );
            SALIDA_OK
        }
        Veredicto::LimiteAlcanzado { limite } => {
            escribir(
                salida,
                &format!("sin diferencias en {limite} instrucciones, pero ninguna se detuvo\n"),
            );
            SALIDA_LIMITE
        }
        Veredicto::Divergen {
            instruccion,
            diferencias,
            ..
        } => {
            escribir(
                salida,
                &format!("difieren en la instruccion {instruccion} (la primera es la 0):\n"),
            );
            for d in diferencias {
                escribir(salida, &format!("  {d}\n"));
            }
            SALIDA_DIFIEREN
        }
    }
}
