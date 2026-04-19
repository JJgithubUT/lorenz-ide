//! `lorenz restricciones` y `lorenz variantes`: comprobar los limites de diseno y buscar
//! variantes que los cumplan (RF-CIC-033).

use std::io::Write;

use lorenz_cic::sugerir::{sugerir, trazar, Opciones};
use lorenz_cic::{
    cargar_config_archivo, comprobar, evaluar, variantes, variantes_isa, Config, Espacio, Medidas,
    Simulador, Veredicto,
};
use lorenz_emu::Emulador;

use crate::ciclos::cargar_con_lineas;
use crate::ordenes::{SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_NO_CUMPLE, SALIDA_OK};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcionesVariantes {
    pub limite: u64,
    pub direccion: u64,
    /// Incluir el bus compartido entre las cosas que se prueban.
    pub bus: bool,
    /// Cuantas variantes se muestran como mucho.
    pub cuantas: usize,
}

impl Default for OpcionesVariantes {
    fn default() -> Self {
        Self {
            limite: crate::args::LIMITE_POR_OMISION,
            direccion: 0,
            bus: false,
            cuantas: 10,
        }
    }
}

fn escribir(w: &mut dyn Write, t: &str) {
    let _ = w.write_all(t.as_bytes());
}

/// Todo lo necesario para simular el programa cuantas veces haga falta.
struct Preparado {
    maquina: lorenz_arch::modelo::Maquina,
    config: Config,
    segmentos: Vec<(u64, Vec<u64>)>,
    palabras: u64,
}

impl Preparado {
    fn nuevo(ruta_maquina: &str, ruta_programa: &str, direccion: u64) -> Result<Self, String> {
        let maquina = lorenz_arch::cargar_archivo(ruta_maquina)
            .map_err(|e| format!("{ruta_maquina}: {e}"))?;
        let config =
            cargar_config_archivo(ruta_maquina).map_err(|e| format!("{ruta_maquina}: {e}"))?;
        let (segmentos, _, _) = cargar_con_lineas(&maquina, ruta_programa, direccion)?;
        let palabras = segmentos.iter().map(|(_, p)| p.len() as u64).sum();
        Ok(Self {
            maquina,
            config,
            segmentos,
            palabras,
        })
    }

    fn emulador(&self) -> Result<Emulador, String> {
        let mut e = Emulador::nuevo(self.maquina.clone()).map_err(|e| e.to_string())?;
        for (dir, p) in &self.segmentos {
            e.cargar_programa(*dir, p).map_err(|e| e.to_string())?;
        }
        Ok(e)
    }
}

fn formato(v: Option<f64>) -> String {
    match v {
        None => "-".into(),
        Some(x) if x.fract() == 0.0 => format!("{x:.0}"),
        Some(x) => format!("{x:.3}"),
    }
}

fn limites(v: &Veredicto) -> String {
    let r = &v.restriccion;
    match (r.minimo, r.maximo) {
        (Some(a), Some(b)) => format!("{} a {}", formato(Some(a)), formato(Some(b))),
        (Some(a), None) => format!(">= {}", formato(Some(a))),
        (None, Some(b)) => format!("<= {}", formato(Some(b))),
        (None, None) => String::new(),
    }
}

fn escribir_veredictos(salida: &mut dyn Write, v: &[Veredicto]) {
    escribir(
        salida,
        &format!(
            "{:<24} {:<18} {:>12} {:>14}  {}\n",
            "restriccion", "metrica", "medido", "limite", "estado"
        ),
    );
    for x in v {
        escribir(
            salida,
            &format!(
                "{:<24} {:<18} {:>12} {:>14}  {}\n",
                x.restriccion.nombre,
                x.restriccion.metrica.nombre(),
                formato(x.valor),
                limites(x),
                if x.cumple { "cumple" } else { "NO CUMPLE" }
            ),
        );
    }
}

