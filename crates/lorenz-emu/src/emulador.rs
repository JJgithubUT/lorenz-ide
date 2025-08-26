//! El emulador completo: junta memoria, estado, puertos, decodificador e interprete y ejecuta
//! programas instruccion por instruccion.

use std::collections::BTreeSet;

use lorenz_arch::modelo::{Ciclos, Endianness, Maquina, SentidoPila, TipoDispositivo};

use crate::decodificador::Decodificador;
use crate::error::ErrorEjecucion;
use crate::estado::Estado;
use crate::interprete::{Entorno, Interprete};
use crate::memoria::Memoria;
use crate::pantalla::{leer_imagen, Imagen};
use crate::puertos::Puertos;

/// Que hizo una instruccion al ejecutarse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paso {
    /// Direccion donde estaba la instruccion.
    pub direccion: u64,
    pub mnemonico: String,
    /// Palabras que ocupa.
    pub longitud: usize,
    /// Ciclos que costo.
    pub ciclos: u64,
    /// Si asigno el contador de programa.
    pub salto: bool,
    /// Si detuvo la maquina.
    pub detenida: bool,
}

/// Resumen de una ejecucion hasta detenerse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resumen {
    /// Instrucciones ejecutadas en esta llamada.
    pub instrucciones: u64,
    /// Ciclos consumidos en esta llamada.
    pub ciclos: u64,
}

/// Estado de un temporizador: cuantos ciclos lleva acumulados y cuantos periodos completo.
#[derive(Debug, Clone)]
struct Temporizador {
    periodo: u64,
    acumulado: u64,
    ticks: u64,
    linea: Option<u32>,
    /// Puerto `cuenta`, donde se publica el numero de periodos completados.
    puerto_cuenta: Option<u64>,
}

/// Una maquina emulada lista para ejecutar.
#[derive(Debug, Clone)]
pub struct Emulador {
    maquina: Maquina,
    decodificador: Decodificador,
    interprete: Interprete,
    estado: Estado,
    memoria: Memoria,
    puertos: Puertos,
    pendientes: BTreeSet<u32>,
    temporizadores: Vec<Temporizador>,
}

impl Emulador {
    pub fn nuevo(maquina: Maquina) -> Result<Self, ErrorEjecucion> {
        let memoria = Memoria::nueva(&maquina)?;
        let temporizadores = maquina
            .dispositivos
            .iter()
            .filter_map(|d| match d.tipo {
                TipoDispositivo::Temporizador { periodo } => Some(Temporizador {
                    periodo,
                    acumulado: 0,
                    ticks: 0,
                    linea: d.interrupcion,
                    puerto_cuenta: d.puertos.get("cuenta").copied(),
                }),
                _ => None,
            })
            .collect();
        Ok(Emulador {
            temporizadores,
            decodificador: Decodificador::nuevo(&maquina),
            interprete: Interprete::nuevo(&maquina),
            estado: Estado::nuevo(&maquina),
            puertos: Puertos::nuevo(&maquina),
            memoria,
            pendientes: BTreeSet::new(),
            maquina,
        })
    }

    pub fn maquina(&self) -> &Maquina {
        &self.maquina
    }

    pub fn estado(&self) -> &Estado {
        &self.estado
    }

    pub fn estado_mut(&mut self) -> &mut Estado {
        &mut self.estado
    }

    pub fn memoria(&self) -> &Memoria {
        &self.memoria
    }

    pub fn memoria_mut(&mut self) -> &mut Memoria {
        &mut self.memoria
    }

    pub fn puertos(&self) -> &Puertos {
        &self.puertos
    }

    pub fn puertos_mut(&mut self) -> &mut Puertos {
        &mut self.puertos
    }

    /// Copia un programa a memoria sin mirar los permisos, por ejemplo dentro de una rom.
    pub fn cargar_programa(
        &mut self,
        direccion: u64,
        palabras: &[u64],
    ) -> Result<(), ErrorEjecucion> {
        self.memoria.cargar_bloque(direccion, palabras)
    }

    /// Vuelve registros, contadores y puertos a su estado inicial. La memoria no se toca.
    pub fn reiniciar(&mut self) {
        self.estado.reiniciar();
        self.puertos.reiniciar();
        self.pendientes.clear();
        for t in &mut self.temporizadores {
            t.acumulado = 0;
            t.ticks = 0;
        }
    }

