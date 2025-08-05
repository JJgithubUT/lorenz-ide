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
    arranque: u64,
}

impl Estado {
    /// Crea el estado inicial: cada registro con su valor declarado y el contador de programa en la
    /// direccion de arranque (RF-EMU-002, RF-EMU-003).
    pub fn nuevo(maquina: &Maquina) -> Self {
        let nombres: Vec<String> = maquina.registros.iter().map(|r| r.nombre.clone()).collect();
        let roles: Vec<RolRegistro> = maquina.registros.iter().map(|r| r.rol).collect();
        let mascaras: Vec<u64> = maquina.registros.iter().map(|r| mascara_de(r.bits)).collect();
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
        let indice_sp = roles.iter().position(|r| *r == RolRegistro::PunteroPila);

        let mut estado = Self {
            nombres,
            roles,
            mascaras,
            valores: iniciales.clone(),
            iniciales,
            indice_pc,
            indice_sp,
            arranque: maquina.arranque,
        };
        estado.valores[indice_pc] = maquina.arranque & estado.mascaras[indice_pc];
        estado
    }

    fn indice(&self, nombre: &str) -> Result<usize, ErrorEjecucion> {
        self.nombres
            .iter()
            .position(|n| n == nombre)
            .ok_or_else(|| ErrorEjecucion::RegistroInexistente { nombre: nombre.to_string() })
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
            return Err(ErrorEjecucion::RegistroDeSoloLectura { nombre: nombre.to_string() });
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
}
