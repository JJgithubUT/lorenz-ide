//! `lorenz nucleos`: ejecutar un programa por nucleo sobre memoria compartida (RF-CIC-023).

use std::io::Write;

use lorenz_cic::{cargar_config_archivo, Sistema};
use lorenz_emu::Emulador;

use crate::ciclos::cargar_con_lineas;
use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_OK};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcionesNucleos {
    pub limite: u64,
    pub direccion: u64,
}

impl Default for OpcionesNucleos {
    fn default() -> Self {
        Self {
            limite: crate::args::LIMITE_POR_OMISION,
            direccion: 0,
        }
    }
}

fn escribir(w: &mut dyn Write, t: &str) {
    let _ = w.write_all(t.as_bytes());
}

pub fn nucleos_cli(
    ruta_maquina: &str,
    programas: &[String],
    opciones: &OpcionesNucleos,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let uso = |errores: &mut dyn Write, m: String| {
        escribir(errores, &format!("{m}\n"));
        SALIDA_ERROR_USO
    };
    let maquina = match lorenz_arch::cargar_archivo(ruta_maquina) {
        Ok(m) => m,
        Err(e) => return uso(errores, format!("{ruta_maquina}: {e}")),
    };
    let config = match cargar_config_archivo(ruta_maquina) {
        Ok(c) => c,
        Err(e) => return uso(errores, format!("{ruta_maquina}: {e}")),
    };
    let n = config.nucleos.as_ref().map_or(1, |n| n.cantidad);
    if programas.len() != 1 && programas.len() != n {
        return uso(
            errores,
            format!(
                "la maquina tiene {n} nucleo{}: da un programa para todos o uno por nucleo, no {}",
                if n == 1 { "" } else { "s" },
                programas.len()
            ),
        );
    }
    let mut emuladores = Vec::new();
    for i in 0..n {
        let ruta = &programas[if programas.len() == 1 { 0 } else { i }];
        let segmentos = match cargar_con_lineas(&maquina, ruta, opciones.direccion) {
            Ok((s, _, _)) => s,
            Err(e) => return uso(errores, e),
        };
        let mut emu = match Emulador::nuevo(maquina.clone()) {
            Ok(e) => e,
            Err(e) => return uso(errores, format!("{ruta_maquina}: {e}")),
        };
        for (dir, p) in &segmentos {
            if let Err(e) = emu.cargar_programa(*dir, p) {
                return uso(errores, format!("{ruta}: {e}"));
            }
        }
        emuladores.push(emu);
    }
    let mut sistema = match Sistema::nuevo(emuladores, config) {
        Ok(s) => s,
        Err(e) => return uso(errores, format!("{ruta_maquina}: {e}")),
    };
    if let Err(e) = sistema.ejecutar(opciones.limite) {
        escribir(errores, &format!("fallo durante la ejecucion: {e}\n"));
        return SALIDA_ERROR_EJECUCION;
    }
    let m = sistema.metricas();
    escribir(
        salida,
        &format!("{n} nucleo{}\n", if n == 1 { "" } else { "s" }),
    );
    escribir(
        salida,
        &format!(
            "{:<8} {:>8} {:>13} {:>7} {:>13}\n",
            "nucleo", "ciclos", "instrucciones", "CPI", "esperas bus"
        ),
    );
    for (i, x) in m.nucleos.iter().enumerate() {
        escribir(
            salida,
            &format!(
                "{:<8} {:>8} {:>13} {:>7} {:>13}\n",
                i,
                x.ciclos,
                x.instrucciones,
                x.cpi().map_or("-".to_string(), |c| format!("{c:.3}")),
                x.esperas_bus
            ),
        );
    }
    escribir(
        salida,
        &format!(
            "total: {} ciclos, {} instrucciones\n",
            m.ciclos, m.instrucciones
        ),
    );
    escribir(
        salida,
        &format!("palabras compartidas copiadas: {}\n", m.propagaciones),
    );
    if m.ciclos_de_bus_comun > 0 {
        escribir(
            salida,
            &format!("bus comun ocupado: {} ciclos\n", m.ciclos_de_bus_comun),
        );
    }
    if sistema.detenido() {
        SALIDA_OK
    } else {
        escribir(
            salida,
            &format!("limite de {} instrucciones alcanzado\n", opciones.limite),
        );
        SALIDA_LIMITE
    }
}
