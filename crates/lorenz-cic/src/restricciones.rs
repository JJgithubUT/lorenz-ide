//! Restricciones de diseno (RF-CIC-033).
//!
//! Una restriccion pone un limite a una cifra que sale de simular un programa: como mucho tantos
//! ciclos, como minimo tal fraccion de saltos acertados, no mas de tantas palabras de codigo. Se
//! declaran en la descripcion de la maquina con `[[restriccion]]` y se comprueban contra las
//! medidas de una ejecucion.

use crate::sim::Metricas;

/// Cifra de una ejecucion sobre la que se puede poner un limite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metrica {
    Ciclos,
    Instrucciones,
    Cpi,
    EsperasDatos,
    EsperasEstructurales,
    EsperasControl,
    EsperasBus,
    SaltosFallados,
    UsoDelBus,
    FallosCache,
    PalabrasCodigo,
}

impl Metrica {
    pub const TODAS: [Metrica; 11] = [
        Metrica::Ciclos,
        Metrica::Instrucciones,
        Metrica::Cpi,
        Metrica::EsperasDatos,
        Metrica::EsperasEstructurales,
        Metrica::EsperasControl,
        Metrica::EsperasBus,
        Metrica::SaltosFallados,
        Metrica::UsoDelBus,
        Metrica::FallosCache,
        Metrica::PalabrasCodigo,
    ];

    pub fn nombre(self) -> &'static str {
        match self {
            Metrica::Ciclos => "ciclos",
            Metrica::Instrucciones => "instrucciones",
            Metrica::Cpi => "cpi",
            Metrica::EsperasDatos => "esperas-datos",
            Metrica::EsperasEstructurales => "esperas-estructurales",
            Metrica::EsperasControl => "esperas-control",
            Metrica::EsperasBus => "esperas-bus",
            Metrica::SaltosFallados => "saltos-fallados",
            Metrica::UsoDelBus => "uso-del-bus",
            Metrica::FallosCache => "fallos-cache",
            Metrica::PalabrasCodigo => "palabras-codigo",
        }
    }

    pub fn desde_nombre(n: &str) -> Result<Self, String> {
        Metrica::TODAS
            .into_iter()
            .find(|m| m.nombre() == n)
            .ok_or_else(|| {
                let todas: Vec<&str> = Metrica::TODAS.iter().map(|m| m.nombre()).collect();
                format!("la metrica `{n}` no existe; usa {}", todas.join(", "))
            })
    }
}

/// Lo que se midio al ejecutar un programa: las metricas del simulador y el tamano del codigo.
#[derive(Debug, Clone, PartialEq)]
pub struct Medidas {
    pub metricas: Metricas,
    pub palabras_codigo: u64,
}

impl Medidas {
    pub fn new(metricas: Metricas, palabras_codigo: u64) -> Self {
        Self {
            metricas,
            palabras_codigo,
        }
    }

    /// Valor de una cifra, o `None` si no se puede medir (el CPI sin instrucciones, los fallos de
    /// cache sin cache).
    pub fn valor(&self, m: Metrica) -> Option<f64> {
        let x = &self.metricas;
        match m {
            Metrica::Ciclos => Some(x.ciclos as f64),
            Metrica::Instrucciones => Some(x.instrucciones as f64),
            Metrica::Cpi => x.cpi(),
            Metrica::EsperasDatos => Some(x.esperas_datos as f64),
            Metrica::EsperasEstructurales => Some(x.esperas_estructurales as f64),
            Metrica::EsperasControl => Some(x.esperas_control as f64),
            Metrica::EsperasBus => Some(x.esperas_bus as f64),
            Metrica::SaltosFallados => Some(x.saltos_fallados() as f64),
            Metrica::UsoDelBus => x.uso_del_bus(),
            Metrica::FallosCache => x
                .cache
                .as_ref()
                .map(|c| (c.fallos_lectura + c.fallos_escritura) as f64),
            Metrica::PalabrasCodigo => Some(self.palabras_codigo as f64),
        }
    }
}

/// Un limite sobre una cifra.
#[derive(Debug, Clone, PartialEq)]
pub struct Restriccion {
    pub nombre: String,
    pub metrica: Metrica,
    pub minimo: Option<f64>,
    pub maximo: Option<f64>,
}

impl Restriccion {
    /// Comprueba que la restriccion tenga sentido: algun limite y que no se contradigan.
    pub fn validar(&self) -> Result<(), String> {
        if self.minimo.is_none() && self.maximo.is_none() {
            return Err(format!(
                "restriccion `{}`: falta `maximo` o `minimo`",
                self.nombre
            ));
        }
        if let (Some(a), Some(b)) = (self.minimo, self.maximo) {
            if a > b {
                return Err(format!(
                    "restriccion `{}`: el minimo ({a}) es mayor que el maximo ({b})",
                    self.nombre
                ));
            }
        }
        Ok(())
    }
}

/// Resultado de comprobar una restriccion.
#[derive(Debug, Clone, PartialEq)]
pub struct Veredicto {
    pub restriccion: Restriccion,
    /// Valor medido; `None` si la cifra no se puede medir en esta maquina.
    pub valor: Option<f64>,
    pub cumple: bool,
}

/// Comprueba cada restriccion contra las medidas. Una cifra que no se puede medir no cumple.
pub fn comprobar(restricciones: &[Restriccion], medidas: &Medidas) -> Vec<Veredicto> {
    restricciones
        .iter()
        .map(|r| {
            let valor = medidas.valor(r.metrica);
            let cumple = valor.is_some_and(|v| {
                r.minimo.map_or(true, |m| v >= m) && r.maximo.map_or(true, |m| v <= m)
            });
            Veredicto {
                restriccion: r.clone(),
                valor,
                cumple,
            }
        })
        .collect()
}

/// Si todas se cumplen.
pub fn todas_cumplen(v: &[Veredicto]) -> bool {
    v.iter().all(|x| x.cumple)
}
