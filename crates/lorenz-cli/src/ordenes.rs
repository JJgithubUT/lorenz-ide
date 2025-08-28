//! Ejecucion de las ordenes de la linea de comandos.

use std::io::Write;

use lorenz_arch::modelo::TipoDispositivo;
use lorenz_emu::{Emulador, ErrorEjecucion};

use crate::args::{interpretar, Disparo, Orden, Volcado};
use crate::programa::{escribir_palabras, leer_palabras};

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
                  [--pantalla] [--teclas TEXTO] [--serie TEXTO]
                  [--volcar DESDE:CANTIDAD[:ARCHIVO]] [--interrumpir LINEA@INSTRUCCION]
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
            pantalla,
            teclas,
            serie,
            volcados,
            disparos,
            direccion,
        } => {
            let opciones = Opciones {
                limite,
                traza,
                pantalla,
                teclas,
                serie,
                volcados,
                disparos,
                direccion,
            };
            ejecutar(&maquina, &programa, &opciones, salida, errores)
        }
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

/// Opciones de `lorenz ejecutar`.
struct Opciones {
    limite: u64,
    traza: bool,
    pantalla: bool,
    teclas: Option<String>,
    serie: Option<String>,
    volcados: Vec<Volcado>,
    disparos: Vec<Disparo>,
    direccion: Option<u64>,
}

fn ejecutar(
    ruta_maquina: &str,
    ruta_programa: &str,
    opciones: &Opciones,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let Opciones {
        limite,
        traza,
        direccion,
        ..
    } = *opciones;
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
    if let Some(teclas) = &opciones.teclas {
        let teclado = emulador
            .maquina()
            .dispositivos
            .iter()
            .find(|d| d.tipo == TipoDispositivo::Teclado)
            .map(|d| d.nombre.clone());
        let Some(teclado) = teclado else {
            escribir(
                errores,
                "error: la maquina no tiene teclado para --teclas\n",
            );
            return SALIDA_ERROR_USO;
        };
        for byte in teclas.bytes() {
            if let Err(e) = emulador.pulsar_tecla(&teclado, u64::from(byte)) {
                escribir(errores, &format!("error: {e}\n"));
                return SALIDA_ERROR_USO;
            }
        }
    }

    if let Some(texto) = &opciones.serie {
        let uart = emulador
            .maquina()
            .dispositivos
            .iter()
            .find(|d| d.tipo == TipoDispositivo::Uart)
            .map(|d| d.nombre.clone());
        let Some(uart) = uart else {
            escribir(errores, "error: la maquina no tiene UART para --serie\n");
            return SALIDA_ERROR_USO;
        };
        if let Err(e) = emulador.recibir_en_uart(&uart, texto.as_bytes()) {
            escribir(errores, &format!("error: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    }

    let mut disparos = opciones.disparos.clone();
    disparos.sort_by_key(|d| d.instruccion);
    let mut siguiente_disparo = 0;
    let mut ejecutadas = 0u64;
    let codigo = 'ejecucion: loop {
        if emulador.estado().detenida() {
            break SALIDA_OK;
        }
        if ejecutadas >= limite {
            let e = ErrorEjecucion::LimiteDeInstrucciones { limite };
            escribir(errores, &format!("error: {e}\n"));
            break SALIDA_LIMITE;
        }
        while siguiente_disparo < disparos.len()
            && disparos[siguiente_disparo].instruccion <= ejecutadas
        {
            let linea = disparos[siguiente_disparo].linea;
            if let Err(e) = emulador.levantar_interrupcion(linea) {
                escribir(errores, &format!("error: {e}\n"));
                break 'ejecucion SALIDA_ERROR_USO;
            }
            siguiente_disparo += 1;
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
    texto_de_uarts(&mut emulador, salida);
    for volcado in &opciones.volcados {
        if let Err(codigo) = volcar(&emulador, volcado, salida, errores) {
            return codigo;
        }
    }
    if opciones.pantalla {
        dibujar_pantallas(&emulador, salida);
    }
    codigo
}

/// Vuelca un rango de memoria a la salida o a un archivo, en el formato que `lorenz ejecutar`
/// sabe volver a cargar.
fn volcar(
    emulador: &Emulador,
    volcado: &Volcado,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> Result<(), i32> {
    let cantidad = usize::try_from(volcado.cantidad).unwrap_or(usize::MAX);
    let palabras = match emulador.memoria().volcar(volcado.desde, cantidad) {
        Ok(p) => p,
        Err(e) => {
            escribir(errores, &format!("error al volcar: {e}\n"));
            return Err(SALIDA_ERROR_USO);
        }
    };
    let bits = emulador.maquina().bits_palabra;
    let texto = escribir_palabras(&palabras, bits, 16);
    match &volcado.archivo {
        Some(ruta) => {
            if let Err(e) = std::fs::write(ruta, &texto) {
                escribir(errores, &format!("{ruta}: no se pudo escribir: {e}\n"));
                return Err(SALIDA_ERROR_USO);
            }
        }
        None => {
            let digitos = (emulador.maquina().bits_direccion as usize).div_ceil(4);
            escribir(
                salida,
                &format!(
                    "volcado desde {:0digitos$x}, {} palabras:\n{texto}",
                    volcado.desde,
                    palabras.len()
                ),
            );
        }
    }
    Ok(())
}

/// Imprime lo que cada UART transmitio, si transmitio algo.
fn texto_de_uarts(emulador: &mut Emulador, salida: &mut dyn Write) {
    let nombres: Vec<String> = emulador
        .maquina()
        .dispositivos
        .iter()
        .filter(|d| d.tipo == TipoDispositivo::Uart)
        .map(|d| d.nombre.clone())
        .collect();
    for nombre in nombres {
        if let Ok(texto) = emulador.texto_de_uart(&nombre) {
            if !texto.is_empty() {
                let final_de_linea = if texto.ends_with('\n') { "" } else { "\n" };
                escribir(salida, &format!("uart {nombre}:\n{texto}{final_de_linea}"));
            }
        }
    }
}

fn dibujar_pantallas(emulador: &Emulador, salida: &mut dyn Write) {
    let nombres: Vec<String> = emulador
        .maquina()
        .dispositivos
        .iter()
        .filter(|d| matches!(d.tipo, TipoDispositivo::Pantalla { .. }))
        .map(|d| d.nombre.clone())
        .collect();
    for nombre in nombres {
        match emulador.pantalla(&nombre) {
            Ok(imagen) => escribir(salida, &format!("pantalla {nombre}:\n{}", imagen.a_texto())),
            Err(e) => escribir(salida, &format!("pantalla {nombre}: {e}\n")),
        }
    }
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
