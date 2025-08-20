//! Estado de los registros de la maquina emulada.
//!
//! Guarda el valor de cada registro declarado, ya truncado a su ancho, y sabe cual es el contador
//! de programa, el puntero de pila y el registro de banderas por su rol, no por su nombre.

use lorenz_arch::modelo::{Maquina, RolRegistro};

use crate::error::ErrorEjecucion;

/// Mascara con un 1 en cada uno de los `bits` bajos.
pub fn mascara_de(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}

/// Valores de todos los registros de una maquina.
#[derive(Debug, Clone)]
pub struct Estado {
    nombres: Vec<String>,
    roles: Vec<RolRegistro>,
    mascaras: Vec<u64>,
    iniciales: Vec<u64>,
    valores: Vec<u64>,
    indice_pc: usize,
    indice_sp: Option<usize>,
    indice_banderas: Option<usize>,
    instrucciones: u64,
    ciclos: u64,
    detenida: bool,
    interrupciones_habilitadas: bool,
    habilitadas_al_arrancar: bool,
    /// Nombre y posicion de cada bandera dentro del registro de banderas.
    banderas: Vec<(String, u32)>,
    arranque: u64,
}

impl Estado {
    /// Crea el estado inicial: cada registro con su valor declarado y el contador de programa en la
    /// direccion de arranque (RF-EMU-002, RF-EMU-003).
    pub fn nuevo(maquina: &Maquina) -> Self {
        let nombres: Vec<String> = maquina.registros.iter().map(|r| r.nombre.clone()).collect();
        let roles: Vec<RolRegistro> = maquina.registros.iter().map(|r| r.rol).collect();
        let mascaras: Vec<u64> = maquina
            .registros
            .iter()
            .map(|r| mascara_de(r.bits))
            .collect();
        let iniciales: Vec<u64> = maquina
            .registros
            .iter()
            .zip(&mascaras)
            .map(|(r, m)| r.inicial & m)
            .collect();

        let indice_pc = roles
            .iter()
            .position(|r| *r == RolRegistro::ContadorPrograma)
            .expect("una maquina validada siempre tiene contador de programa");
        let habilitadas_al_arrancar =
            maquina.interrupciones.as_ref().is_some_and(|i| i.habilitadas_al_arrancar);
        let indice_sp = roles.iter().position(|r| *r == RolRegistro::PunteroPila);
        let indice_banderas = roles.iter().position(|r| *r == RolRegistro::Banderas);
        let banderas = maquina
            .banderas
            .iter()
            .map(|b| (b.nombre.clone(), b.bit))
            .collect();

        let mut estado = Self {
            nombres,
            roles,
            mascaras,
            valores: iniciales.clone(),
            iniciales,
            indice_pc,
            indice_sp,
            indice_banderas,
            instrucciones: 0,
            ciclos: 0,
            detenida: false,
            interrupciones_habilitadas: habilitadas_al_arrancar,
            habilitadas_al_arrancar,
            banderas,
            arranque: maquina.arranque,
        };
        estado.valores[indice_pc] = maquina.arranque & estado.mascaras[indice_pc];
        estado
    }

    fn indice(&self, nombre: &str) -> Result<usize, ErrorEjecucion> {
        self.nombres
            .iter()
            .position(|n| n == nombre)
            .ok_or_else(|| ErrorEjecucion::RegistroInexistente {
                nombre: nombre.to_string(),
            })
    }

    /// Cuantos registros tiene la maquina.
    pub fn numero_de_registros(&self) -> usize {
        self.nombres.len()
    }

    /// Nombres de los registros, en el orden en que se declararon.
    pub fn nombres(&self) -> &[String] {
        &self.nombres
    }

    /// Valor de un registro por su nombre.
    pub fn registro(&self, nombre: &str) -> Result<u64, ErrorEjecucion> {
        Ok(self.valores[self.indice(nombre)?])
    }

    /// Asigna un registro por su nombre, truncando el valor a su ancho.
    ///
    /// Un registro de rol `fijo` no se puede modificar (RF-ARQ-008).
    pub fn fijar_registro(&mut self, nombre: &str, valor: u64) -> Result<(), ErrorEjecucion> {
        let i = self.indice(nombre)?;
        if self.roles[i] == RolRegistro::Fijo {
            return Err(ErrorEjecucion::RegistroDeSoloLectura {
                nombre: nombre.to_string(),
            });
        }
        self.valores[i] = valor & self.mascaras[i];
        Ok(())
    }

    /// Contador de programa.
    pub fn pc(&self) -> u64 {
        self.valores[self.indice_pc]
    }

