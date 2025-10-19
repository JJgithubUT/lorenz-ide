//! Ejecutar todas las pruebas de un proyecto, o un subconjunto, y resumir el resultado
//! (RF-TST-011, RF-TST-012, RF-TST-015, RF-TST-031, RF-TST-032).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::cobertura::{Cobertura, InformeCobertura};
use crate::ejecutor::{ejecutar, ejecutar_con_cobertura, Resultado, Veredicto};
use crate::formato::Proyecto;
use crate::regresion::InformeRegresion;

pub const FORMATO_RESULTADOS: &str = "lorenz-resultados-0.1";

/// Todas las pruebas pasaron.
pub const SALIDA_OK: i32 = 0;
/// Alguna prueba fallo.
pub const SALIDA_FALLAN: i32 = 1;
/// Alguna prueba no se pudo preparar: el programa no ensambla, falta un archivo...
pub const SALIDA_ERROR_EN_PRUEBAS: i32 = 5;

#[derive(Debug, Clone, Default)]
pub struct Opciones {
    /// Solo las pruebas cuyo nombre contiene este texto.
    pub nombre: Option<String>,
    /// Solo las pruebas con esta etiqueta.
    pub etiqueta: Option<String>,
    /// Cuantas pruebas a la vez; 0 elige segun los procesadores disponibles.
    pub hilos: usize,
    /// Medir la cobertura de instrucciones y ramas de todas las pruebas (RF-TST-021).
    pub cobertura: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resumen {
    pub total: usize,
    pub pasan: usize,
    pub fallan: usize,
    pub errores: usize,
    /// Pruebas del archivo que el filtro dejo fuera.
    pub omitidas: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Informe {
    pub formato: String,
    pub proyecto: String,
    pub maquina: String,
    pub semilla: u64,
    pub resumen: Resumen,
    pub milisegundos: u64,
    pub pruebas: Vec<Resultado>,
    /// Solo si se pidio medirla.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cobertura: Option<InformeCobertura>,
    /// Solo si se comparo con una ejecucion anterior (RF-TST-027, RF-TST-028).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regresion: Option<InformeRegresion>,
}

impl Informe {
    /// 0 si todo pasa; 5 si alguna no se pudo preparar; 1 si alguna fallo.
    pub fn codigo_de_salida(&self) -> i32 {
        if self.resumen.errores > 0 {
            SALIDA_ERROR_EN_PRUEBAS
        } else if self.resumen.fallan > 0 {
            SALIDA_FALLAN
        } else {
            SALIDA_OK
        }
    }

    pub fn a_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("el informe siempre se serializa");
        s.push('\n');
        s
    }

    pub fn desde_json(texto: &str) -> Result<Informe, String> {
        let i: Informe =
            serde_json::from_str(texto).map_err(|e| format!("resultados invalidos: {e}"))?;
        if i.formato != FORMATO_RESULTADOS {
            return Err(format!(
                "formato de resultados `{}` no reconocido; esta version lee `{FORMATO_RESULTADOS}`",
                i.formato
            ));
        }
        Ok(i)
    }

    /// Un resumen legible: una linea por prueba y el detalle de los fallos.
    pub fn a_texto(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        for r in &self.pruebas {
            s.push_str(&texto_de(r));
        }
        let r = &self.resumen;
        let _ = write!(
            s,
            "\n{} pruebas: {} pasan, {} fallan, {} con error",
            r.total, r.pasan, r.fallan, r.errores
        );
        if r.omitidas > 0 {
            let _ = write!(s, ", {} omitidas", r.omitidas);
        }
        let _ = writeln!(s, " ({} ms)", self.milisegundos);
        if let Some(c) = &self.cobertura {
            s.push('\n');
            s.push_str(&c.a_texto());
        }
        if let Some(r) = &self.regresion {
            s.push('\n');
            s.push_str(&r.a_texto());
        }
        s
    }
}

/// El resultado de una prueba como texto: una linea con el veredicto y, si no paso, el detalle.
pub fn texto_de(r: &Resultado) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let marca = match r.veredicto {
        Veredicto::Pasa => "ok     ",
        Veredicto::Falla => "FALLA  ",
        Veredicto::Error => "ERROR  ",
    };
    let _ = writeln!(s, "{marca}{}", r.nombre);
    if let Some(m) = &r.motivo {
        for linea in m.lines() {
            let _ = writeln!(s, "         {linea}");
        }
    }
    for f in &r.fallos {
        let _ = writeln!(s, "         {}:", f.que);
        let _ = writeln!(s, "           esperado: {}", f.esperado);
        let _ = writeln!(s, "           obtenido: {}", f.obtenido);
        if let Some(d) = &f.donde {
            let fuente = d
                .fuente
                .as_ref()
                .map(|(a, l)| format!(" ({a}:{l})"))
                .unwrap_or_default();
            let _ = writeln!(
                s,
                "           en la instruccion {} {:#06x}: {}{fuente}",
                d.instante, d.direccion, d.instruccion
            );
        }
    }
    s
}

