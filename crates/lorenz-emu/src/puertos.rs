//! Puertos de entrada y salida declarados por los dispositivos de la maquina.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use lorenz_arch::modelo::{Maquina, TipoDispositivo};
use lorenz_arch::semantica::Programa;

use crate::error::ErrorEjecucion;
use crate::estado::mascara_de;

/// Papel de un puerto dentro de un dispositivo de bloques.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Papel {
    /// Escribir el numero de bloque lo selecciona y pone el cursor al principio.
    Bloque,
    /// Leer o escribir la siguiente palabra del bloque seleccionado.
    Datos,
    /// Bit 0: listo (siempre). Bit 1: el ultimo acceso fue erroneo.
    Estado,
}

/// Contenido y posicion de un dispositivo de almacenamiento por bloques (RF-EMU-034).
#[derive(Debug, Clone)]
struct Almacen {
    nombre: String,
    tamano: usize,
    cantidad: usize,
    palabras: Vec<u64>,
    bloque: usize,
    cursor: usize,
    error: bool,
}

/// Dispositivo generico: su script y los puertos que declara.
#[derive(Debug, Clone)]
struct Script {
    nombre: String,
    programa: Arc<Programa>,
    nombres: BTreeMap<String, u64>,
    linea: Option<u32>,
}

#[derive(Debug, Clone, Default)]
struct Puerto {
    /// Dispositivo generico al que pertenece: su script corre al escribir en este puerto.
    script: Option<usize>,
    /// Dispositivo de bloques al que pertenece y que papel juega en el.
    almacen: Option<(usize, Papel)>,
    entrada: VecDeque<u64>,
    salida: Vec<u64>,
    /// Si es el puerto de estado de un teclado o una UART: el puerto de datos del que informa y
    /// los bits que siempre estan encendidos.
    estado_de: Option<(u64, u64)>,
    /// Valor que un dispositivo publica en el puerto; leerlo no lo consume.
    publicado: Option<u64>,
}

/// Colas de entrada y registro de salida de cada puerto.
///
/// Solo existen los puertos que algun dispositivo declara; tocar otro es un error
/// (RF-EMU-030). Una lectura con la cola vacia devuelve 0, como un puerto sin dato.
#[derive(Debug, Clone)]
pub struct Puertos {
    puertos: BTreeMap<u64, Puerto>,
    almacenes: Vec<Almacen>,
    scripts: Vec<Script>,
    /// Lineas de interrupcion que los scripts pidieron y el emulador aun no recogio.
    pedidas: Vec<u32>,
    mascara: u64,
}

