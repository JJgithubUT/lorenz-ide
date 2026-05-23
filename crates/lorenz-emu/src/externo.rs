//! Dispositivos externos: un proceso o un servidor TCP al que el emulador le pasa las lecturas y
//! escrituras de sus puertos (RF-ECO-037). El protocolo esta en `docs/EXTERNOS.md`.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Cuanto espera el emulador una respuesta antes de dar el dispositivo por muerto.
pub const ESPERA: Duration = Duration::from_secs(2);

static PROHIBIDOS: AtomicBool = AtomicBool::new(false);

/// En modo seguro no se lanza ningun proceso ni se abre ninguna conexion. Es una opcion de todo el
/// proceso: quien la activa debe devolverla a su valor al terminar.
pub fn prohibir(prohibidos: bool) {
    PROHIBIDOS.store(prohibidos, Ordering::SeqCst);
}

/// Si los dispositivos externos estan prohibidos ahora mismo.
pub fn estan_prohibidos() -> bool {
    PROHIBIDOS.load(Ordering::SeqCst)
}

enum Canal {
    Proceso {
        hijo: Child,
        entrada: ChildStdin,
        lineas: Receiver<String>,
    },
    Tcp {
        escritura: TcpStream,
        lineas: Receiver<String>,
    },
}

impl Drop for Canal {
    fn drop(&mut self) {
        if let Canal::Proceso { hijo, entrada, .. } = self {
            let _ = writeln!(entrada, "FIN");
            let _ = hijo.kill();
            let _ = hijo.wait();
        }
        if let Canal::Tcp { escritura, .. } = self {
            let _ = writeln!(escritura, "FIN");
        }
    }
}

fn leer_en_hilo<R: std::io::Read + Send + 'static>(origen: R) -> Receiver<String> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        for linea in BufReader::new(origen).lines() {
            let Ok(l) = linea else { break };
            if tx.send(l).is_err() {
                break;
            }
        }
    });
    rx
}

/// Un dispositivo externo. La conexion se abre al primer acceso y se comparte entre las copias del
/// emulador que la hereden.
#[derive(Clone)]
pub struct Externo {
    pub nombre: String,
    orden: Option<String>,
    direccion: Option<String>,
    canal: Arc<Mutex<Option<Canal>>>,
}

impl std::fmt::Debug for Externo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Externo")
            .field("nombre", &self.nombre)
            .finish_non_exhaustive()
    }
}

impl Externo {
    pub fn nuevo(nombre: &str, orden: Option<String>, direccion: Option<String>) -> Self {
        Self {
            nombre: nombre.into(),
            orden,
            direccion,
            canal: Arc::new(Mutex::new(None)),
        }
    }

    fn abrir(&self) -> Result<Canal, String> {
        if estan_prohibidos() {
            return Err("los dispositivos externos estan prohibidos en modo seguro".into());
        }
        if let Some(orden) = &self.orden {
            let mut partes = orden.split_whitespace();
            let programa = partes.next().ok_or("la orden esta vacia")?;
            let mut hijo = Command::new(programa)
                .args(partes)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("no se pudo lanzar `{orden}`: {e}"))?;
            let entrada = hijo.stdin.take().ok_or("sin entrada estandar")?;
            let salida = hijo.stdout.take().ok_or("sin salida estandar")?;
            return Ok(Canal::Proceso {
                hijo,
                entrada,
                lineas: leer_en_hilo(salida),
            });
        }
        let direccion = self.direccion.as_deref().ok_or("sin orden ni direccion")?;
        let destino = direccion
            .to_socket_addrs()
            .map_err(|e| format!("direccion `{direccion}` invalida: {e}"))?
            .next()
            .ok_or_else(|| format!("la direccion `{direccion}` no se resuelve"))?;
        let flujo = TcpStream::connect_timeout(&destino, ESPERA)
            .map_err(|e| format!("no se pudo conectar con {direccion}: {e}"))?;
        let _ = flujo.set_nodelay(true);
        let lectura = flujo.try_clone().map_err(|e| e.to_string())?;
        Ok(Canal::Tcp {
            escritura: flujo,
            lineas: leer_en_hilo(lectura),
        })
    }

    /// Manda una linea y devuelve la respuesta.
    fn pedir(&self, linea: &str) -> Result<String, String> {
        let mut guardia = self
            .canal
            .lock()
            .map_err(|_| "canal envenenado".to_string())?;
        if guardia.is_none() {
            *guardia = Some(self.abrir()?);
        }
        let canal = guardia.as_mut().ok_or("sin canal")?;
        let (envio, lineas) = match canal {
            Canal::Proceso {
                entrada, lineas, ..
            } => (
                writeln!(entrada, "{linea}").and_then(|()| entrada.flush()),
                &*lineas,
            ),
            Canal::Tcp { escritura, lineas } => (
                writeln!(escritura, "{linea}").and_then(|()| escritura.flush()),
                &*lineas,
            ),
        };
        if let Err(e) = envio {
            *guardia = None;
            return Err(format!("no se pudo escribir: {e}"));
        }
        match lineas.recv_timeout(ESPERA) {
            Ok(r) => Ok(r.trim().to_string()),
            Err(RecvTimeoutError::Timeout) => {
                *guardia = None;
                Err(format!("no respondio en {} ms", ESPERA.as_millis()))
            }
            Err(RecvTimeoutError::Disconnected) => {
                *guardia = None;
                Err("cerro la conexion".into())
            }
        }
    }

    pub fn leer(&self, puerto: u64) -> Result<u64, String> {
        let r = self.pedir(&format!("LEER {puerto}"))?;
        if let Some(e) = r.strip_prefix("ERR") {
            return Err(e.trim().to_string());
        }
        let n = r.strip_prefix("OK ").unwrap_or(&r).trim();
        match n.strip_prefix("0x").or_else(|| n.strip_prefix("0X")) {
            Some(h) => u64::from_str_radix(h, 16),
            None => n.parse(),
        }
        .map_err(|_| format!("respuesta `{r}` a LEER no es un numero"))
    }

    pub fn escribir(&self, puerto: u64, valor: u64) -> Result<(), String> {
        let r = self.pedir(&format!("ESCRIBIR {puerto} {valor}"))?;
        if let Some(e) = r.strip_prefix("ERR") {
            return Err(e.trim().to_string());
        }
        if r.starts_with("OK") {
            Ok(())
        } else {
            Err(format!("respuesta `{r}` a ESCRIBIR; se esperaba OK"))
        }
    }
}
