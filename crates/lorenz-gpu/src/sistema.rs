//! La CPU y la unidad grafica juntas (RF-GPU-010, RF-GPU-012).
//!
//! La descripcion declara un dispositivo `generico` con tres puertos y la seccion `[gpu]` lo
//! nombra:
//!
//! - `comando`: la CPU escribe aqui las palabras de cada comando, una tras otra.
//! - `estado`: se lee. Bit 0, la unidad esta ocupada; bit 1, la cola esta llena; bit 2, hubo un
//!   error (comando desconocido o cola desbordada); bit 3, termino un lote. Desde el bit 8, los
//!   huecos libres de la cola.
//! - `control`: escribir un 1 borra los bits 2 y 3 de `estado`.
//!
//! Las palabras se leen con el ancho de la maquina: los argumentos con signo se extienden, y los
//! colores van en el formato nativo de la palabra (RGB332 en 8 bits, RGB565 en 16 y 0xRRGGBB desde
//! 24).

use lorenz_arch::modelo::Maquina;
use lorenz_emu::{Emulador, ErrorEjecucion, Paso, Resumen};

use crate::comandos::*;
use crate::config::ConfigGpu;
use crate::error::ErrorGpu;
use crate::unidad::Gpu;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arg {
    Con,
    Sin,
    Color,
}

/// Como se interpreta cada palabra de un comando que llega por el puerto.
fn formato(op: i64) -> &'static [Arg] {
    use Arg::*;
    match op {
        OP_COLOR => &[Color],
        OP_RELLENAR | OP_LINEA => &[Con, Con, Con, Con],
        OP_TRIANGULO => &[Con, Con, Con, Con, Con, Con],
        OP_TRIANGULO_COLOR => &[Con, Con, Color, Con, Con, Color, Con, Con, Color],
        OP_PROFUNDIDAD | OP_PRUEBA_Z => &[Sin],
        _ => &[],
    }
}

/// Lleva un color de la palabra de la maquina a `0xRRGGBB`.
pub fn color_nativo(palabra: u64, bits_palabra: u32) -> u32 {
    let ampliar = |v: u64, bits: u32| -> u32 {
        let mut r = (v << (8 - bits)) as u32 & 0xFF;
        let mut k = bits;
        while k < 8 {
            r |= r >> k;
            k *= 2;
        }
        r
    };
    if bits_palabra >= 24 {
        return (palabra & 0xFF_FFFF) as u32;
    }
    let (r, g, b) = if bits_palabra >= 16 {
        (
            ampliar((palabra >> 11) & 0x1F, 5),
            ampliar((palabra >> 5) & 0x3F, 6),
            ampliar(palabra & 0x1F, 5),
        )
    } else {
        (
            ampliar((palabra >> 5) & 0x7, 3),
            ampliar((palabra >> 2) & 0x7, 3),
            ampliar(palabra & 0x3, 2),
        )
    };
    (r << 16) | (g << 8) | b
}

fn con_signo(v: u64, bits: u32) -> i64 {
    if bits >= 64 {
        return v as i64;
    }
    let desp = 64 - bits;
    ((v << desp) as i64) >> desp
}

/// Convierte las palabras crudas de un comando completo (la operacion y sus argumentos) a los
/// enteros que entiende [`Comando::decodificar`].
pub fn convertir_palabras(crudas: &[u64], bits_palabra: u32) -> Vec<i64> {
    let op = crudas[0] as i64;
    let mut v = vec![op];
    for (w, a) in crudas[1..].iter().zip(formato(op)) {
        v.push(match a {
            Arg::Con => con_signo(*w, bits_palabra),
            Arg::Sin => *w as i64,
            Arg::Color => color_nativo(*w, bits_palabra) as i64,
        });
    }
    v
}

pub struct SistemaGrafico {
    emu: Emulador,
    gpu: Gpu,
    bits: u32,
    p_comando: u64,
    p_estado: u64,
    p_control: Option<u64>,
    leidas: usize,
    leidas_control: usize,
    pendiente: Vec<u64>,
    error: bool,
    lote: bool,
    ciclos_cpu: u64,
}

impl SistemaGrafico {
    pub fn nuevo(emu: Emulador, config: ConfigGpu) -> Result<Self, ErrorGpu> {
        let nombre = config.dispositivo.clone().ok_or_else(|| {
            ErrorGpu::Config(
                "gpu: falta `dispositivo`, el nombre del dispositivo que la CPU maneja".into(),
            )
        })?;
        let maquina: &Maquina = emu.maquina();
        let d = maquina.dispositivo(&nombre).ok_or_else(|| {
            ErrorGpu::Config(format!(
                "gpu: la maquina no tiene un dispositivo llamado `{nombre}`"
            ))
        })?;
        let puerto = |p: &str| d.puertos.get(p).copied();
        let p_comando = puerto("comando").ok_or_else(|| {
            ErrorGpu::Config(format!(
                "gpu: el dispositivo `{nombre}` no declara el puerto `comando`"
            ))
        })?;
        let p_estado = puerto("estado").ok_or_else(|| {
            ErrorGpu::Config(format!(
                "gpu: el dispositivo `{nombre}` no declara el puerto `estado`"
            ))
        })?;
        let p_control = puerto("control");
        let bits = maquina.bits_palabra;
        let mut s = Self {
            emu,
            gpu: Gpu::nueva(config),
            bits,
            p_comando,
            p_estado,
            p_control,
            leidas: 0,
            leidas_control: 0,
            pendiente: Vec::new(),
            error: false,
            lote: false,
            ciclos_cpu: 0,
        };
        s.publicar_estado();
        Ok(s)
    }

