//! `lorenz cotejar`: el mismo programa SGT en varias maquinas, con tamano y ciclos lado a lado
//! (RF-SGT-039, RF-SGT-040).

use std::collections::BTreeMap;
use std::io::Write;

use lorenz_emu::Emulador;
use lorenz_sgt::analisis::Opciones;
use lorenz_sgt::compilar::compilar;

use crate::ordenes::{SALIDA_DIFIEREN, SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpcionesCotejar {
    pub nivel: Option<u8>,
    pub limite: Option<u64>,
}

/// Lo que dio el programa en una maquina.
struct Fila {
    maquina: String,
    bits_palabra: u32,
    resultado: Result<Medida, String>,
}

struct Medida {
    palabras: usize,
    instrucciones: u64,
    ciclos: u64,
    /// Valor final de cada variable global escalar, por nombre.
    globales: BTreeMap<String, u64>,
}

fn medir(ruta_maquina: &str, ruta_fuente: &str, texto: &str, opciones: &OpcionesCotejar) -> Fila {
    let maquina = match lorenz_arch::cargar_archivo(ruta_maquina) {
        Ok(m) => m,
        Err(e) => {
            return Fila {
                maquina: ruta_maquina.to_string(),
                bits_palabra: 0,
                resultado: Err(e.to_string()),
            }
        }
    };
    let nombre = maquina.nombre.clone();
    let bits = maquina.bits_palabra;
    let resultado = (|| -> Result<Medida, String> {
        let mut o = Opciones::default();
        if let Some(n) = opciones.nivel {
            o.nivel = n;
        }
        let c = compilar(ruta_fuente, texto, &maquina, o).map_err(|e| {
            e.iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        })?;
        let palabras = c.palabras();
        let mut emu = Emulador::nuevo(maquina.clone()).map_err(|e| e.to_string())?;
        for (dir, w) in &palabras {
            emu.cargar_programa(*dir, &[*w])
                .map_err(|e| e.to_string())?;
        }
        let r = emu
            .ejecutar(opciones.limite.unwrap_or(crate::args::LIMITE_POR_OMISION))
            .map_err(|e| format!("al ejecutar: {e}"))?;
        let mut globales = BTreeMap::new();
        for (i, g) in c.modulo.globales.iter().enumerate() {
            if g.tipo.es_escalar() && !g.nombre.starts_with("__") {
                let a = c.disposicion.globales[i];
                if let Ok(v) = emu.memoria().leer_privilegiado(a) {
                    globales.insert(g.nombre.clone(), v);
                }
            }
        }
        Ok(Medida {
            palabras: palabras.len(),
            instrucciones: r.instrucciones,
            ciclos: r.ciclos,
            globales,
        })
    })();
    Fila {
        maquina: nombre,
        bits_palabra: bits,
        resultado,
    }
}

pub fn cotejar_cli(
    ruta_fuente: &str,
    maquinas: &[String],
    opciones: &OpcionesCotejar,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let texto = match std::fs::read_to_string(ruta_fuente) {
        Ok(t) => t,
        Err(e) => {
            let _ = writeln!(errores, "{ruta_fuente}: no se pudo leer: {e}");
            return SALIDA_ERROR_USO;
        }
    };
    let filas: Vec<Fila> = maquinas
        .iter()
        .map(|m| medir(m, ruta_fuente, &texto, opciones))
        .collect();
    // Tabla de tamano y ciclos.
    let ancho = filas
        .iter()
        .map(|f| f.maquina.len())
        .max()
        .unwrap_or(0)
        .max(7);
    let _ = writeln!(
        salida,
        "{:<ancho$}  {:>7}  {:>12}  {:>13}  {:>9}",
        "maquina", "palabra", "codigo(bits)", "instrucciones", "ciclos"
    );
    let mut fallo = false;
    for f in &filas {
        match &f.resultado {
            Ok(m) => {
                let _ = writeln!(
                    salida,
                    "{:<ancho$}  {:>7}  {:>12}  {:>13}  {:>9}",
                    f.maquina,
                    f.bits_palabra,
                    m.palabras as u64 * u64::from(f.bits_palabra),
                    m.instrucciones,
                    m.ciclos
                );
            }
            Err(e) => {
                fallo = true;
                let _ = writeln!(
                    salida,
                    "{:<ancho$}  no se pudo: {}",
                    f.maquina,
                    e.lines().next().unwrap_or("")
                );
                let _ = writeln!(errores, "{}: {e}", f.maquina);
            }
        }
    }
    // Los resultados deben coincidir en el ancho de la palabra mas estrecha.
    let buenas: Vec<(&Fila, &Medida)> = filas
        .iter()
        .filter_map(|f| f.resultado.as_ref().ok().map(|m| (f, m)))
        .collect();
    let mut difieren = false;
    if buenas.len() > 1 {
        let ancho_comun = buenas
            .iter()
            .map(|(f, _)| f.bits_palabra)
            .min()
            .unwrap_or(0);
        let mascara = if ancho_comun >= 64 {
            u64::MAX
        } else {
            (1u64 << ancho_comun) - 1
        };
        let (primera, ref_m) = buenas[0];
        for (f, m) in &buenas[1..] {
            for (k, v) in &ref_m.globales {
                match m.globales.get(k) {
                    Some(w) if w & mascara == v & mascara => {}
                    otro => {
                        difieren = true;
                        let _ = writeln!(
                            errores,
                            "`{k}` difiere: {} da {}, {} da {}",
                            primera.maquina,
                            v & mascara,
                            f.maquina,
                            otro.map_or("nada".to_string(), |w| (w & mascara).to_string())
                        );
                    }
                }
            }
        }
        let _ = writeln!(
            salida,
            "{}",
            if difieren {
                "los resultados DIFIEREN entre maquinas"
            } else {
                "los resultados coinciden en todas las maquinas"
            }
        );
    }
    if fallo {
        SALIDA_ERROR_EJECUCION
    } else if difieren {
        SALIDA_DIFIEREN
    } else {
        SALIDA_OK
    }
}
