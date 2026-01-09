//! `lorenz compilar`: de SGT a un programa para la maquina (RF-SGT-042).

use std::io::Write;

use lorenz_arch::modelo::Maquina;
use lorenz_sgt::analisis::Opciones;
use lorenz_sgt::compilar::{compilar, Compilado};

use crate::ensamblar::imagen_hex;
use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};

/// Extension de los fuentes de SGT.
pub const EXTENSION: &str = "sgt";

/// `true` si la ruta es un fuente de SGT.
pub fn es_sgt(ruta: &str) -> bool {
    std::path::Path::new(ruta)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpcionesCompilar {
    pub salida: Option<String>,
    /// Ensamblador generado, con la linea de SGT antes de cada grupo de instrucciones.
    pub ensamblador: Option<String>,
    /// Direccion de cada tramo de codigo y la linea de SGT que lo origino.
    pub lineas: Option<String>,
}

fn escribir(w: &mut dyn Write, t: &str) {
    let _ = w.write_all(t.as_bytes());
}

/// Compila un archivo. Los errores salen formateados, uno por diagnostico.
pub fn compilar_archivo(maquina: &Maquina, ruta: &str) -> Result<(Compilado, String), String> {
    let texto =
        std::fs::read_to_string(ruta).map_err(|e| format!("{ruta}: no se pudo leer: {e}"))?;
    match compilar(ruta, &texto, maquina, Opciones::default()) {
        Ok(c) => Ok((c, texto)),
        Err(e) => Err(e
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join("\n")),
    }
}

/// Tabla de depuracion: una linea por tramo, `direccion linea`.
pub fn tabla_de_lineas(c: &Compilado, ancho_direccion: u32) -> String {
    let digitos = (ancho_direccion as usize).div_ceil(4);
    let mut s = String::from("# direccion linea-sgt\n");
    for (d, l) in &c.lineas_fuente {
        s.push_str(&format!("{d:#0ancho$x} {l}\n", ancho = digitos + 2));
    }
    s
}

pub fn compilar_cli(
    ruta_maquina: &str,
    ruta_fuente: &str,
    opciones: &OpcionesCompilar,
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
    let (c, texto) = match compilar_archivo(&maquina, ruta_fuente) {
        Ok(r) => r,
        Err(e) => {
            escribir(errores, &format!("{e}\n"));
            return SALIDA_ERROR_EJECUCION;
        }
    };
    let hex = imagen_hex(&c.objeto, &maquina);
    let mut escribir_en = |ruta: &str, contenido: &str| -> bool {
        match std::fs::write(ruta, contenido) {
            Ok(()) => true,
            Err(e) => {
                escribir(errores, &format!("{ruta}: no se pudo escribir: {e}\n"));
                false
            }
        }
    };
    match &opciones.salida {
        Some(ruta) => {
            if !escribir_en(ruta, &hex) {
                return SALIDA_ERROR_USO;
            }
        }
        None => escribir(salida, &hex),
    }
    if let Some(ruta) = &opciones.ensamblador {
        if !escribir_en(ruta, &c.ensamblador_comentado(&texto)) {
            return SALIDA_ERROR_USO;
        }
    }
    if let Some(ruta) = &opciones.lineas {
        if !escribir_en(ruta, &tabla_de_lineas(&c, maquina.bits_direccion)) {
            return SALIDA_ERROR_USO;
        }
    }
    if opciones.salida.is_some() {
        escribir(
            salida,
            &format!(
                "{ruta_fuente}: {} palabras para {}\n",
                c.objeto.tamano(),
                maquina.nombre
            ),
        );
    }
    SALIDA_OK
}