    pub fn emulador(&self) -> &Emulador {
        &self.emu
    }

    pub fn emulador_mut(&mut self) -> &mut Emulador {
        &mut self.emu
    }

    pub fn gpu(&self) -> &Gpu {
        &self.gpu
    }

    /// Ciclos que lleva consumidos la CPU, aparte de los de la unidad (RF-GPU-017).
    pub fn ciclos_cpu(&self) -> u64 {
        self.ciclos_cpu
    }

    fn publicar_estado(&mut self) {
        let libres = (self.gpu.config().cola - self.gpu.en_cola()) as u64;
        let valor = u64::from(self.gpu.ocupada())
            | u64::from(libres == 0) << 1
            | u64::from(self.error) << 2
            | u64::from(self.lote) << 3
            | libres << 8;
        let _ = self.emu.puertos_mut().publicar(self.p_estado, valor);
    }

    /// Recoge lo que la CPU escribio en los puertos desde la ultima vez.
    fn recoger(&mut self) {
        let nuevas: Vec<u64> = self
            .emu
            .puertos_mut()
            .salida(self.p_comando)
            .map(|s| s[self.leidas..].to_vec())
            .unwrap_or_default();
        self.leidas += nuevas.len();
        for w in nuevas {
            if self.pendiente.is_empty() && Comando::palabras_necesarias(w as i64).is_none() {
                self.error = true;
                continue;
            }
            self.pendiente.push(w);
            let op = self.pendiente[0] as i64;
            let (n, _) = Comando::palabras_necesarias(op).unwrap_or((1, ""));
            if self.pendiente.len() == n {
                let palabras = convertir_palabras(&self.pendiente, self.bits);
                self.pendiente.clear();
                match Comando::decodificar(&palabras) {
                    Ok((c, _)) => {
                        if self.gpu.enviar(c).is_err() {
                            self.error = true;
                        }
                    }
                    Err(_) => self.error = true,
                }
            }
        }
        if let Some(pc) = self.p_control {
            let n = self.emu.puertos_mut().salida(pc).map_or(0, |s| s.len());
            let reconocidos = n > self.leidas_control;
            let hubo_uno = reconocidos
                && self
                    .emu
                    .puertos_mut()
                    .salida(pc)
                    .is_ok_and(|s| s[self.leidas_control..].iter().any(|v| v & 1 == 1));
            self.leidas_control = n;
            if hubo_uno {
                self.error = false;
                self.lote = false;
            }
        }
    }

    /// Ejecuta una instruccion de la CPU y hace correr la unidad el mismo tiempo.
    pub fn paso(&mut self) -> Result<Option<Paso>, ErrorEjecucion> {
        let paso = self.emu.paso()?;
        self.recoger();
        if let Some(p) = &paso {
            self.ciclos_cpu += p.ciclos;
            self.gpu.avanzar(p.ciclos * self.gpu.config().razon_reloj);
            let lotes = self.gpu.tomar_lotes_terminados();
            if lotes > 0 {
                self.lote = true;
                if let Some(linea) = self.gpu.config().interrupcion {
                    self.emu.levantar_interrupcion(linea)?;
                }
            }
        }
        self.publicar_estado();
        Ok(paso)
    }

    /// Ejecuta hasta que la CPU se detenga (o se agote `limite` instrucciones).
    pub fn ejecutar(&mut self, limite: u64) -> Result<Resumen, ErrorEjecucion> {
        let mut r = Resumen {
            instrucciones: 0,
            ciclos: 0,
        };
        while !self.emu.estado().detenida() {
            if r.instrucciones >= limite {
                return Err(ErrorEjecucion::LimiteDeInstrucciones { limite });
            }
            if let Some(p) = self.paso()? {
                r.instrucciones += 1;
                r.ciclos += p.ciclos;
            }
        }
        Ok(r)
    }

    /// Deja que la unidad termine lo que tiene en la cola, sin la CPU. Devuelve los ciclos de la
    /// unidad que hicieron falta.
    pub fn terminar_gpu(&mut self, limite: u64) -> u64 {
        let g = self.gpu.vaciar(limite);
        if self.gpu.tomar_lotes_terminados() > 0 {
            self.lote = true;
        }
        self.publicar_estado();
        g
    }
}
