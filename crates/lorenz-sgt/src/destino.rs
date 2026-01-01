//! Lo que el compilador sabe de la maquina destino, sacado de su descripcion (RF-SGT-002, 024).

use std::collections::BTreeSet;

use lorenz_arch::modelo::Maquina;

/// Una region de memoria vista por el compilador.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionDestino {
    pub nombre: String,
    pub desde: u64,
    pub hasta: u64,
    pub lectura: bool,
    pub escritura: bool,
    /// Si responde un dispositivo y no es memoria corriente.
    pub dispositivo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destino {
    pub nombre: String,
    /// Ancho de palabra en bits.
    pub palabra: u32,
    /// Ancho de direccion en bits.
    pub direccion: u32,
    pub arranque: u64,
    /// Si la palabra menos significativa de un valor multipalabra va primero.
    pub little: bool,
    pub regiones: Vec<RegionDestino>,
    /// Numeros de puerto que algun dispositivo declara.
    pub puertos: BTreeSet<u64>,
    /// Registros con su ancho.
    pub registros: Vec<(String, u32)>,
    pub banderas: Vec<String>,
    /// Lineas de interrupcion con vector declarado.
    pub lineas: BTreeSet<u32>,
}

impl Destino {
    pub fn de(m: &Maquina) -> Self {
        let regiones = m
            .regiones
            .iter()
            .map(|r| RegionDestino {
                nombre: r.nombre.clone(),
                desde: r.desde,
                hasta: r.hasta,
                lectura: r.acceso.lectura,
                escritura: r.acceso.escritura,
                dispositivo: r.dispositivo.is_some(),
            })
            .collect();
        let mut puertos = BTreeSet::new();
        for d in &m.dispositivos {
            puertos.extend(d.puertos.values().copied());
        }
        Self {
            nombre: m.nombre.clone(),
            palabra: m.bits_palabra,
            direccion: m.bits_direccion,
            arranque: m.arranque,
            little: matches!(m.endianness, lorenz_arch::modelo::Endianness::Little),
            regiones,
            puertos,
            registros: m
                .registros
                .iter()
                .map(|r| (r.nombre.clone(), r.bits))
                .collect(),
            banderas: m.banderas.iter().map(|b| b.nombre.clone()).collect(),
            lineas: m
                .interrupciones
                .as_ref()
                .map(|i| i.vectores.keys().copied().collect())
                .unwrap_or_default(),
        }
    }

    /// Region que contiene una direccion.
    pub fn region_de(&self, direccion: u64) -> Option<&RegionDestino> {
        self.regiones
            .iter()
            .find(|r| (r.desde..=r.hasta).contains(&direccion))
    }

    /// Anchos de entero disponibles: la palabra y sus multiplos por 2 y por 4, hasta 64 bits
    /// (RF-SGT-002).
    pub fn anchos_de_entero(&self) -> Vec<u32> {
        [1u32, 2, 4]
            .iter()
            .map(|k| self.palabra * k)
            .filter(|b| *b <= 64)
            .collect()
    }

    /// Cuantas palabras ocupa un valor de `bits` bits.
    pub fn palabras_de(&self, bits: u32) -> u64 {
        u64::from(bits.div_ceil(self.palabra))
    }
}