/// `lorenz restricciones`: simula el programa y dice si se cumple cada restriccion.
pub fn restricciones_cli(
    ruta_maquina: &str,
    ruta_programa: &str,
    opciones: &OpcionesVariantes,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let prep = match Preparado::nuevo(ruta_maquina, ruta_programa, opciones.direccion) {
        Ok(p) => p,
        Err(e) => {
            escribir(errores, &format!("{e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    if prep.config.restricciones.is_empty() {
        escribir(
            errores,
            &format!("{ruta_maquina}: no declara ninguna [[restriccion]]\n"),
        );
        return SALIDA_ERROR_USO;
    }
    let emu = match prep.emulador() {
        Ok(e) => e,
        Err(e) => {
            escribir(errores, &format!("{ruta_programa}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let mut sim = match Simulador::nuevo(emu, prep.config.clone()) {
        Ok(s) => s,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    if let Err(e) = sim.ejecutar(opciones.limite) {
        escribir(errores, &format!("fallo durante la ejecucion: {e}\n"));
        return crate::ordenes::SALIDA_ERROR_EJECUCION;
    }
    if !sim.detenida() {
        escribir(
            salida,
            &format!(
                "el programa no termino en {} instrucciones: no se puede comprobar\n",
                opciones.limite
            ),
        );
        return SALIDA_LIMITE;
    }
    let medidas = Medidas::new(sim.metricas(), prep.palabras);
    let v = comprobar(&prep.config.restricciones, &medidas);
    escribir_veredictos(salida, &v);
    let fallan = v.iter().filter(|x| !x.cumple).count();
    if fallan == 0 {
        escribir(salida, "se cumplen todas las restricciones\n");
        SALIDA_OK
    } else {
        escribir(
            salida,
            &format!(
                "no se cumple{} {fallan} restriccion{}\n",
                if fallan == 1 { "" } else { "n" },
                if fallan == 1 { "" } else { "es" }
            ),
        );
        SALIDA_NO_CUMPLE
    }
}

/// `lorenz variantes`: prueba variantes de la temporizacion y del repertorio y dice cuales
/// cumplen las restricciones.
pub fn variantes_cli(
    ruta_maquina: &str,
    ruta_programa: &str,
    opciones: &OpcionesVariantes,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let prep = match Preparado::nuevo(ruta_maquina, ruta_programa, opciones.direccion) {
        Ok(p) => p,
        Err(e) => {
            escribir(errores, &format!("{e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    if prep.config.restricciones.is_empty() {
        escribir(
            errores,
            &format!("{ruta_maquina}: no declara ninguna [[restriccion]]\n"),
        );
        return SALIDA_ERROR_USO;
    }
    let espacio = Espacio {
        bus: opciones.bus,
        ..Espacio::default()
    };
    let vs = variantes(&prep.config, &espacio);
    let resultados = match evaluar(
        &vs,
        &prep.config.restricciones,
        prep.palabras,
        opciones.limite,
        &mut || prep.emulador(),
    ) {
        Ok(r) => r,
        Err(e) => {
            escribir(errores, &format!("{e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let cumplen = resultados.iter().filter(|r| r.cumple).count();
    escribir(
        salida,
        &format!(
            "{} variante{} probada{}, {cumplen} cumple{} las restricciones\n",
            resultados.len(),
            if resultados.len() == 1 { "" } else { "s" },
            if resultados.len() == 1 { "" } else { "s" },
            if cumplen == 1 { "" } else { "n" },
        ),
    );
    escribir(
        salida,
        &format!(
            "{:<5} {:>8} {:>8} {:>7}  {}\n",
            "", "ciclos", "palabras", "CPI", "variante"
        ),
    );
    for r in resultados.iter().take(opciones.cuantas) {
        escribir(
            salida,
            &format!(
                "{:<5} {:>8} {:>8} {:>7}  {}\n",
                if r.cumple { "si" } else { "no" },
                r.medidas.metricas.ciclos,
                r.medidas.palabras_codigo,
                formato(r.medidas.metricas.cpi()),
                r.nombre
            ),
        );
    }
    if resultados.len() > opciones.cuantas {
        escribir(
            salida,
            &format!("(y {} mas)\n", resultados.len() - opciones.cuantas),
        );
    }
    // Variantes del repertorio, sobre el diseno tal como esta.
    if let Some(base) = resultados.iter().find(|r| r.nombre == "base") {
        if let Ok(mut emu) = prep.emulador() {
            if let Ok(traza) = trazar(&mut emu, opciones.limite) {
                let s = sugerir(&[traza], &Opciones::default());
                let isa = variantes_isa(&base.medidas, &s, &prep.config.restricciones);
                if !isa.is_empty() {
                    escribir(
                        salida,
                        "\ninstrucciones nuevas (estimacion sobre la base):\n",
                    );
                    escribir(
                        salida,
                        &format!(
                            "{:<5} {:>8} {:>8}  {}\n",
                            "", "ciclos", "palabras", "instruccion"
                        ),
                    );
                    for x in isa.iter().take(opciones.cuantas) {
                        escribir(
                            salida,
                            &format!(
                                "{:<5} {:>8} {:>8}  {}\n",
                                if x.cumple { "si" } else { "no" },
                                x.medidas.metricas.ciclos,
                                x.medidas.palabras_codigo,
                                x.instruccion
                            ),
                        );
                    }
                }
            }
        }
    }
    if cumplen > 0 {
        SALIDA_OK
    } else {
        escribir(salida, "ninguna variante cumple las restricciones\n");
        SALIDA_NO_CUMPLE
    }
}
