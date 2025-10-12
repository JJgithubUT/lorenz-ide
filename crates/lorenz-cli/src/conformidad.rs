//! `lorenz conformidad`: comprobar el emulador contra la semantica de una arquitectura
//! (RF-TST-016 a RF-TST-020).

use std::io::Write;

use lorenz_tst::conformidad::{ejecutar_conformidad, Opciones};

use crate::ordenes::SALIDA_ERROR_USO;

fn escribir(salida: &mut dyn Write, texto: &str) {
    let _ = salida.write_all(texto.as_bytes());
}

/// Argumentos de `conformidad` ya interpretados.
#[derive(Debug, Clone, Default)]
pub struct OpcionesConformidad {
    pub instruccion: Option<String>,
    pub casos: Option<usize>,
    pub repeticiones: Option<usize>,
    pub json: bool,
    pub salida: Option<String>,
}

pub fn conformidad_cli(
    ruta_maquina: &str,
    o: &OpcionesConformidad,
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
    let mut opciones = Opciones {
        mnemonico: o.instruccion.clone(),
        ..Opciones::default()
    };
    if let Some(c) = o.casos {
        opciones.max_combinaciones = c;
    }
    if let Some(r) = o.repeticiones {
        opciones.repeticiones = r;
    }
    let informe = match ejecutar_conformidad(&maquina, &opciones) {
        Ok(i) => i,
        Err(e) => {
            escribir(errores, &format!("error: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
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
