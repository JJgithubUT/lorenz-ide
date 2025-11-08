//! Avisos de errores de programa que la maquina no detecta por si sola: desbordar la pila y leer
//! lo que nadie ha escrito, sea un registro o una celda de memoria (RF-DEP-039, RF-DEP-040,
//! RF-DEP-041).
//!
//! Se anotan en la lista de avisos del depurador, que las ordenes muestran al parar; no detienen
//! la ejecucion. Cada aviso se da una sola vez por causa.

use std::collections::BTreeSet;

use lorenz_arch::modelo::SentidoPila;
use lorenz_emu::Paso;

use crate::depurador::Depurador;

/// Estado de la vigilancia mientras esta activa.
#[derive(Debug, Clone)]
pub(crate) struct Vigilancia {
    /// Lecturas de memoria sin inicializar ya revisadas.
    memoria_vistas: usize,
    /// Avisos ya dados, para no repetirlos.
    dados: BTreeSet<String>,
    pila: Option<PilaVigilada>,
}

#[derive(Debug, Clone)]
struct PilaVigilada {
    /// Valor del puntero de pila al activar la vigilancia: el fondo de la pila.
    base: u64,
    region: String,
    desde: u64,
    hasta: u64,
    /// Si el puntero esta ahora mismo fuera de su sitio; solo se avisa al salir, no en cada paso.
    fuera: bool,
}

impl Depurador {
    /// Activa o desactiva los avisos (RF-DEP-039, RF-DEP-040, RF-DEP-041).
    ///
    /// Al activarlos, el puntero de pila actual se toma como el fondo de la pila, y los registros
    /// de uso general y las celdas que valen cero cuentan como sin escribir.
    pub fn avisar(&mut self, activos: bool) {
        if !activos {
            self.vigilancia = None;
            return;
        }
        self.emu.memoria_mut().activar_seguimiento();
        self.emu.estado_mut().activar_seguimiento();
        let pila = self.emu.estado().sp().and_then(|sp| {
            let r = self.maquina.region_de(sp)?;
            Some(PilaVigilada {
                base: sp,
                region: r.nombre.clone(),
                desde: r.desde,
                hasta: r.hasta,
                fuera: false,
            })
        });
        self.vigilancia = Some(Vigilancia {
            memoria_vistas: self.emu.memoria().lecturas_sin_inicializar().len(),
            dados: BTreeSet::new(),
            pila,
        });
    }

    pub fn avisos_activos(&self) -> bool {
        self.vigilancia.is_some()
    }

    /// Mira lo que ocurrio en la instruccion `paso` y anota los avisos que correspondan.
    pub(crate) fn revisar_avisos(&mut self, paso: &Paso) {
        let Some(mut v) = self.vigilancia.take() else {
            return;
        };
        let mut nuevos = Vec::new();

        let lecturas = self.emu.memoria().lecturas_sin_inicializar();
        if lecturas.len() < v.memoria_vistas {
            v.memoria_vistas = lecturas.len();
        }
        for l in &lecturas[v.memoria_vistas..] {
            nuevos.push(format!(
                "la instruccion en {:#x} leyo la celda {:#x}, que nadie habia escrito",
                l.pc, l.direccion
            ));
        }
        v.memoria_vistas = lecturas.len();

        for l in self.emu.estado_mut().tomar_lecturas_sin_inicializar() {
            nuevos.push(format!(
                "la instruccion en {:#x} leyo el registro {}, que nadie habia escrito",
                l.pc, l.registro
            ));
        }

        if let (Some(p), Some(sp)) = (&mut v.pila, self.emu.estado().sp()) {
            let en_su_region = sp >= p.desde && sp <= p.hasta;
            let vaciada = match self.maquina.pila {
                SentidoPila::Descendente => sp > p.base,
                SentidoPila::Ascendente => sp < p.base,
            };
            let fuera = !en_su_region || vaciada;
            if fuera && !p.fuera {
                nuevos.push(if vaciada {
                    format!(
                        "la pila se vacio por debajo de su base: tras la instruccion en {:#x}, SP={sp:#x} pasa de {:#x}",
                        paso.direccion, p.base
                    )
                } else {
                    format!(
                        "desbordamiento de pila: tras la instruccion en {:#x}, SP={sp:#x} salio de la region `{}` ({:#x}..{:#x})",
                        paso.direccion, p.region, p.desde, p.hasta
                    )
                });
            }
            p.fuera = fuera;
        }

        for aviso in nuevos {
            if v.dados.insert(aviso.clone()) {
                self.avisos.push(aviso);
            }
        }
        self.vigilancia = Some(v);
    }
}
