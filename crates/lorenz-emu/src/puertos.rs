//! Puertos de entrada y salida declarados por los dispositivos de la maquina.

use std::collections::{BTreeMap, VecDeque};

use lorenz_arch::modelo::{Maquina, TipoDispositivo};

use crate::error::ErrorEjecucion;
use crate::estado::mascara_de;

#[derive(Debug, Clone, Default)]
struct Puerto {
    entrada: VecDeque<u64>,
    salida: Vec<u64>,
    /// Si es el puerto de estado de un teclado: el puerto de datos del que informa.
    estado_de: Option<u64>,
}

/// Colas de entrada y registro de salida de cada puerto.
///
/// Solo existen los puertos que algun dispositivo declara; tocar otro es un error
/// (RF-EMU-030). Una lectura con la cola vacia devuelve 0, como un puerto sin dato.
#[derive(Debug, Clone)]
pub struct Puertos {
    puertos: BTreeMap<u64, Puerto>,
    mascara: u64,
}

impl Puertos {
    pub fn nuevo(maquina: &Maquina) -> Self {
        let mut puertos = BTreeMap::new();
        for d in &maquina.dispositivos {
            for numero in d.puertos.values() {
                puertos.insert(*numero, Puerto::default());
            }
            // Un teclado informa por `estado` de si hay una tecla esperando en `datos`.
            if d.tipo == TipoDispositivo::Teclado {
                if let (Some(datos), Some(estado)) = (d.puertos.get("datos"), d.puertos.get("estado")) {
                    if let Some(p) = puertos.get_mut(estado) {
                        p.estado_de = Some(*datos);
                    }
                }
            }
        }
        Puertos {
            puertos,
            mascara: mascara_de(maquina.bits_palabra),
        }
    }

    fn puerto(&mut self, numero: u64) -> Result<&mut Puerto, ErrorEjecucion> {
        self.puertos
            .get_mut(&numero)
            .ok_or(ErrorEjecucion::PuertoNoDeclarado { puerto: numero })
    }

    /// Numeros de puerto declarados, de menor a mayor.
    pub fn numeros(&self) -> Vec<u64> {
        self.puertos.keys().copied().collect()
    }

    /// Deja un valor en la cola de entrada, truncado al ancho de palabra.
    pub fn encolar_entrada(&mut self, numero: u64, valor: u64) -> Result<(), ErrorEjecucion> {
        let mascara = self.mascara;
        self.puerto(numero)?.entrada.push_back(valor & mascara);
        Ok(())
    }

    /// Lee el siguiente valor de entrada; 0 si no hay ninguno.
    ///
    /// El puerto de estado de un teclado no consume nada: vale 1 si hay una tecla esperando en el
    /// puerto de datos y 0 si no.
    pub fn leer(&mut self, numero: u64) -> Result<u64, ErrorEjecucion> {
        if let Some(datos) = self.puerto(numero)?.estado_de {
            let hay = self.puertos.get(&datos).is_some_and(|p| !p.entrada.is_empty());
            return Ok(u64::from(hay));
        }
        Ok(self.puerto(numero)?.entrada.pop_front().unwrap_or(0))
    }

    /// Cuantos valores esperan en la cola de entrada.
    pub fn pendientes(&mut self, numero: u64) -> Result<usize, ErrorEjecucion> {
        Ok(self.puerto(numero)?.entrada.len())
    }

    /// Escribe un valor de salida, truncado al ancho de palabra.
    pub fn escribir(&mut self, numero: u64, valor: u64) -> Result<(), ErrorEjecucion> {
        let mascara = self.mascara;
        self.puerto(numero)?.salida.push(valor & mascara);
        Ok(())
    }

    /// Todo lo escrito en un puerto, en orden.
    pub fn salida(&mut self, numero: u64) -> Result<&[u64], ErrorEjecucion> {
        Ok(&self.puerto(numero)?.salida)
    }

    /// Vacia todas las colas y registros.
    pub fn reiniciar(&mut self) {
        for p in self.puertos.values_mut() {
            p.entrada.clear();
            p.salida.clear();
        }
    }
}