    /// Fija el contador de programa, truncado a su ancho.
    pub fn fijar_pc(&mut self, valor: u64) {
        self.valores[self.indice_pc] = valor & self.mascaras[self.indice_pc];
    }

    /// Puntero de pila, si la maquina tiene uno.
    pub fn sp(&self) -> Option<u64> {
        self.indice_sp.map(|i| self.valores[i])
    }

    /// Fija el puntero de pila; no hace nada si la maquina no tiene.
    pub fn fijar_sp(&mut self, valor: u64) {
        if let Some(i) = self.indice_sp {
            self.valores[i] = valor & self.mascaras[i];
        }
    }

    fn posicion_de_bandera(&self, nombre: &str) -> Result<(usize, u32), ErrorEjecucion> {
        let inexistente = || ErrorEjecucion::BanderaInexistente {
            nombre: nombre.to_string(),
        };
        let registro = self.indice_banderas.ok_or_else(inexistente)?;
        let bit = self
            .banderas
            .iter()
            .find(|(n, _)| n == nombre)
            .map(|(_, b)| *b)
            .ok_or_else(inexistente)?;
        Ok((registro, bit))
    }

    /// Si la bandera esta encendida.
    pub fn bandera(&self, nombre: &str) -> Result<bool, ErrorEjecucion> {
        let (registro, bit) = self.posicion_de_bandera(nombre)?;
        Ok((self.valores[registro] >> bit) & 1 == 1)
    }

    /// Enciende o apaga una bandera sin tocar las demas (RF-EMU-007).
    pub fn fijar_bandera(&mut self, nombre: &str, encendida: bool) -> Result<(), ErrorEjecucion> {
        let (registro, bit) = self.posicion_de_bandera(nombre)?;
        if encendida {
            self.valores[registro] |= 1u64 << bit;
        } else {
            self.valores[registro] &= !(1u64 << bit);
        }
        self.valores[registro] &= self.mascaras[registro];
        Ok(())
    }

    /// Nombres de las banderas declaradas, en el orden en que se declararon.
    pub fn nombres_de_banderas(&self) -> Vec<&str> {
        self.banderas.iter().map(|(n, _)| n.as_str()).collect()
    }

    /// Vuelve cada registro a su valor inicial y el contador de programa a la direccion de arranque
    /// (RF-EMU-014).
    pub fn reiniciar(&mut self) {
        self.valores.clone_from(&self.iniciales);
        self.valores[self.indice_pc] = self.arranque & self.mascaras[self.indice_pc];
        self.instrucciones = 0;
        self.ciclos = 0;
        self.detenida = false;
        self.interrupciones_habilitadas = self.habilitadas_al_arrancar;
    }

    /// Instrucciones ejecutadas desde el ultimo reinicio.
    pub fn instrucciones(&self) -> u64 {
        self.instrucciones
    }

    /// Ciclos consumidos desde el ultimo reinicio.
    pub fn ciclos(&self) -> u64 {
        self.ciclos
    }

    /// Cuenta una instruccion ejecutada que costo `ciclos` ciclos (RF-EMU-016).
    pub fn contar_instruccion(&mut self, ciclos: u64) {
        self.instrucciones = self.instrucciones.saturating_add(1);
        self.ciclos = self.ciclos.saturating_add(ciclos);
    }

    /// Si la maquina ejecuto una instruccion que la detiene.
    pub fn detenida(&self) -> bool {
        self.detenida
    }

    pub fn detener(&mut self) {
        self.detenida = true;
    }

    pub fn interrupciones_habilitadas(&self) -> bool {
        self.interrupciones_habilitadas
    }

    pub fn habilitar_interrupciones(&mut self, habilitadas: bool) {
        self.interrupciones_habilitadas = habilitadas;
    }

    /// Mascara con un 1 en cada bit del registro (su ancho), por nombre.
    pub fn mascara_de_registro(&self, nombre: &str) -> Result<u64, ErrorEjecucion> {
        Ok(self.mascaras[self.indice(nombre)?])
    }

    /// Nombre con el que se declaro el registro de contador de programa.
    pub fn nombre_del_pc(&self) -> &str {
        &self.nombres[self.indice_pc]
    }

    /// Nombre con el que se declaro el puntero de pila, si existe.
    pub fn nombre_del_sp(&self) -> Option<&str> {
        self.indice_sp.map(|i| self.nombres[i].as_str())
    }

    /// Si hay una bandera con ese nombre.
    pub fn tiene_bandera(&self, nombre: &str) -> bool {
        self.banderas.iter().any(|(n, _)| n == nombre)
    }
}
