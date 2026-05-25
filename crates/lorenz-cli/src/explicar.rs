//! La orden `explicar`: cuenta en palabras lo que hace un programa, instruccion a instruccion
//! (RF-ECO-044).
//!
//! No usa ningun modelo de lenguaje: la explicacion sale de la descripcion de la maquina (el texto
//! de cada instruccion) y de lo que la ejecucion cambia de verdad, asi que es siempre la misma y
//! siempre cierta para ese programa.

use std::io::Write;

use lorenz_emu::Emulador;

use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};

/// Instrucciones que se explican si no se pide otra cantidad.
pub const LIMITE_POR_OMISION: u64 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpcionesExplicar {
    pub limite: u64,
    pub direccion: Option<u64>,
}

impl Default for OpcionesExplicar {
    fn default() -> Self {
        Self {
            limite: LIMITE_POR_OMISION,
            direccion: None,
        }
    }
}

fn escribir(w: &mut dyn Write, t: &str) {
    let _ = w.write_all(t.as_bytes());
}

fn valores(e: &Emulador) -> Vec<(String, u64)> {
    let estado = e.estado();
    estado
        .nombres()
        .iter()
        .map(|n| (n.clone(), estado.registro(n).unwrap_or(0)))
        .collect()
}

fn banderas(e: &Emulador) -> Vec<(String, bool)> {
    let estado = e.estado();
    estado
        .nombres_de_banderas()
        .iter()
        .map(|n| ((*n).to_string(), estado.bandera(n).unwrap_or(false)))
        .collect()
}

pub fn explicar_cli(
    ruta_maquina: &str,
    ruta_programa: &str,
    opciones: &OpcionesExplicar,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let fallo = |errores: &mut dyn Write, m: String| {
        escribir(errores, &format!("{m}\n"));
        SALIDA_ERROR_USO
    };
    let maquina = match lorenz_arch::cargar_archivo(ruta_maquina) {
        Ok(m) => m,
        Err(e) => return fallo(errores, format!("{ruta_maquina}: {e}")),
    };
    let inicio = opciones.direccion.unwrap_or(maquina.arranque);
    let segmentos = match crate::ensamblar::cargar_segmentos(&maquina, ruta_programa, inicio) {
        Ok(s) => s,
        Err(e) => return fallo(errores, e),
    };
    let maquina_para_desensamblar = maquina.clone();
    let desensamblador =
        match lorenz_ens::desensamblador::Desensamblador::nuevo(&maquina_para_desensamblar) {
            Ok(d) => d,
            Err(e) => return fallo(errores, format!("{ruta_maquina}: {e}")),
        };
    let digitos = (maquina.bits_direccion as usize).div_ceil(4);
    let nombre_pc = maquina
        .registro_con_rol(lorenz_arch::RolRegistro::ContadorPrograma)
        .map(|r| r.nombre.clone())
        .unwrap_or_default();
    let descripciones: Vec<(String, String)> = maquina
        .instrucciones
        .iter()
        .map(|i| (i.descripcion.clone(), i.semantica.clone()))
        .collect();
    let mut emulador = match Emulador::nuevo(maquina) {
        Ok(e) => e,
        Err(e) => return fallo(errores, format!("{ruta_maquina}: {e}")),
    };
    if let Err(e) = lorenz_emu::programa::precargar_regiones(
        &mut emulador,
        &crate::ordenes::carpeta_de(ruta_maquina),
    ) {
        return fallo(errores, format!("{ruta_maquina}: {e}"));
    }
    for (desde, palabras) in &segmentos {
        if let Err(e) = emulador.cargar_programa(*desde, palabras) {
            return fallo(errores, format!("{ruta_programa}: {e}"));
        }
    }
    emulador.estado_mut().fijar_pc(inicio);
    emulador.memoria_mut().anotar_trafico(true);

    escribir(
        salida,
        &format!(
            "Explicacion de {ruta_programa} en {}\n",
            emulador.maquina().nombre
        ),
    );
    escribir(
        salida,
        &format!("{}\n\n", lorenz_arch::marca_generado("lorenz explicar")),
    );

    let mut n = 0u64;
    let mut codigo = SALIDA_OK;
    while !emulador.estado().detenida() && n < opciones.limite {
        let pc = emulador.estado().pc();
        let palabras = emulador.memoria().volcar(pc, 4).unwrap_or_default();
        let lineas = desensamblador.desensamblar(pc, &palabras, None);
        let (texto, indice) = match lineas.first() {
            Some(lorenz_ens::desensamblador::Linea::Instruccion { texto, indice, .. }) => {
                (texto.clone(), Some(*indice))
            }
            _ => ("(palabra que no es una instruccion)".to_string(), None),
        };
        let antes = valores(&emulador);
        let banderas_antes = banderas(&emulador);
        let paso = match emulador.paso() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(e) => {
                escribir(errores, &format!("error en {pc:0digitos$x}: {e}\n"));
                codigo = SALIDA_ERROR_EJECUCION;
                break;
            }
        };
        n += 1;
        escribir(salida, &format!("{n}. {pc:0digitos$x}  {texto}\n"));
        if let Some((d, _)) = indice.and_then(|i| descripciones.get(i)) {
            if !d.is_empty() {
                escribir(salida, &format!("   {d}\n"));
            }
        }
        let trafico = emulador.memoria_mut().tomar_trafico();
        let despues = valores(&emulador);
        let mut cambio = false;
        for ((nombre, a), (_, d)) in antes.iter().zip(&despues) {
            if a != d && *nombre != nombre_pc {
                cambio = true;
                escribir(salida, &format!("   {nombre} pasa de {a:#x} a {d:#x}.\n"));
            }
        }
        for ((nombre, a), (_, d)) in banderas_antes.iter().zip(banderas(&emulador)) {
            if *a != d {
                cambio = true;
                escribir(
                    salida,
                    &format!(
                        "   La bandera {nombre} pasa de {} a {}.\n",
                        u8::from(*a),
                        u8::from(d)
                    ),
                );
            }
        }
        if !cambio && trafico.is_empty() && !paso.salto && !paso.detenida {
            escribir(salida, "   No cambia ningun registro ni la memoria.\n");
        }
        for t in trafico {
            let v = emulador
                .memoria()
                .leer_privilegiado(t.direccion)
                .unwrap_or(0);
            if t.escritura {
                escribir(
                    salida,
                    &format!("   Escribe {v:#x} en la direccion {:#x}.\n", t.direccion),
                );
            } else {
                escribir(
                    salida,
                    &format!("   Lee {v:#x} de la direccion {:#x}.\n", t.direccion),
                );
            }
        }
        if paso.detenida {
            escribir(salida, "   La maquina se detiene.\n");
        } else if paso.salto {
            escribir(
                salida,
                &format!(
                    "   Salta: el contador de programa pasa a {:0digitos$x}.\n",
                    emulador.estado().pc()
                ),
            );
        }
    }
    let e = emulador.estado();
    if e.detenida() {
        escribir(
            salida,
            &format!(
                "\nLa maquina se detuvo tras {} instrucciones y {} ciclos.\n",
                e.instrucciones(),
                e.ciclos()
            ),
        );
    } else if codigo == SALIDA_OK {
        escribir(
            salida,
            &format!("\nSe explican las primeras {n} instrucciones y la maquina sigue en marcha; usa --limite para ver mas.\n"),
        );
    }
    codigo
}
