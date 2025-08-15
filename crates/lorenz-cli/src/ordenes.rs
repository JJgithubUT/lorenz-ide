//! Ejecucion de las ordenes de la linea de comandos.

use std::io::Write;

use lorenz_emu::{Emulador, ErrorEjecucion};

use crate::args::{interpretar, Orden};
use crate::programa::leer_palabras;

/// El programa termino bien: la maquina se detuvo.
pub const SALIDA_OK: i32 = 0;
/// La ejecucion fallo: acceso invalido, instruccion desconocida, division entre cero...
pub const SALIDA_ERROR_EJECUCION: i32 = 1;
/// Argumentos mal escritos, archivo ilegible o arquitectura invalida.
pub const SALIDA_ERROR_USO: i32 = 2;
/// Se alcanzo el limite de instrucciones sin que la maquina se detuviera.
pub const SALIDA_LIMITE: i32 = 3;

const AYUDA: &str = "\
Lorenz: emulador de maquinas definidas por el usuario

Uso:
  lorenz validar <maquina.toml>
  lorenz ejecutar <maquina.toml> <programa.hex> [--limite N] [--direccion A] [--traza]
  lorenz ayuda
  lorenz version

Codigos de salida:
  0  la maquina se detuvo
  1  fallo durante la ejecucion
  2  argumentos, archivos o arquitectura invalidos
  3  se alcanzo el limite de instrucciones
";

fn escribir(salida: &mut dyn Write, texto: &str) {
    // Si la salida esta cerrada (por ejemplo una tuberia), no hay a quien avisar.
    let _ = salida.write_all(texto.as_bytes());
}

/// Ejecuta la linea de comandos y devuelve el codigo de salida.
pub fn ejecutar_cli(args: &[String], salida: &mut dyn Write, errores: &mut dyn Write) -> i32 {
    let orden = match interpretar(args) {
        Ok(o) => o,
        Err(e) => {
            escribir(errores, &format!("error: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    match orden {
        Orden::Ayuda => {
            escribir(salida, AYUDA);
            SALIDA_OK
        }
        Orden::Version => {
            escribir(salida, &format!("lorenz {}\n", env!("CARGO_PKG_VERSION")));
            SALIDA_OK
        }
        Orden::Validar { maquina } => validar(&maquina, salida, errores),
        Orden::Ejecutar {
            maquina,
            programa,
            limite,
            traza,
            direccion,
        } => ejecutar(
            &maquina, &programa, limite, traza, direccion, salida, errores,
        ),
    }
}

fn validar(ruta: &str, salida: &mut dyn Write, errores: &mut dyn Write) -> i32 {
    match lorenz_arch::cargar_archivo(ruta) {
        Ok(m) => {
            escribir(
                salida,
                &format!(
                    "{ruta}: valida ('{}', {} instrucciones, {} regiones)\n",
                    m.nombre,
                    m.instrucciones.len(),
                    m.regiones.len()
                ),
            );
            SALIDA_OK
        }
        Err(e) => {
            escribir(errores, &format!("{ruta}: {e}\n"));
            SALIDA_ERROR_USO
        }
    }
}

fn ejecutar(
    ruta_maquina: &str,
    ruta_programa: &str,
    limite: u64,
    traza: bool,
    direccion: Option<u64>,
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
    let texto = match std::fs::read_to_string(ruta_programa) {
        Ok(t) => t,
        Err(e) => {
            escribir(errores, &format!("{ruta_programa}: no se pudo leer: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let palabras = match leer_palabras(&texto, maquina.bits_palabra) {
        Ok(p) => p,
        Err(e) => {
            escribir(errores, &format!("{ruta_programa}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let digitos = (maquina.bits_direccion as usize).div_ceil(4);
    let inicio = direccion.unwrap_or(maquina.arranque);
    let mut emulador = match Emulador::nuevo(maquina) {
        Ok(e) => e,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    if let Err(e) = emulador.cargar_programa(inicio, &palabras) {
        escribir(errores, &format!("{ruta_programa}: {e}\n"));
        return SALIDA_ERROR_USO;
    }
    emulador.estado_mut().fijar_pc(inicio);

    let mut ejecutadas = 0u64;
    let codigo = loop {
        if emulador.estado().detenida() {
            break SALIDA_OK;
        }
        if ejecutadas >= limite {
            let e = ErrorEjecucion::LimiteDeInstrucciones { limite };
            escribir(errores, &format!("error: {e}\n"));
            break SALIDA_LIMITE;
        }
        match emulador.paso() {
            Ok(Some(p)) => {
                ejecutadas += 1;
                if traza {
                    escribir(
                        salida,
                        &format!(
                            "{:0ancho$x}  {:<6} {} ciclos{}\n",
                            p.direccion,
                            p.mnemonico,
                            p.ciclos,
                            if p.salto { "  salto" } else { "" },
                            ancho = digitos
                        ),
                    );
                }
            }
            Ok(None) => break SALIDA_OK,
            Err(e) => {
                escribir(
                    errores,
                    &format!(
                        "error en {:0ancho$x}: {e}\n",
                        emulador.estado().pc(),
                        ancho = digitos
                    ),
                );
                break SALIDA_ERROR_EJECUCION;
            }
        }
    };
    resumen(&mut emulador, digitos, salida);
    codigo
}

fn resumen(emulador: &mut Emulador, digitos: usize, salida: &mut dyn Write) {
    let estado = emulador.estado();
    let mut texto = String::new();
    texto.push_str(&format!(
        "detenida: {}\ninstrucciones: {}\nciclos: {}\npc: {:0ancho$x}\n",
        if estado.detenida() { "si" } else { "no" },
        estado.instrucciones(),
        estado.ciclos(),
        estado.pc(),
        ancho = digitos
    ));
    let registros: Vec<String> = estado
        .nombres()
        .iter()
        .map(|n| format!("{n}={:#x}", estado.registro(n).unwrap_or(0)))
        .collect();
    texto.push_str(&format!("registros: {}\n", registros.join(" ")));
    let banderas: Vec<String> = estado
        .nombres_de_banderas()
        .iter()
        .map(|n| format!("{n}={}", u8::from(estado.bandera(n).unwrap_or(false))))
        .collect();
    if !banderas.is_empty() {
        texto.push_str(&format!("banderas: {}\n", banderas.join(" ")));
    }
    let puertos = emulador.puertos().numeros();
    for p in puertos {
        let escritos = emulador
            .puertos_mut()
            .salida(p)
            .map(<[u64]>::to_vec)
            .unwrap_or_default();
        if !escritos.is_empty() {
            let lista: Vec<String> = escritos.iter().map(|v| format!("{v:#x}")).collect();
            texto.push_str(&format!("salida del puerto {p:#x}: {}\n", lista.join(" ")));
        }
    }
    escribir(salida, &texto);
}
