//! La unidad grafica en marcha: cola de comandos, ejecucion y ciclos propios.
//!
//! La unidad tiene su propio reloj. La CPU deja comandos en la cola; la unidad toma uno cuando
//! esta libre, lo ejecuta y queda ocupada tantos ciclos como cuesta. Los ciclos de la unidad se
//! cuentan aparte de los de la CPU (RF-GPU-017).
//!
//! **Modelo:** el efecto de un comando sobre la superficie se aplica al tomarlo; el coste se paga
//! despues, como tiempo ocupado. Es una simplificacion que no cambia el resultado final ni el
//! tiempo total, solo el instante en que un pixel se ve (D-101).

use std::collections::VecDeque;

use crate::comandos::Comando;
use crate::config::ConfigGpu;
use crate::error::ErrorGpu;
use crate::raster::{self, Vertice};
use crate::superficie::Superficie;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Metricas {
    /// Ciclos en que la unidad estuvo ocupada.
    pub ciclos_ocupada: u64,
    /// Ciclos transcurridos desde que arranco, ocupada o no.
    pub ciclos_totales: u64,
    pub comandos: u64,
    pub pixeles_escritos: u64,
    pub triangulos: u64,
    pub lotes: u64,
    /// Comandos rechazados por cola llena.
    pub rechazados: u64,
    /// Mayor numero de comandos que esperaron a la vez en la cola.
    pub cola_maxima: usize,
}

impl Metricas {
    /// Fraccion del tiempo que la unidad estuvo ocupada.
    pub fn utilizacion(&self) -> f64 {
        if self.ciclos_totales == 0 {
            0.0
        } else {
            self.ciclos_ocupada as f64 / self.ciclos_totales as f64
        }
    }
}

#[derive(Debug, Clone)]
pub struct Gpu {
    config: ConfigGpu,
    superficie: Superficie,
    color: u32,
    z: u32,
    prueba_z: bool,
    cola: VecDeque<Comando>,
    /// Ciclos que le faltan a la unidad para quedar libre.
    restante: u64,
    metricas: Metricas,
    /// Lotes terminados cuya interrupcion la CPU aun no recogio.
    lotes_pendientes: u64,
}

impl Gpu {
    pub fn nueva(config: ConfigGpu) -> Self {
        Self {
            superficie: Superficie::nueva(config.superficie),
            config,
            color: 0xFF_FFFF,
            z: 0,
            prueba_z: false,
            cola: VecDeque::new(),
            restante: 0,
            metricas: Metricas::default(),
            lotes_pendientes: 0,
        }
    }

    pub fn config(&self) -> &ConfigGpu {
        &self.config
    }

    pub fn superficie(&self) -> &Superficie {
        &self.superficie
    }

    pub fn metricas(&self) -> Metricas {
        self.metricas
    }

    /// Comandos esperando en la cola.
    pub fn en_cola(&self) -> usize {
        self.cola.len()
    }

    /// Si la unidad tiene trabajo: un comando en curso o comandos esperando.
    pub fn ocupada(&self) -> bool {
        self.restante > 0 || !self.cola.is_empty()
    }

    /// Deja un comando en la cola (RF-GPU-011). Con la cola llena lo rechaza y lo cuenta.
    pub fn enviar(&mut self, c: Comando) -> Result<(), ErrorGpu> {
        if self.cola.len() >= self.config.cola {
            self.metricas.rechazados += 1;
            return Err(ErrorGpu::ColaLlena {
                profundidad: self.config.cola,
            });
        }
        self.cola.push_back(c);
        self.metricas.cola_maxima = self.metricas.cola_maxima.max(self.cola.len());
        Ok(())
    }

    /// Lotes terminados desde la ultima llamada (RF-GPU-012). La CPU lo consulta para saber si
    /// debe levantar la interrupcion.
    pub fn tomar_lotes_terminados(&mut self) -> u64 {
        std::mem::take(&mut self.lotes_pendientes)
    }

    /// Hace pasar `ciclos` ciclos de la unidad.
    pub fn avanzar(&mut self, ciclos: u64) {
        let mut disponibles = ciclos;
        while disponibles > 0 {
            if self.restante == 0 {
                match self.cola.pop_front() {
                    Some(c) => self.restante = self.ejecutar(c),
                    None => {
                        self.metricas.ciclos_totales += disponibles;
                        return;
                    }
                }
            }
            let t = self.restante.min(disponibles);
            self.restante -= t;
            disponibles -= t;
            self.metricas.ciclos_ocupada += t;
            self.metricas.ciclos_totales += t;
        }
    }

    /// Avanza hasta vaciar la cola; devuelve los ciclos que tardo. Con `limite` se detiene antes.
    pub fn vaciar(&mut self, limite: u64) -> u64 {
        let mut gastados = 0;
        while self.ocupada() && gastados < limite {
            let paso = self.restante.max(1).min(limite - gastados);
            self.avanzar(paso);
            gastados += paso;
        }
        gastados
    }

    fn z_activo(&self) -> Option<u32> {
        self.prueba_z.then_some(self.z)
    }

    /// Aplica el comando y devuelve lo que cuesta.
    fn ejecutar(&mut self, c: Comando) -> u64 {
        let k = self.config.costes;
        self.metricas.comandos += 1;
        let z = self.z_activo();
        let (pixeles, extra) = match c {
            Comando::Nada | Comando::Profundidad(_) | Comando::PruebaZ(_) | Comando::Color(_) => {
                match c {
                    Comando::Color(rgb) => self.color = rgb,
                    Comando::Profundidad(v) => self.z = v,
                    Comando::PruebaZ(b) => self.prueba_z = b && self.config.superficie.profundidad,
                    _ => {}
                }
                (0, 0)
            }
            Comando::Limpiar => {
                self.superficie.limpiar(self.color);
                (
                    (self.superficie.ancho() as u64) * (self.superficie.alto() as u64),
                    0,
                )
            }
            Comando::Rellenar { x, y, ancho, alto } => (
                raster::rellenar(&mut self.superficie, x, y, ancho, alto, self.color, z),
                0,
            ),
            Comando::Linea { x0, y0, x1, y1 } => (
                raster::linea(&mut self.superficie, x0, y0, x1, y1, self.color, z),
                0,
            ),
            Comando::Triangulo { v } => {
                self.metricas.triangulos += 1;
                let v = v.map(|(x, y)| Vertice {
                    x,
                    y,
                    color: self.color,
                });
                (
                    raster::triangulo(&mut self.superficie, v, false, z),
                    k.triangulo,
                )
            }
            Comando::TrianguloColor { v } => {
                self.metricas.triangulos += 1;
                (
                    raster::triangulo(&mut self.superficie, v, true, z),
                    k.triangulo,
                )
            }
            Comando::FinLote => {
                self.metricas.lotes += 1;
                self.lotes_pendientes += 1;
                (0, 0)
            }
        };
        self.metricas.pixeles_escritos += pixeles;
        k.comando + extra + pixeles * k.pixel
    }
}