impl Puertos {
    pub fn nuevo(maquina: &Maquina) -> Self {
        let mut puertos = BTreeMap::new();
        let mut almacenes = Vec::new();
        let mut scripts = Vec::new();
        for d in &maquina.dispositivos {
            for numero in d.puertos.values() {
                puertos.insert(*numero, Puerto::default());
            }
            if let TipoDispositivo::Bloques {
                tamano_bloque,
                cantidad_bloques,
            } = d.tipo
            {
                let indice = almacenes.len();
                let (tamano, cantidad) = (tamano_bloque as usize, cantidad_bloques as usize);
                almacenes.push(Almacen {
                    nombre: d.nombre.clone(),
                    tamano,
                    cantidad,
                    palabras: vec![0; tamano * cantidad],
                    bloque: 0,
                    cursor: 0,
                    error: false,
                });
                for (nombre, papel) in [
                    ("bloque", Papel::Bloque),
                    ("datos", Papel::Datos),
                    ("estado", Papel::Estado),
                ] {
                    if let Some(p) = d.puertos.get(nombre).and_then(|n| puertos.get_mut(n)) {
                        p.almacen = Some((indice, papel));
                    }
                }
            }
            if let TipoDispositivo::Generico { programa, .. } = &d.tipo {
                let indice = scripts.len();
                scripts.push(Script {
                    nombre: d.nombre.clone(),
                    programa: Arc::new(programa.clone()),
                    nombres: d.puertos.clone(),
                    linea: d.interrupcion,
                });
                for numero in d.puertos.values() {
                    if let Some(p) = puertos.get_mut(numero) {
                        p.script = Some(indice);
                        p.publicado = Some(0);
                    }
                }
            }
            // Un teclado informa por `estado` de si hay una tecla esperando en `datos`; una UART
            // ademas tiene siempre encendido el bit 1 (lista para transmitir).
            let fijos = match d.tipo {
                TipoDispositivo::Teclado => Some(0),
                TipoDispositivo::Uart => Some(2),
                _ => None,
            };
            if let (Some(fijos), Some(datos), Some(estado)) =
                (fijos, d.puertos.get("datos"), d.puertos.get("estado"))
            {
                if let Some(p) = puertos.get_mut(estado) {
                    p.estado_de = Some((*datos, fijos));
                }
            }
        }
        Puertos {
            puertos,
            almacenes,
            scripts,
            pedidas: Vec::new(),
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
    /// puerto de datos y 0 si no. El de una UART es igual, con el bit 1 siempre encendido.
    pub fn leer(&mut self, numero: u64) -> Result<u64, ErrorEjecucion> {
        if let Some((i, papel)) = self.puerto(numero)?.almacen {
            return Ok(self.leer_almacen(i, papel));
        }
        if let Some(valor) = self.puerto(numero)?.publicado {
            return Ok(valor);
        }
        if let Some((datos, fijos)) = self.puerto(numero)?.estado_de {
            let hay = self
                .puertos
                .get(&datos)
                .is_some_and(|p| !p.entrada.is_empty());
            return Ok(u64::from(hay) | fijos);
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
        if let Some((i, papel)) = self.puerto(numero)?.almacen {
            self.escribir_almacen(i, papel, valor & mascara);
            return Ok(());
        }
        if let Some(i) = self.puerto(numero)?.script {
            return self.ejecutar_script(i, numero, valor & mascara);
        }
        self.puerto(numero)?.salida.push(valor & mascara);
        Ok(())
    }

    /// Todo lo escrito en un puerto, en orden.
    pub fn salida(&mut self, numero: u64) -> Result<&[u64], ErrorEjecucion> {
        Ok(&self.puerto(numero)?.salida)
    }

    /// Un dispositivo deja un valor en el puerto; cada lectura lo devuelve sin consumirlo.
    pub fn publicar(&mut self, numero: u64, valor: u64) -> Result<(), ErrorEjecucion> {
        let mascara = self.mascara;
        self.puerto(numero)?.publicado = Some(valor & mascara);
        Ok(())
    }

    /// Corre el script del dispositivo `i` porque el programa escribio `valor` en `numero`.
    fn ejecutar_script(&mut self, i: usize, numero: u64, valor: u64) -> Result<(), ErrorEjecucion> {
        let script = self.scripts[i].clone();
        let mut valores: BTreeMap<u64, u64> = script
            .nombres
            .values()
            .map(|n| {
                (
                    *n,
                    self.puertos.get(n).and_then(|p| p.publicado).unwrap_or(0),
                )
            })
            .collect();
        let mut entorno = crate::script::Entorno {
            nombres: &script.nombres,
            valores: &mut valores,
            valor,
            puerto: numero,
            mascara: self.mascara,
            interrumpir: false,
            dispositivo: &script.nombre,
        };
        crate::script::ejecutar(&script.programa, &mut entorno)?;
        let interrumpir = entorno.interrumpir;
        for (n, v) in valores {
            if let Some(p) = self.puertos.get_mut(&n) {
                p.publicado = Some(v);
            }
        }
        if interrumpir {
            self.pedidas.extend(script.linea);
        }
        Ok(())
    }

    /// Lineas de interrupcion pedidas por scripts desde la ultima vez, en orden.
    pub fn tomar_interrupciones(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.pedidas)
    }

    fn leer_almacen(&mut self, i: usize, papel: Papel) -> u64 {
        let a = &mut self.almacenes[i];
        match papel {
            Papel::Bloque => a.bloque as u64,
            Papel::Estado => 1 | (u64::from(a.error) << 1),
            Papel::Datos => {
                if a.cursor >= a.tamano {
                    a.error = true;
                    return 0;
                }
                let v = a.palabras[a.bloque * a.tamano + a.cursor];
                a.cursor += 1;
                v
            }
        }
    }

    fn escribir_almacen(&mut self, i: usize, papel: Papel, valor: u64) {
        let a = &mut self.almacenes[i];
        match papel {
            Papel::Estado => {}
            Papel::Bloque => {
                a.cursor = 0;
                a.error = valor >= a.cantidad as u64;
                if !a.error {
                    a.bloque = valor as usize;
                }
            }
            Papel::Datos => {
                if a.error || a.cursor >= a.tamano {
                    a.error = true;
                    return;
                }
                a.palabras[a.bloque * a.tamano + a.cursor] = valor;
                a.cursor += 1;
            }
        }
    }

    fn almacen(&mut self, nombre: &str) -> Result<&mut Almacen, ErrorEjecucion> {
        self.almacenes
            .iter_mut()
            .find(|a| a.nombre == nombre)
            .ok_or_else(|| ErrorEjecucion::DispositivoInexistente {
                nombre: nombre.to_string(),
            })
    }

    /// Deja el contenido de un bloque desde fuera de la maquina (por ejemplo, una imagen de disco).
    /// Lo que falte hasta el final del bloque se rellena con ceros.
    pub fn cargar_bloque(
        &mut self,
        dispositivo: &str,
        bloque: usize,
        palabras: &[u64],
    ) -> Result<(), ErrorEjecucion> {
        let mascara = self.mascara;
        let a = self.almacen(dispositivo)?;
        if bloque >= a.cantidad || palabras.len() > a.tamano {
            return Err(ErrorEjecucion::BloqueFueraDeRango {
                dispositivo: dispositivo.to_string(),
                bloque,
            });
        }
        let base = bloque * a.tamano;
        for (k, celda) in a.palabras[base..base + a.tamano].iter_mut().enumerate() {
            *celda = palabras.get(k).map_or(0, |v| v & mascara);
        }
        Ok(())
    }

    /// Contenido actual de un bloque.
    pub fn leer_bloque(
        &mut self,
        dispositivo: &str,
        bloque: usize,
    ) -> Result<Vec<u64>, ErrorEjecucion> {
        let a = self.almacen(dispositivo)?;
        if bloque >= a.cantidad {
            return Err(ErrorEjecucion::BloqueFueraDeRango {
                dispositivo: dispositivo.to_string(),
                bloque,
            });
        }
        let base = bloque * a.tamano;
        Ok(a.palabras[base..base + a.tamano].to_vec())
    }

    /// Vacia todas las colas y registros. El contenido de los discos se conserva.
    pub fn reiniciar(&mut self) {
        self.pedidas.clear();
        for a in &mut self.almacenes {
            a.bloque = 0;
            a.cursor = 0;
            a.error = false;
        }
        for p in self.puertos.values_mut() {
            p.entrada.clear();
            p.salida.clear();
            if p.publicado.is_some() {
                p.publicado = Some(0);
            }
        }
    }
}
