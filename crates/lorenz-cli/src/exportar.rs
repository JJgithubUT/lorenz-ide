//! La orden `exportar` (RF-ECO-024, RF-ECO-034 a RF-ECO-036).

use std::io::Write;
use std::path::Path;

use crate::ordenes::{SALIDA_ERROR_USO, SALIDA_OK};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formato {
    Verilog,
    Pdf,
    Logisim,
}

impl Formato {
    pub fn desde(texto: &str) -> Option<Self> {
        match texto {
            "verilog" => Some(Self::Verilog),
            "pdf" => Some(Self::Pdf),
            "logisim" => Some(Self::Logisim),
            _ => None,
        }
    }
}

fn fallo(errores: &mut dyn Write, m: impl std::fmt::Display) -> i32 {
    let _ = writeln!(errores, "{m}");
    SALIDA_ERROR_USO
}

fn escribir(ruta: &Path, texto: impl AsRef<[u8]>) -> Result<(), String> {
    std::fs::write(ruta, texto).map_err(|e| format!("{}: no se pudo escribir: {e}", ruta.display()))
}

pub fn exportar_cli(
    formato: Formato,
    ruta_maquina: &str,
    programa: Option<&str>,
    destino: Option<&str>,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let maquina = match lorenz_arch::cargar_archivo(ruta_maquina) {
        Ok(m) => m,
        Err(e) => return fallo(errores, format!("{ruta_maquina}: {e}")),
    };
    let tramos = match programa {
        None => None,
        Some(p) => match crate::ensamblar::cargar_segmentos(&maquina, p, maquina.arranque) {
            Ok(t) => Some(t),
            Err(e) => return fallo(errores, e),
        },
    };
    match formato {
        Formato::Verilog => {
            let Some(dir) = destino else {
                return fallo(errores, "exportar verilog necesita -o CARPETA");
            };
            let modelo = match lorenz_arch::verilog::exportar(&maquina) {
                Ok(m) => m,
                Err(e) => return fallo(errores, format!("{ruta_maquina}: {e}")),
            };
            let id = lorenz_arch::verilog::identificador(&maquina.nombre);
            let dir = Path::new(dir);
            let r = std::fs::create_dir_all(dir)
                .map_err(|e| format!("{}: {e}", dir.display()))
                .and_then(|()| escribir(&dir.join(format!("{id}_cpu.v")), &modelo.cpu))
                .and_then(|()| escribir(&dir.join(format!("{id}_tb.v")), &modelo.banco))
                .and_then(|()| {
                    let t = tramos.unwrap_or_default();
                    escribir(
                        &dir.join("programa.mem"),
                        lorenz_arch::verilog::imagen_mem(&t, maquina.bits_palabra),
                    )
                });
            if let Err(e) = r {
                return fallo(errores, e);
            }
            for a in &modelo.avisos {
                let _ = writeln!(errores, "aviso: {a}");
            }
            let _ = writeln!(
                salida,
                "{}: {id}_cpu.v, {id}_tb.v y programa.mem en {}",
                maquina.nombre,
                dir.display()
            );
            SALIDA_OK
        }
        Formato::Pdf => {
            let Some(ruta) = destino else {
                return fallo(errores, "exportar pdf necesita -o ARCHIVO.pdf");
            };
            let doc = match crate::documentar::generar(
                ruta_maquina,
                &std::fs::read_to_string(ruta_maquina).unwrap_or_default(),
            ) {
                Ok(d) => d,
                Err(e) => return fallo(errores, e),
            };
            let pdf = crate::pdf::markdown_a_pdf(&doc, &format!("Arquitectura {}", maquina.nombre));
            if let Err(e) = escribir(Path::new(ruta), pdf) {
                return fallo(errores, e);
            }
            let _ = writeln!(salida, "{}: documentacion en {ruta}", maquina.nombre);
            SALIDA_OK
        }
        Formato::Logisim => {
            let Some(ruta) = destino else {
                return fallo(errores, "exportar logisim necesita -o ARCHIVO.circ");
            };
            let circ = lorenz_arch::logisim::exportar(&maquina, &tramos.unwrap_or_default());
            if let Err(e) = escribir(Path::new(ruta), circ) {
                return fallo(errores, e);
            }
            let _ = writeln!(
                salida,
                "{}: circuito en {ruta} (bloques sin cablear)",
                maquina.nombre
            );
            SALIDA_OK
        }
    }
}
