//! Imagen que una pantalla muestra a partir de su region de memoria.

use lorenz_arch::modelo::{Maquina, TipoDispositivo};

use crate::error::ErrorEjecucion;
use crate::memoria::Memoria;

/// Pixeles de una pantalla, fila por fila de arriba abajo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imagen {
    pub ancho: u32,
    pub alto: u32,
    /// Un indice de color por pixel; `ancho * alto` valores.
    pub pixeles: Vec<u8>,
}

impl Imagen {
    /// Indice del pixel en la columna `x` y la fila `y`, o `None` si queda fuera.
    pub fn pixel(&self, x: u32, y: u32) -> Option<u8> {
        if x >= self.ancho || y >= self.alto {
            return None;
        }
        self.pixeles.get((y * self.ancho + x) as usize).copied()
    }

    /// Dibujo con caracteres: `#` para un pixel encendido y `.` para uno apagado.
    pub fn a_texto(&self) -> String {
        let mut texto = String::with_capacity(((self.ancho + 1) * self.alto) as usize);
        for fila in self.pixeles.chunks(self.ancho.max(1) as usize) {
            texto.extend(fila.iter().map(|p| if *p == 0 { '.' } else { '#' }));
            texto.push('\n');
        }
        texto
    }
}

/// Lee la region de memoria de la pantalla `nombre` y la convierte en imagen.
///
/// Formatos admitidos: `indice1` (un bit por pixel, el bit mas significativo de cada palabra
/// primero) e `indice8` (una palabra por pixel). Lo que la region no cubra se dibuja apagado.
pub fn leer_imagen(
    maquina: &Maquina,
    memoria: &Memoria,
    nombre: &str,
) -> Result<Imagen, ErrorEjecucion> {
    let dispositivo =
        maquina
            .dispositivo(nombre)
            .ok_or_else(|| ErrorEjecucion::DispositivoInexistente {
                nombre: nombre.to_string(),
            })?;
    let TipoDispositivo::Pantalla {
        ancho,
        alto,
        formato,
    } = &dispositivo.tipo
    else {
        return Err(ErrorEjecucion::DispositivoIncorrecto {
            nombre: nombre.to_string(),
            esperado: "una pantalla",
        });
    };
    let total = (*ancho as usize) * (*alto as usize);
    let mut pixeles = vec![0u8; total];
    let Some(region) = maquina
        .regiones
        .iter()
        .find(|r| r.dispositivo.as_deref() == Some(nombre))
    else {
        return Ok(Imagen {
            ancho: *ancho,
            alto: *alto,
            pixeles,
        });
    };
    let bits = maquina.bits_palabra as usize;
    match formato.as_str() {
        "indice1" => {
            for (i, pixel) in pixeles.iter_mut().enumerate() {
                let direccion = region.desde + (i / bits) as u64;
                if direccion > region.hasta {
                    break;
                }
                let palabra = memoria.leer_privilegiado(direccion)?;
                let bit = bits - 1 - (i % bits);
                *pixel = ((palabra >> bit) & 1) as u8;
            }
        }
        "indice8" => {
            for (i, pixel) in pixeles.iter_mut().enumerate() {
                let direccion = region.desde + i as u64;
                if direccion > region.hasta {
                    break;
                }
                *pixel = (memoria.leer_privilegiado(direccion)? & 0xFF) as u8;
            }
        }
        otro => {
            return Err(ErrorEjecucion::FormatoDePantallaNoSoportado {
                nombre: nombre.to_string(),
                formato: otro.to_string(),
            })
        }
    }
    Ok(Imagen {
        ancho: *ancho,
        alto: *alto,
        pixeles,
    })
}
