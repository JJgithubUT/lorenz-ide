//! El servidor: acepta conexiones, las atiende en hilos acotados y se cierra de forma ordenada.

use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::peticion::{leer, ErrorPeticion};
use crate::{Respuesta, Router};

#[derive(Debug, Clone)]
pub struct Limites {
    /// Tamano maximo del cuerpo de una peticion.
    pub cuerpo_max: usize,
    /// Tamano maximo de la linea de peticion y las cabeceras juntas.
    pub cabeceras_max: usize,
    /// Conexiones atendidas a la vez; las demas reciben 503.
    pub conexiones_max: usize,
    /// Plazo de lectura y de escritura de cada conexion.
    pub espera: Duration,
    /// Si es `true`, solo se aceptan peticiones cuyo `Host` sea `localhost`, `127.0.0.1` o `[::1]`
    /// (protege a un servidor local de paginas que lo ataquen desde el navegador).
    pub solo_locales: bool,
}

impl Default for Limites {
    fn default() -> Self {
        Self {
            cuerpo_max: 4 * 1024 * 1024,
            cabeceras_max: 16 * 1024,
            conexiones_max: 64,
            espera: Duration::from_secs(10),
            solo_locales: true,
        }
    }
}

/// Un servidor en marcha.
pub struct Servidor {
    direccion: SocketAddr,
    parada: Arc<AtomicBool>,
    hilo: Option<JoinHandle<()>>,
}

fn host_local(host: &str) -> bool {
    let h = host.trim();
    let nombre = if let Some(r) = h.strip_prefix('[') {
        r.split(']').next().unwrap_or("")
    } else {
        h.rsplit_once(':').map_or(h, |(n, _)| n)
    };
    matches!(
        nombre.to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "::1"
    )
}

fn atender(mut flujo: TcpStream, router: &Router, limites: &Limites) {
    let _ = flujo.set_read_timeout(Some(limites.espera));
    let _ = flujo.set_write_timeout(Some(limites.espera));
    let respuesta = match leer(&mut flujo, limites) {
        Ok(p) => {
            if limites.solo_locales && !p.cabecera("host").is_some_and(host_local) {
                Respuesta::texto(403, "host no permitido")
            } else {
                catch_unwind(AssertUnwindSafe(|| router.despachar(&p)))
                    .unwrap_or_else(|_| Respuesta::texto(500, "error interno"))
            }
        }
        Err(ErrorPeticion::Vacia) => return,
        Err(ErrorPeticion::Mal(m)) => Respuesta::texto(400, &m),
        Err(ErrorPeticion::CabecerasGrandes) => {
            Respuesta::texto(400, "cabeceras demasiado grandes")
        }
        Err(ErrorPeticion::CuerpoGrande) => Respuesta::texto(413, "cuerpo demasiado grande"),
        Err(ErrorPeticion::SinLongitud) => Respuesta::texto(411, "falta Content-Length"),
        Err(ErrorPeticion::NoSoportado(q)) => {
            Respuesta::texto(501, &format!("{q} no esta soportado"))
        }
        Err(ErrorPeticion::Espera) => Respuesta::texto(408, "tiempo agotado"),
    };
    let _ = flujo.write_all(&respuesta.a_bytes());
    let _ = flujo.flush();
    let _ = flujo.shutdown(std::net::Shutdown::Both);
}

impl Servidor {
    /// Empieza a escuchar en `direccion` (por ejemplo `127.0.0.1:0` para un puerto libre).
    pub fn iniciar(direccion: &str, router: Router, limites: Limites) -> std::io::Result<Self> {
        let escucha = TcpListener::bind(direccion)?;
        let local = escucha.local_addr()?;
        let parada = Arc::new(AtomicBool::new(false));
        let router = Arc::new(router);
        let limites = Arc::new(limites);
        let activas = Arc::new(AtomicUsize::new(0));
        let p = Arc::clone(&parada);
        let hilo = std::thread::spawn(move || {
            let mut hijos: Vec<JoinHandle<()>> = Vec::new();
            for flujo in escucha.incoming() {
                if p.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(mut flujo) = flujo else { continue };
                hijos.retain(|h| !h.is_finished());
                if activas.load(Ordering::SeqCst) >= limites.conexiones_max {
                    let _ = flujo.write_all(&Respuesta::texto(503, "servidor ocupado").a_bytes());
                    continue;
                }
                activas.fetch_add(1, Ordering::SeqCst);
                let (r, l, a) = (
                    Arc::clone(&router),
                    Arc::clone(&limites),
                    Arc::clone(&activas),
                );
                hijos.push(std::thread::spawn(move || {
                    atender(flujo, &r, &l);
                    a.fetch_sub(1, Ordering::SeqCst);
                }));
            }
            for h in hijos {
                let _ = h.join();
            }
        });
        Ok(Self {
            direccion: local,
            parada,
            hilo: Some(hilo),
        })
    }

    pub fn direccion(&self) -> SocketAddr {
        self.direccion
    }

    /// Deja de aceptar conexiones y espera a que terminen las que estan en curso.
    pub fn detener(mut self) {
        self.cerrar();
    }

    fn cerrar(&mut self) {
        self.parada.store(true, Ordering::SeqCst);
        // `accept` esta bloqueado: una conexion propia lo despierta.
        let _ = TcpStream::connect_timeout(&self.direccion, Duration::from_secs(1));
        if let Some(h) = self.hilo.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Servidor {
    fn drop(&mut self) {
        self.cerrar();
    }
}
