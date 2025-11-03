//! Explicar una instruccion: que operaciones hace, que registros y banderas cambia, que escribe
//! en memoria y si salta (RF-DEP-032).
//!
//! La explicacion sale de ejecutar la instruccion en una copia de la maquina, asi que no toca la
//! sesion: se puede pedir antes de dar el paso y el resultado es el mismo que dara el paso real.

use std::fmt::Write;

use lorenz_emu::decodificador::Decodificador;
use lorenz_emu::EscrituraRegistrada;

use crate::depurador::Depurador;
use crate::error::ErrorDepurador;
use crate::vistas;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CambioRegistro {
    pub nombre: String,
    pub antes: u64,
    pub despues: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CambioBandera {
    pub nombre: String,
    pub antes: bool,
    pub despues: bool,
}

/// Que hace la instruccion que esta a punto de ejecutarse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explicacion {
    pub direccion: u64,
    /// La instruccion desensamblada.
    pub texto: String,
    pub descripcion: String,
    /// Las operaciones de su semantica, una por elemento, tal como las escribio la arquitectura.
    pub operaciones: Vec<String>,
    /// Operandos con el valor con que se decodificaron.
    pub operandos: Vec<(String, String)>,
    /// Banderas que la arquitectura declara que la instruccion puede modificar.
    pub afecta: Vec<String>,
    pub registros: Vec<CambioRegistro>,
    pub banderas: Vec<CambioBandera>,
    pub escrituras: Vec<EscrituraRegistrada>,
    /// `Some(tomado)` si la instruccion puede cambiar el contador de programa.
    pub salto: Option<bool>,
    pub pc_despues: u64,
    pub ciclos: u64,
    pub detiene: bool,
}

impl Depurador {
    /// Explica la instruccion que toca ejecutar, sin ejecutarla de verdad.
    pub fn explicar(&self) -> Result<Explicacion, ErrorDepurador> {
        let emu = self.emulador();
        let estado = emu.estado();
        let direccion = estado.pc();
        let maquina = emu.maquina();
        let dec = Decodificador::nuevo(maquina).decodificar(emu.memoria(), direccion)?;
        let ins = &maquina.instrucciones[dec.indice];

        let mut copia = emu.clone();
        copia.memoria_mut().registrar_escrituras(true);
        let paso = copia.paso()?.ok_or(ErrorDepurador::MaquinaDetenida)?;
        let escrituras = copia.memoria_mut().tomar_escrituras();
        let despues = copia.estado();

        let pc = estado.nombre_del_pc().to_string();
        let registros = estado
            .nombres()
            .iter()
            .filter(|n| !n.eq_ignore_ascii_case(&pc))
            .filter_map(|n| {
                let a = estado.registro(n).ok()?;
                let d = despues.registro(n).ok()?;
                (a != d).then(|| CambioRegistro {
                    nombre: n.clone(),
                    antes: a,
                    despues: d,
                })
            })
            .collect();
        let banderas = estado
            .nombres_de_banderas()
            .into_iter()
            .filter_map(|n| {
                let a = estado.bandera(n).ok()?;
                let d = despues.bandera(n).ok()?;
                (a != d).then(|| CambioBandera {
                    nombre: n.to_string(),
                    antes: a,
                    despues: d,
                })
            })
            .collect();
        let puede_saltar = ins
            .semantica
            .to_ascii_lowercase()
            .contains(&pc.to_ascii_lowercase());
        Ok(Explicacion {
            direccion,
            texto: vistas::instruccion_en(self, direccion).unwrap_or_else(|| ins.mnemonico.clone()),
            descripcion: ins.descripcion.clone(),
            operaciones: ins
                .semantica
                .split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            operandos: dec
                .operandos
                .iter()
                .map(|o| {
                    let v = match &o.registro {
                        Some(r) => format!("{r} (registro)"),
                        None => format!("{} ({:#x})", o.valor, o.valor),
                    };
                    (o.nombre.clone(), v)
                })
                .collect(),
            afecta: ins.afecta.clone(),
            registros,
            banderas,
            escrituras,
            salto: puede_saltar.then_some(paso.salto),
            pc_despues: despues.pc(),
            ciclos: paso.ciclos,
            detiene: paso.detenida,
        })
    }
}

impl Explicacion {
    pub fn a_texto(&self, ancho_hex: usize) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "{:0ancho_hex$x}  {}", self.direccion, self.texto);
        if !self.descripcion.is_empty() {
            let _ = writeln!(s, "  {}", self.descripcion);
        }
        if !self.operandos.is_empty() {
            let v: Vec<String> = self
                .operandos
                .iter()
                .map(|(n, v)| format!("{n} = {v}"))
                .collect();
            let _ = writeln!(s, "  operandos: {}", v.join(", "));
        }
        let _ = writeln!(s, "  operaciones:");
        for (i, o) in self.operaciones.iter().enumerate() {
            let _ = writeln!(s, "    {}. {o}", i + 1);
        }
        if self.registros.is_empty() {
            let _ = writeln!(s, "  registros: sin cambios");
        } else {
            for r in &self.registros {
                let _ = writeln!(
                    s,
                    "  registro {}: {:#x} -> {:#x}",
                    r.nombre, r.antes, r.despues
                );
            }
        }
        if self.banderas.is_empty() {
            let _ = writeln!(s, "  banderas: sin cambios");
        } else {
            for b in &self.banderas {
                let _ = writeln!(
                    s,
                    "  bandera {}: {} -> {}",
                    b.nombre,
                    u8::from(b.antes),
                    u8::from(b.despues)
                );
            }
        }
        if !self.afecta.is_empty() {
            let _ = writeln!(
                s,
                "  puede modificar las banderas: {}",
                self.afecta.join(", ")
            );
        }
        for e in &self.escrituras {
            let _ = writeln!(
                s,
                "  memoria [{:0ancho_hex$x}]: {:#x} -> {:#x}",
                e.direccion, e.antes, e.despues
            );
        }
        match self.salto {
            Some(true) => {
                let _ = writeln!(
                    s,
                    "  salto: tomado, continua en {:0ancho_hex$x}",
                    self.pc_despues
                );
            }
            Some(false) => {
                let _ = writeln!(
                    s,
                    "  salto: no tomado, continua en {:0ancho_hex$x}",
                    self.pc_despues
                );
            }
            None => {}
        }
        if self.detiene {
            let _ = writeln!(s, "  detiene la maquina");
        }
        let _ = writeln!(s, "  ciclos: {}", self.ciclos);
        s
    }
}
