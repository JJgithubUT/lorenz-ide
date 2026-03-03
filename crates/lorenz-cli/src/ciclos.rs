//! La orden `ciclos`: ejecuta un programa con la temporizacion de la descripcion (pipeline,
//! prediccion de saltos, cache y latencias) y cuenta ciclos (RF-CIC-001, RF-CIC-002,
//! RF-CIC-012, RF-CIC-025, RF-CIC-026, RF-CIC-027, RF-CIC-031).

use std::io::Write;

use lorenz_cic::{cargar_config, Metricas, Simulador};
use lorenz_emu::Emulador;

use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_OK};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcionesCiclos {
    pub limite: u64,
    pub direccion: u64,
    /// Mostrar el pipeline de los primeros N ciclos.
    pub vista: Option<u64>,
    /// Dibujar el diagrama de tiempos de las primeras N instrucciones.
    pub diagrama: Option<usize>,
    pub perfil: bool,
    pub volcar_cache: bool,
}

impl Default for OpcionesCiclos {
    fn default() -> Self {
        Self {
            limite: crate::args::LIMITE_POR_OMISION,
            direccion: 0,
            vista: None,
            diagrama: None,
            perfil: false,
            volcar_cache: false,
        }
    }
}

fn escribir(w: &mut dyn Write, t: &str) {
    let _ = w.write_all(t.as_bytes());
}

fn porcentaje(x: Option<f64>) -> String {
    x.map_or("-".to_string(), |v| format!("{:.1} %", v * 100.0))
}

/// Las metricas como texto, una por linea.
pub fn texto_de_metricas(m: &Metricas, bytes_de_codigo: Option<u64>) -> String {
    let mut s = String::new();
    s.push_str(&format!("ciclos: {}\n", m.ciclos));
    s.push_str(&format!("instrucciones: {}\n", m.instrucciones));
    s.push_str(&format!(
        "ciclos por instruccion: {}\n",
        m.cpi().map_or("-".to_string(), |c| format!("{c:.3}"))
    ));
    s.push_str(&format!(
        "esperas: {} por datos, {} estructurales, {} por saltos\n",
        m.esperas_datos, m.esperas_estructurales, m.esperas_control
    ));
    if m.saltos > 0 {
        s.push_str(&format!(
            "saltos: {} ({} acertados, {} fallados, {} instrucciones descartadas)\n",
            m.saltos,
            m.saltos_acertados,
            m.saltos_fallados(),
            m.descartadas
        ));
    }
    if let Some(c) = &m.cache {
        s.push_str(&format!(
            "cache: lecturas {} aciertos / {} fallos, escrituras {} aciertos / {} fallos, tasa {}\n",
            c.aciertos_lectura,
            c.fallos_lectura,
            c.aciertos_escritura,
            c.fallos_escritura,
            porcentaje(c.tasa_de_aciertos())
        ));
    }
    s.push_str(&format!("uso del bus: {}\n", porcentaje(m.uso_del_bus())));
    if let Some(b) = bytes_de_codigo {
        s.push_str(&format!("tamano del codigo: {b} bytes\n"));
    }
    s
}

pub fn ciclos_cli(
    ruta_maquina: &str,
    ruta_programa: &str,
    opciones: &OpcionesCiclos,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let texto = match std::fs::read_to_string(ruta_maquina) {
        Ok(t) => t,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: no se pudo leer: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let maquina = match lorenz_arch::cargar(&texto) {
        Ok(m) => m,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let config = match cargar_config(&texto) {
        Ok(c) => c,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let segmentos =
        match crate::ensamblar::cargar_segmentos(&maquina, ruta_programa, opciones.direccion) {
            Ok(s) => s,
            Err(e) => {
                escribir(errores, &format!("{e}\n"));
                return SALIDA_ERROR_USO;
            }
        };
    let bits_palabra = u64::from(maquina.bits_palabra);
    let palabras: u64 = segmentos.iter().map(|(_, p)| p.len() as u64).sum();
    let mut emu = match Emulador::nuevo(maquina) {
        Ok(e) => e,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    for (dir, p) in &segmentos {
        if let Err(e) = emu.cargar_programa(*dir, p) {
            escribir(errores, &format!("{ruta_programa}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    }
    let mut sim = match Simulador::nuevo(emu, config) {
        Ok(s) => s,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let ejecutadas = match sim.ejecutar(opciones.limite) {
        Ok(n) => n,
        Err(e) => {
            escribir(errores, &format!("fallo durante la ejecucion: {e}\n"));
            return SALIDA_ERROR_EJECUCION;
        }
    };
    if let Some(n) = opciones.vista {
        for c in 0..n.min(sim.ciclos()) {
            match sim.vista(c) {
                Ok(v) => escribir(salida, &format!("{}\n", v.texto())),
                Err(e) => {
                    escribir(errores, &format!("{e}\n"));
                    return SALIDA_ERROR_EJECUCION;
                }
            }
        }
    }
    if let Some(n) = opciones.diagrama {
        escribir(salida, &sim.diagrama(0, n));
    }
    escribir(
        salida,
        &texto_de_metricas(&sim.metricas(), Some((palabras * bits_palabra).div_ceil(8))),
    );
    if opciones.perfil {
        escribir(salida, "# direccion instruccion ejecuciones ciclos\n");
        let mut filas = sim.perfil();
        filas.sort_by_key(|f| std::cmp::Reverse(f.ciclos));
        for f in filas {
            escribir(
                salida,
                &format!(
                    "{:#06x} {} {} {}\n",
                    f.direccion, f.mnemonico, f.ejecuciones, f.ciclos
                ),
            );
        }
    }
    if opciones.volcar_cache {
        match sim.cache() {
            Some(c) => escribir(salida, &c.volcar()),
            None => escribir(salida, "la descripcion no declara cache\n"),
        }
    }
    if sim.detenida() {
        SALIDA_OK
    } else {
        escribir(
            salida,
            &format!("limite de {ejecutadas} instrucciones alcanzado\n"),
        );
        SALIDA_LIMITE
    }
}
