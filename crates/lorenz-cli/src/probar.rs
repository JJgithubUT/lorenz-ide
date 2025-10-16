//! `lorenz probar`: ejecutar las pruebas de un proyecto (RF-TST-011, RF-TST-014, RF-TST-030 a
//! RF-TST-032).

use std::io::{BufRead, Write};

use lorenz_dbg::comandos::Sesion;
use lorenz_tst::ejecutor::{ejecutar_con_depurador, Veredicto};
use lorenz_tst::formato::Proyecto;
use lorenz_tst::suite::{
    ejecutar_suite, texto_de, Opciones, SALIDA_ERROR_EN_PRUEBAS, SALIDA_FALLAN, SALIDA_OK,
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
    let informe = ejecutar_suite(
        &proyecto,
        &Opciones {
            nombre: o.nombre.clone(),
            etiqueta: o.etiqueta.clone(),
            hilos: o.hilos,
            cobertura: o.cobertura,
        },
    );
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
    informe.codigo_de_salida()
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
