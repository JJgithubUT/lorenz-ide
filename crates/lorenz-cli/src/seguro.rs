//! El modo seguro (RF-ECO-009, RF-ECO-038): `--seguro` o `LORENZ_SEGURO=1`.
//!
//! Pensado para ejecutar programas ajenos, por ejemplo en un servidor. Prohibe escribir archivos,
//! lanzar procesos y abrir conexiones: solo se permiten las ordenes que leen lo que se les nombra y
//! escriben por la salida estandar, y los dispositivos externos de las maquinas no arrancan.

use crate::args::Orden;

pub const OPCION: &str = "--seguro";
pub const VARIABLE: &str = "LORENZ_SEGURO";

/// Si el entorno pide el modo seguro.
pub fn pedido_por_entorno() -> bool {
    std::env::var(VARIABLE).is_ok_and(|v| !v.is_empty() && v != "0")
}

/// Quita `--seguro` de los argumentos y dice si estaba.
pub fn extraer(args: &[String]) -> (Vec<String>, bool) {
    let seguro = args.iter().any(|a| a == OPCION);
    (
        args.iter().filter(|a| *a != OPCION).cloned().collect(),
        seguro,
    )
}

/// Deja el modo seguro activo mientras viva y lo devuelve a su valor anterior al soltarlo.
pub struct Guardia {
    antes: bool,
}

impl Guardia {
    pub fn activar() -> Self {
        let antes = lorenz_emu::externo::estan_prohibidos();
        lorenz_emu::externo::prohibir(true);
        Self { antes }
    }
}

impl Drop for Guardia {
    fn drop(&mut self) {
        lorenz_emu::externo::prohibir(self.antes);
    }
}

fn sin(valor: bool, que: &str) -> Result<(), String> {
    if valor {
        Err(format!(
            "{que} escribe en disco; no esta permitido en modo seguro"
        ))
    } else {
        Ok(())
    }
}

/// Comprueba que la orden se puede ejecutar en modo seguro.
pub fn comprobar(orden: &Orden) -> Result<(), String> {
    match orden {
        Orden::Ayuda
        | Orden::Version
        | Orden::Validar { .. }
        | Orden::Comparar { .. }
        | Orden::Desensamblar { .. }
        | Orden::Explicar { .. }
        | Orden::Plantillas
        | Orden::Ejercicio { .. }
        | Orden::Desafio { .. } => Ok(()),
        Orden::Ensamblar { opciones, .. } => sin(
            opciones.salida.is_some()
                || opciones.mapa.is_some()
                || opciones.listado.is_some()
                || opciones.simbolos.is_some()
                || opciones.objeto.is_some(),
            "esa opcion de `ensamblar`",
        ),
        Orden::Compilar { opciones, .. } => sin(
            opciones.salida.is_some()
                || opciones.ensamblador.is_some()
                || opciones.lineas.is_some()
                || opciones.costes.is_some(),
            "esa opcion de `compilar`",
        ),
        Orden::Ejecutar { volcados, .. } => sin(
            volcados.iter().any(|v| v.archivo.is_some()),
            "un volcado a archivo",
        ),
        Orden::Probar {
            salida, historial, ..
        } => sin(salida.is_some() || *historial, "esa opcion de `probar`"),
        Orden::Documentar {
            salida, vigilar, ..
        } => sin(salida.is_some() || *vigilar, "esa opcion de `documentar`"),
        _ => Err("esta orden no esta disponible en modo seguro".into()),
    }
}