    /// Imagen actual de la pantalla `nombre` (RF-EMU-036).
    pub fn pantalla(&self, nombre: &str) -> Result<Imagen, ErrorEjecucion> {
        leer_imagen(&self.maquina, &self.memoria, nombre)
    }

    /// Pulsa una tecla en el teclado `nombre`: deja el codigo en su puerto de datos y, si el
    /// teclado declara una linea de interrupcion, la levanta (RF-EMU-035).
    pub fn pulsar_tecla(&mut self, nombre: &str, codigo: u64) -> Result<(), ErrorEjecucion> {
        let dispositivo = self.maquina.dispositivo(nombre).ok_or_else(|| {
            ErrorEjecucion::DispositivoInexistente {
                nombre: nombre.to_string(),
            }
        })?;
        let incorrecto = || ErrorEjecucion::DispositivoIncorrecto {
            nombre: nombre.to_string(),
            esperado: "un teclado",
        };
        if dispositivo.tipo != TipoDispositivo::Teclado {
            return Err(incorrecto());
        }
        let datos = *dispositivo.puertos.get("datos").ok_or_else(incorrecto)?;
        let linea = dispositivo.interrupcion;
        self.puertos.encolar_entrada(datos, codigo)?;
        if let Some(linea) = linea {
            self.levantar_interrupcion(linea)?;
        }
        Ok(())
    }

    fn puerto_datos_de(
        &self,
        nombre: &str,
        tipo: TipoDispositivo,
        esperado: &'static str,
    ) -> Result<(u64, Option<u32>), ErrorEjecucion> {
        let dispositivo = self.maquina.dispositivo(nombre).ok_or_else(|| {
            ErrorEjecucion::DispositivoInexistente {
                nombre: nombre.to_string(),
            }
        })?;
        let incorrecto = || ErrorEjecucion::DispositivoIncorrecto {
            nombre: nombre.to_string(),
            esperado,
        };
        if dispositivo.tipo != tipo {
            return Err(incorrecto());
        }
        let datos = *dispositivo.puertos.get("datos").ok_or_else(incorrecto)?;
        Ok((datos, dispositivo.interrupcion))
    }

    /// Entrega bytes a la UART `nombre` como si llegaran por su linea serie. Si la UART declara una
    /// linea de interrupcion, la levanta (RF-EMU-039).
    pub fn recibir_en_uart(&mut self, nombre: &str, bytes: &[u8]) -> Result<(), ErrorEjecucion> {
        let (datos, linea) = self.puerto_datos_de(nombre, TipoDispositivo::Uart, "una UART")?;
        for b in bytes {
            self.puertos.encolar_entrada(datos, u64::from(*b))?;
        }
        if let (Some(linea), false) = (linea, bytes.is_empty()) {
            self.levantar_interrupcion(linea)?;
        }
        Ok(())
    }

