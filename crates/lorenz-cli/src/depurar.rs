//! `lorenz depurar`: el depurador de linea de comandos (RF-DEP-001, RF-DEP-018).

use std::io::{BufRead, Write};

use lorenz_dbg::comandos::Sesion;
use lorenz_dbg::Depurador;
use lorenz_emu::Emulador;

use crate::ordenes::{SALIDA_ERROR_USO, SALIDA_OK};

/// Lo que se muestra antes de esperar una orden.
pub const INDICADOR: &str = "(lorenz) ";

fn escribir(salida: &mut dyn Write, texto: &str) {
    let _ = salida.write_all(texto.as_bytes());
}

pub fn depurar_cli(
    ruta_maquina: &str,
    ruta_programa: &str,
    guion: Option<&str>,
    entrada: &mut dyn BufRead,
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
    let inicio = maquina.arranque;
    let mut objeto = None;
    let segmentos = if crate::ensamblar::es_fuente(ruta_programa) {
        match crate::ensamblar::ensamblar_archivo(&maquina, ruta_programa) {
            Ok((o, fuentes)) => {
                let segs = o
                    .segmentos
                    .iter()
                    .map(|s| (s.direccion, s.palabras.clone()))
                    .collect::<Vec<_>>();
                objeto = Some((o, fuentes));
                segs
            }
            Err(e) => {
                escribir(errores, &format!("{e}\n"));
                return SALIDA_ERROR_USO;
            }
        }
    } else {
        match crate::ensamblar::cargar_segmentos(&maquina, ruta_programa, inicio) {
            Ok(s) => s,
            Err(e) => {
                escribir(errores, &format!("{e}\n"));
                return SALIDA_ERROR_USO;
            }
        }
    };
    let mut emulador = match Emulador::nuevo(maquina) {
        Ok(e) => e,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    for (desde, palabras) in &segmentos {
        if let Err(e) = emulador.cargar_programa(*desde, palabras) {
            escribir(errores, &format!("{ruta_programa}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    }
    emulador.estado_mut().fijar_pc(inicio);
    let mut depurador = Depurador::nuevo(emulador);
    if let Some((o, fuentes)) = &objeto {
        let pares: Vec<(String, String)> = fuentes
            .iter()
            .map(|f| (f.nombre.clone(), f.texto.clone()))
            .collect();
        depurador.con_fuente(o, &pares);
    }
    let mut sesion = Sesion::nueva(depurador);

    // Las ordenes vienen de un guion o de la entrada; del guion se muestra cada orden para que la
    // salida se lea como una sesion.
    let (mut lector, es_guion): (Box<dyn BufRead>, bool) = match guion {
        Some(ruta) => match std::fs::File::open(ruta) {
            Ok(f) => (Box::new(std::io::BufReader::new(f)), true),
            Err(e) => {
                escribir(errores, &format!("{ruta}: no se pudo leer: {e}\n"));
                return SALIDA_ERROR_USO;
            }
        },
        None => (Box::new(entrada), false),
    };
    escribir(
        salida,
        &format!(
            "{} cargado; escribe `ayuda` para ver las ordenes\n",
            ruta_programa
        ),
    );
    loop {
        escribir(salida, INDICADOR);
        let _ = salida.flush();
        let mut linea = String::new();
        match lector.read_line(&mut linea) {
            Ok(0) | Err(_) => {
                escribir(salida, "\n");
                break;
            }
            Ok(_) => {}
        }
        let linea = linea.trim_end();
        if es_guion {
            // Los comentarios y las lineas en blanco de un guion no son ordenes.
            if linea.trim().is_empty() || linea.trim_start().starts_with('#') {
                escribir(salida, "\n");
                continue;
            }
            escribir(salida, &format!("{linea}\n"));
        }
        let r = sesion.ejecutar(linea);
        if !r.texto.is_empty() {
            escribir(salida, &r.texto);
            if !r.texto.ends_with('\n') {
                escribir(salida, "\n");
            }
        }
        if r.salir {
            break;
        }
    }
    SALIDA_OK
}
