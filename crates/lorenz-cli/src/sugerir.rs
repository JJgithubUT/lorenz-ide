//! `lorenz sugerir`: instrucciones nuevas que acortarian los programas (RF-CIC-034).

use std::io::Write;

use lorenz_cic::sugerir::{sugerir, trazar, Opciones};
use lorenz_emu::Emulador;

use crate::ordenes::{SALIDA_ERROR_USO, SALIDA_OK};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcionesSugerir {
    pub limite: u64,
    pub direccion: u64,
    pub minima: usize,
    pub maxima: usize,
    pub cuantas: usize,
}

impl Default for OpcionesSugerir {
    fn default() -> Self {
        let o = Opciones::default();
        Self {
            limite: crate::args::LIMITE_POR_OMISION,
            direccion: 0,
            minima: o.minima,
            maxima: o.maxima,
            cuantas: o.cuantas,
        }
    }
}

fn escribir(w: &mut dyn Write, t: &str) {
    let _ = w.write_all(t.as_bytes());
}

pub fn sugerir_cli(
    ruta_maquina: &str,
    programas: &[String],
    opciones: &OpcionesSugerir,
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
    if opciones.minima < 2 || opciones.maxima < opciones.minima {
        escribir(
            errores,
            "--minima debe ser al menos 2 y --maxima no menor que --minima\n",
        );
        return SALIDA_ERROR_USO;
    }
    let mut ejecuciones = Vec::new();
    for ruta in programas {
        let segmentos = match crate::ciclos::cargar_con_lineas(&maquina, ruta, opciones.direccion) {
            Ok((s, _, _)) => s,
            Err(e) => {
                escribir(errores, &format!("{e}\n"));
                return SALIDA_ERROR_USO;
            }
        };
        let mut emu = match Emulador::nuevo(maquina.clone()) {
            Ok(e) => e,
            Err(e) => {
                escribir(errores, &format!("{ruta_maquina}: {e}\n"));
                return SALIDA_ERROR_USO;
            }
        };
        for (dir, p) in &segmentos {
            if let Err(e) = emu.cargar_programa(*dir, p) {
                escribir(errores, &format!("{ruta}: {e}\n"));
                return SALIDA_ERROR_USO;
            }
        }
        match trazar(&mut emu, opciones.limite) {
            Ok(t) => ejecuciones.push(t),
            Err(e) => {
                escribir(
                    errores,
                    &format!("{ruta}: fallo durante la ejecucion: {e}\n"),
                );
                return SALIDA_ERROR_USO;
            }
        }
    }
    let instrucciones: usize = ejecuciones.iter().map(Vec::len).sum();
    let s = sugerir(
        &ejecuciones,
        &Opciones {
            minima: opciones.minima,
            maxima: opciones.maxima,
            cuantas: opciones.cuantas,
        },
    );
    escribir(
        salida,
        &format!(
            "{} programa{} analizado{}, {instrucciones} instrucciones ejecutadas\n",
            programas.len(),
            if programas.len() == 1 { "" } else { "s" },
            if programas.len() == 1 { "" } else { "s" },
        ),
    );
    if s.is_empty() {
        escribir(salida, "no hay secuencias que valga la pena fusionar\n");
        return SALIDA_OK;
    }
    escribir(
        salida,
        &format!(
            "{:<24} {:>10} {:>12} {:>10} {:>10}\n",
            "secuencia", "apariciones", "ejecuciones", "palabras", "ciclos"
        ),
    );
    for x in &s {
        escribir(
            salida,
            &format!(
                "{:<24} {:>10} {:>12} {:>10} {:>10}\n",
                x.nombre(),
                x.apariciones,
                x.ejecuciones,
                x.palabras_ahorradas,
                x.ciclos_ahorrados
            ),
        );
    }
    escribir(
        salida,
        "(palabras y ciclos que se ahorrarian si la secuencia fuera una instruccion del tamano y la\nduracion de la mas larga; es una estimacion)\n",
    );
    SALIDA_OK
}
