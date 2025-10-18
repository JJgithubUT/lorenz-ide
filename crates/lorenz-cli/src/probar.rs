//! `lorenz probar`: ejecutar las pruebas de un proyecto (RF-TST-011, RF-TST-014, RF-TST-030 a
//! RF-TST-032).

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use lorenz_dbg::comandos::Sesion;
use lorenz_tst::ejecutor::{ejecutar_con_depurador, Veredicto};
use lorenz_tst::formato::Proyecto;
use lorenz_tst::regresion::{comparar_resultados, Umbral};
use lorenz_tst::suite::{
    ejecutar_suite, texto_de, Informe, Opciones, SALIDA_ERROR_EN_PRUEBAS, SALIDA_FALLAN, SALIDA_OK,
};

use crate::ordenes::SALIDA_ERROR_USO;

fn escribir(salida: &mut dyn Write, texto: &str) {
    let _ = salida.write_all(texto.as_bytes());
}

/// Argumentos de `probar` ya interpretados.
#[derive(Debug, Clone, Default)]
pub struct OpcionesProbar {
    pub nombre: Option<String>,
    pub etiqueta: Option<String>,
    pub hilos: usize,
    pub json: bool,
    pub salida: Option<String>,
    pub depurar: Option<String>,
    pub cobertura: bool,
    /// Resultados anteriores con los que comparar.
    pub comparar: Option<String>,
    /// Comparar con la ultima ejecucion guardada y guardar esta.
    pub historial: bool,
    /// Aumento de ciclos tolerado, en porcentaje.
    pub umbral: f64,
}

pub fn probar_cli(
    ruta: &str,
    o: &OpcionesProbar,
    entrada: &mut dyn BufRead,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let proyecto = match Proyecto::desde_archivo(ruta) {
        Ok(p) => p,
        Err(e) => {
            escribir(errores, &format!("error: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    if let Some(nombre) = &o.depurar {
        return depurar_prueba(&proyecto, nombre, entrada, salida, errores);
    }
    let mut informe = ejecutar_suite(
        &proyecto,
        &Opciones {
            nombre: o.nombre.clone(),
            etiqueta: o.etiqueta.clone(),
            hilos: o.hilos,
            cobertura: o.cobertura,
        },
    );
    let ruta_historial = o.historial.then(|| ruta_del_historial(ruta));
    // Lo que se guarda para la proxima vez no lleva la comparacion de esta.
    let para_guardar = ruta_historial.as_ref().map(|_| informe.a_json());
    let base = match (&o.comparar, &ruta_historial) {
        (Some(archivo), _) => match leer_resultados(archivo) {
            Ok(i) => Some(i),
            Err(e) => {
                escribir(errores, &format!("error: {e}\n"));
                return SALIDA_ERROR_USO;
            }
        },
        (None, Some(h)) if h.exists() => match leer_resultados(&h.to_string_lossy()) {
            Ok(i) => Some(i),
            Err(e) => {
                escribir(errores, &format!("error: {e}\n"));
                return SALIDA_ERROR_USO;
            }
        },
        (None, Some(_)) => {
            escribir(
                errores,
                "aviso: no hay una ejecucion anterior con la que comparar; esta quedara guardada\n",
            );
            None
        }
        (None, None) => None,
    };
    let mut codigo_regresion = 0;
    if let Some(base) = &base {
        let mut r = comparar_resultados(
            base,
            &informe,
            Umbral {
                porcentaje: o.umbral,
            },
        );
        if o.nombre.is_some() || o.etiqueta.is_some() {
            // Las pruebas que el filtro dejo fuera no han desaparecido.
            r.cambios
                .retain(|c| !matches!(c, lorenz_tst::regresion::Cambio::Desaparecida { .. }));
        }
        codigo_regresion = r.codigo_de_salida();
        informe.regresion = Some(r);
    }
    if let (Some(h), Some(json)) = (&ruta_historial, &para_guardar) {
        if let Err(e) = guardar_historial(h, json) {
            escribir(
                errores,
                &format!("{}: no se pudo escribir: {e}\n", h.display()),
            );
            return SALIDA_ERROR_USO;
        }
    }
    if let Some(archivo) = &o.salida {
        if let Err(e) = std::fs::write(archivo, informe.a_json()) {
            escribir(errores, &format!("{archivo}: no se pudo escribir: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    }
    if o.json {
        escribir(salida, &informe.a_json());
    } else {
        escribir(salida, &informe.a_texto());
    }
    match informe.codigo_de_salida() {
        SALIDA_OK => codigo_regresion,
        otro => otro,
    }
}

/// Donde se guardan los resultados de la ultima ejecucion de un proyecto (RF-TST-036).
pub fn ruta_del_historial(pruebas: &str) -> PathBuf {
    let carpeta = Path::new(pruebas)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    carpeta.join(".lorenz").join("ultimo.json")
}

fn guardar_historial(ruta: &Path, json: &str) -> std::io::Result<()> {
    if let Some(dir) = ruta.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(ruta, json)
}

fn leer_resultados(archivo: &str) -> Result<Informe, String> {
    let texto =
        std::fs::read_to_string(archivo).map_err(|e| format!("{archivo}: no se pudo leer: {e}"))?;
    Informe::desde_json(&texto).map_err(|e| format!("{archivo}: {e}"))
}

/// Ejecuta una prueba y, si falla, abre el depurador en el estado exacto en que termino
/// (RF-TST-014). Desde ahi se puede retroceder, porque la ejecucion se grabo.
fn depurar_prueba(
    proyecto: &Proyecto,
    nombre: &str,
    entrada: &mut dyn BufRead,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let Some(prueba) = proyecto.pruebas.iter().find(|p| p.nombre() == nombre) else {
        escribir(
            errores,
            &format!("error: el proyecto no tiene una prueba llamada `{nombre}`\n"),
        );
        return SALIDA_ERROR_USO;
    };
    let (resultado, depurador) = ejecutar_con_depurador(proyecto, prueba);
    escribir(salida, &texto_de(&resultado));
    match (resultado.veredicto, depurador) {
        (Veredicto::Pasa, _) => {
            escribir(salida, "la prueba pasa; no hay nada que depurar\n");
            SALIDA_OK
        }
        (Veredicto::Error, _) => SALIDA_ERROR_EN_PRUEBAS,
        (Veredicto::Falla, None) => {
            escribir(
                errores,
                "no se pudo reconstruir el estado del fallo para depurarlo\n",
            );
            SALIDA_FALLAN
        }
        (Veredicto::Falla, Some(d)) => {
            let titulo = format!("depurando `{nombre}` en el estado en que fallo");
            crate::depurar::sesion_de_ordenes(
                Sesion::nueva(d),
                &titulo,
                None,
                entrada,
                salida,
                errores,
            );
            SALIDA_FALLAN
        }
    }
}
