//! El IDE de Lorenz: un servidor local con una API JSON sobre el nucleo (RF-IDE-001).
//!
//! Cada pestana del navegador usa sesiones independientes; el proyecto es la carpeta desde la que
//! se lanza `lorenz ide`, y ninguna ruta puede salir de ella.

mod api;
mod sesion;

pub use api::router;
pub use sesion::{Ide, MAX_SESIONES};

pub const PAGINA: &str = include_str!("pagina.html");

use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};

/// `lorenz ide`: sirve el proyecto en `127.0.0.1` y espera (para siempre, o `segundos`).
pub fn ide_cli(
    carpeta: Option<&str>,
    puerto: u16,
    segundos: Option<u64>,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let raiz = std::path::PathBuf::from(carpeta.unwrap_or("."));
    if !raiz.is_dir() {
        let _ = writeln!(errores, "{}: no es una carpeta", raiz.display());
        return SALIDA_ERROR_USO;
    }
    let ide = Arc::new(Ide::nuevo(&raiz));
    let servidor = match lorenz_web::Servidor::iniciar(
        &format!("127.0.0.1:{puerto}"),
        router(ide),
        lorenz_web::Limites::default(),
    ) {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(errores, "no se pudo abrir el puerto {puerto}: {e}");
            return SALIDA_ERROR_EJECUCION;
        }
    };
    let _ = writeln!(
        salida,
        "Lorenz IDE en http://{} (carpeta {}); Ctrl-C para salir",
        servidor.direccion(),
        raiz.display()
    );
    let _ = salida.flush();
    match segundos {
        Some(s) => std::thread::sleep(Duration::from_secs(s)),
        None => loop {
            std::thread::sleep(Duration::from_secs(3600));
        },
    }
    servidor.detener();
    SALIDA_OK
}