/// Resultado de una prueba y, si se pidio, lo que cubrio.
type Corrida = (Resultado, Option<Cobertura>);

fn hilos_para(n: usize, trabajo: usize) -> usize {
    let disponibles = if n == 0 {
        std::thread::available_parallelism().map_or(1, |x| x.get())
    } else {
        n
    };
    disponibles.clamp(1, trabajo.max(1))
}

/// Ejecuta las pruebas seleccionadas. Son independientes entre si (cada una tiene su propia
/// maquina), asi que se reparten entre hilos; el informe conserva el orden del archivo.
pub fn ejecutar_suite(proyecto: &Proyecto, opciones: &Opciones) -> Informe {
    let inicio = Instant::now();
    let elegidas = proyecto.seleccionar(opciones.nombre.as_deref(), opciones.etiqueta.as_deref());
    let omitidas = proyecto.pruebas.len() - elegidas.len();
    let n = hilos_para(opciones.hilos, elegidas.len());
    let resultados: Mutex<Vec<Option<Corrida>>> = Mutex::new(vec![None; elegidas.len()]);
    let siguiente = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..n {
            s.spawn(|| loop {
                let i = siguiente.fetch_add(1, Ordering::SeqCst);
                let Some(prueba) = elegidas.get(i) else { break };
                let r = if opciones.cobertura {
                    let mut c = Cobertura::nueva(&proyecto.maquina);
                    let r = ejecutar_con_cobertura(proyecto, prueba, &mut c);
                    (r, Some(c))
                } else {
                    (ejecutar(proyecto, prueba), None)
                };
                resultados
                    .lock()
                    .expect("ningun hilo entra en panico con el candado")[i] = Some(r);
            });
        }
    });
    let hechas: Vec<Corrida> = resultados
        .into_inner()
        .expect("ningun hilo entra en panico con el candado")
        .into_iter()
        .flatten()
        .collect();
    let cobertura = opciones.cobertura.then(|| {
        let mut total = Cobertura::nueva(&proyecto.maquina);
        for (_, c) in &hechas {
            if let Some(c) = c {
                total.fusionar(c);
            }
        }
        total.informe()
    });
    let pruebas: Vec<Resultado> = hechas.into_iter().map(|(r, _)| r).collect();
    let cuenta = |v: Veredicto| pruebas.iter().filter(|r| r.veredicto == v).count();
    Informe {
        formato: FORMATO_RESULTADOS.to_string(),
        proyecto: proyecto.ruta.to_string_lossy().into_owned(),
        maquina: proyecto.maquina.nombre.clone(),
        semilla: proyecto.semilla,
        resumen: Resumen {
            total: pruebas.len(),
            pasan: cuenta(Veredicto::Pasa),
            fallan: cuenta(Veredicto::Falla),
            errores: cuenta(Veredicto::Error),
            omitidas,
        },
        milisegundos: inicio.elapsed().as_millis() as u64,
        pruebas,
        cobertura,
        regresion: None,
    }
}