    /// Todo lo que el programa transmitio por la UART `nombre`, como texto (RF-EMU-039).
    ///
    /// Cada palabra escrita en el puerto de datos es un byte; los que no son UTF-8 valido se
    /// sustituyen por el caracter de reemplazo.
    pub fn texto_de_uart(&mut self, nombre: &str) -> Result<String, ErrorEjecucion> {
        let (datos, _) = self.puerto_datos_de(nombre, TipoDispositivo::Uart, "una UART")?;
        let bytes: Vec<u8> = self
            .puertos
            .salida(datos)?
            .iter()
            .map(|v| (*v & 0xFF) as u8)
            .collect();
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Levanta una linea de interrupcion. Se atiende antes de la siguiente instruccion si las
    /// interrupciones estan habilitadas; mientras tanto queda pendiente.
    ///
    /// Si varias lineas esperan, se atiende primero la de numero menor (D-019).
    pub fn levantar_interrupcion(&mut self, linea: u32) -> Result<(), ErrorEjecucion> {
        let declarada = self
            .maquina
            .interrupciones
            .as_ref()
            .is_some_and(|i| i.vectores.contains_key(&linea));
        if !declarada {
            return Err(ErrorEjecucion::InterrupcionNoDeclarada { linea });
        }
        self.pendientes.insert(linea);
        Ok(())
    }

    /// Lineas de interrupcion que esperan ser atendidas, de menor a mayor.
    pub fn interrupciones_pendientes(&self) -> Vec<u32> {
        self.pendientes.iter().copied().collect()
    }

    /// Palabras que hacen falta para guardar un valor de `bits` bits.
    fn palabras_para(&self, bits: u32) -> u32 {
        bits.div_ceil(self.maquina.bits_palabra)
    }

    /// Escribe `valor` en `palabras` palabras consecutivas desde `direccion`, en el orden del
    /// `endianness` de la maquina.
    fn escribir_valor(
        &mut self,
        direccion: u64,
        valor: u64,
        palabras: u32,
    ) -> Result<(), ErrorEjecucion> {
        let bw = self.maquina.bits_palabra;
        for i in 0..palabras {
            let posicion = match self.maquina.endianness {
                Endianness::Little => i,
                Endianness::Big => palabras - 1 - i,
            };
            let palabra = valor.checked_shr(posicion * bw).unwrap_or(0);
            self.memoria
                .escribir(direccion.wrapping_add(u64::from(i)), palabra)?;
        }
        Ok(())
    }

    /// Lee un valor de `palabras` palabras consecutivas desde `direccion`.
    fn leer_valor(&self, direccion: u64, palabras: u32) -> Result<u64, ErrorEjecucion> {
        let bw = self.maquina.bits_palabra;
        let mut valor = 0u64;
        for i in 0..palabras {
            let palabra = self.memoria.leer(direccion.wrapping_add(u64::from(i)))?;
            let posicion = match self.maquina.endianness {
                Endianness::Little => i,
                Endianness::Big => palabras - 1 - i,
            };
            valor |= palabra.checked_shl(posicion * bw).unwrap_or(0);
        }
        Ok(valor)
    }

    /// Suma `ciclos` a cada temporizador: por cada periodo completo cuenta un tick, lo publica en
    /// su puerto `cuenta` y levanta su linea de interrupcion (RF-EMU-037).
    fn avanzar_temporizadores(&mut self, ciclos: u64) -> Result<(), ErrorEjecucion> {
        let mut lineas = Vec::new();
        for t in &mut self.temporizadores {
            t.acumulado += ciclos;
            let completos = t.acumulado / t.periodo;
            if completos == 0 {
                continue;
            }
            t.acumulado %= t.periodo;
            t.ticks = t.ticks.wrapping_add(completos);
            if let Some(puerto) = t.puerto_cuenta {
                self.puertos.publicar(puerto, t.ticks)?;
            }
            lineas.extend(t.linea);
        }
        for linea in lineas {
            self.levantar_interrupcion(linea)?;
        }
        Ok(())
    }

    /// Periodos completos que lleva contados el temporizador `nombre`.
    pub fn ticks_de(&self, nombre: &str) -> Result<u64, ErrorEjecucion> {
        let posicion = self
            .maquina
            .dispositivos
            .iter()
            .filter(|d| matches!(d.tipo, TipoDispositivo::Temporizador { .. }))
            .position(|d| d.nombre == nombre)
            .ok_or_else(|| ErrorEjecucion::DispositivoInexistente {
                nombre: nombre.to_string(),
            })?;
        Ok(self.temporizadores[posicion].ticks)
    }

    /// Atiende la interrupcion pendiente de menor numero, si hay y estan habilitadas.
    fn atender_interrupcion(&mut self) -> Result<bool, ErrorEjecucion> {
        if !self.estado.interrupciones_habilitadas() {
            return Ok(false);
        }
        let Some(&linea) = self.pendientes.iter().next() else {
            return Ok(false);
        };
        let vector = self
            .maquina
            .interrupciones
            .as_ref()
            .and_then(|i| i.vectores.get(&linea))
            .copied();
        let vector = vector.ok_or(ErrorEjecucion::InterrupcionNoDeclarada { linea })?;
        let sp = self
            .estado
            .sp()
            .ok_or(ErrorEjecucion::InterrupcionSinPila { linea })?;
        let nombre_pc = self.estado.nombre_del_pc().to_string();
        let palabras_pc =
            self.palabras_para(self.estado.mascara_de_registro(&nombre_pc)?.count_ones());
        let palabras_dir = self.palabras_para(self.maquina.bits_direccion);
        let pc = self.estado.pc();
        let destino = self.leer_valor(vector, palabras_dir)?;
        match self.maquina.pila {
            SentidoPila::Descendente => {
                let nuevo = sp.wrapping_sub(u64::from(palabras_pc));
                self.escribir_valor(nuevo, pc, palabras_pc)?;
                self.estado.fijar_sp(nuevo);
            }
            SentidoPila::Ascendente => {
                self.escribir_valor(sp, pc, palabras_pc)?;
                self.estado
                    .fijar_sp(sp.wrapping_add(u64::from(palabras_pc)));
            }
        }
        self.pendientes.remove(&linea);
        self.estado.habilitar_interrupciones(false);
        self.estado.fijar_pc(destino);
        Ok(true)
    }

    /// Ejecuta una instruccion. Devuelve `None` si la maquina ya estaba detenida.
    ///
    /// Si la instruccion falla, el contador de programa vuelve a apuntar a ella para que el
    /// depurador muestre donde ocurrio el error.
    pub fn paso(&mut self) -> Result<Option<Paso>, ErrorEjecucion> {
        if self.estado.detenida() {
            return Ok(None);
        }
        self.atender_interrupcion()?;
        let direccion = self.estado.pc();
        let dec = self.decodificador.decodificar(&self.memoria, direccion)?;
        self.estado
            .fijar_pc(direccion.wrapping_add(dec.longitud as u64));
        let instruccion = &self.maquina.instrucciones[dec.indice];
        let mut entorno = Entorno {
            estado: &mut self.estado,
            memoria: &mut self.memoria,
            puertos: &mut self.puertos,
        };
        let resultado = match self.interprete.ejecutar(instruccion, &dec, &mut entorno) {
            Ok(r) => r,
            Err(e) => {
                self.estado.fijar_pc(direccion);
                return Err(e);
            }
        };
        let ciclos = match instruccion.ciclos {
            Ciclos::Fijo(n) => n,
            Ciclos::SegunSalto { tomado, no_tomado } => {
                if resultado.salto {
                    tomado
                } else {
                    no_tomado
                }
            }
        } as u64;
        let mnemonico = instruccion.mnemonico.clone();
        self.estado.contar_instruccion(ciclos);
        self.avanzar_temporizadores(ciclos)?;
        Ok(Some(Paso {
            direccion,
            mnemonico,
            longitud: dec.longitud,
            ciclos,
            salto: resultado.salto,
            detenida: resultado.detenida,
        }))
    }

    /// Ejecuta como mucho `cantidad` instrucciones y devuelve el control, se haya detenido la
    /// maquina o no (RF-EMU-010). Es lo que usa un depurador para avanzar de a poco.
    pub fn avanzar(&mut self, cantidad: u64) -> Result<Resumen, ErrorEjecucion> {
        let mut resumen = Resumen {
            instrucciones: 0,
            ciclos: 0,
        };
        while resumen.instrucciones < cantidad {
            match self.paso()? {
                Some(paso) => {
                    resumen.instrucciones += 1;
                    resumen.ciclos += paso.ciclos;
                }
                None => break,
            }
        }
        Ok(resumen)
    }

    /// Ejecuta hasta que la maquina se detenga o se agote el limite de instrucciones.
    ///
    /// El limite cuenta las instrucciones de esta llamada. Si se alcanza sin detenerse, devuelve
    /// `LimiteDeInstrucciones` (RF-EMU-017): un programa que no termina no cuelga al usuario.
    pub fn ejecutar(&mut self, limite: u64) -> Result<Resumen, ErrorEjecucion> {
        let mut resumen = Resumen {
            instrucciones: 0,
            ciclos: 0,
        };
        while !self.estado.detenida() {
            if resumen.instrucciones >= limite {
                return Err(ErrorEjecucion::LimiteDeInstrucciones { limite });
            }
            if let Some(paso) = self.paso()? {
                resumen.instrucciones += 1;
                resumen.ciclos += paso.ciclos;
            }
        }
        Ok(resumen)
    }
}
