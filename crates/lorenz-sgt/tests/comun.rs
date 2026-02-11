//! Ayudas para comparar el programa compilado con el interprete de referencia.
#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};

use lorenz_arch::carga::cargar_archivo;
use lorenz_arch::modelo::Maquina;
use lorenz_emu::Emulador;
use lorenz_sgt::analisis::{analizar, Opciones};
use lorenz_sgt::compilar::{compilar, Compilado};
use lorenz_sgt::destino::Destino;
use lorenz_sgt::interprete::{ejecutar, Resultado};
use lorenz_sgt::sintaxis::analizar_programa;

pub fn maquina(nombre: &str) -> Maquina {
    let ruta = format!(
        "{}/../../ejemplos/{nombre}/maquina.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    cargar_archivo(ruta).unwrap()
}

pub fn compilar_en(nombre: &str, fuente: &str) -> Result<Compilado, String> {
    compilar_nivel(nombre, fuente, Opciones::default().nivel)
}

pub fn compilar_nivel(nombre: &str, fuente: &str, nivel: u8) -> Result<Compilado, String> {
    let opciones = Opciones {
        nivel,
        ..Opciones::default()
    };
    compilar("t.sgt", fuente, &maquina(nombre), opciones).map_err(|e| {
        e.iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

pub fn oraculo(nombre: &str, fuente: &str) -> Resultado {
    let m = maquina(nombre);
    let d = Destino::de(&m);
    let usuario = analizar_programa("t.sgt", fuente).unwrap();
    // El oraculo tambien ve la biblioteca publica (`mem_*`, `cad_*`).
    let mut p = lorenz_sgt::biblioteca::programa(m.bits_palabra).unwrap();
    p.elementos.extend(usuario.elementos);
    let h = analizar("t.sgt", &p, &d, Opciones::default()).unwrap();
    ejecutar(&h, &d, HashMap::<u64, VecDeque<u64>>::new(), 10_000_000).unwrap()
}

/// Compila, ejecuta en el emulador y devuelve el valor final de cada global escalar.
pub fn ejecutado(nombre: &str, fuente: &str) -> (HashMap<String, i128>, u64) {
    ejecutado_nivel(nombre, fuente, Opciones::default().nivel).0
}

/// Como `ejecutado`, con un nivel de optimizacion; devuelve tambien el programa compilado.
pub fn ejecutado_nivel(
    nombre: &str,
    fuente: &str,
    nivel: u8,
) -> ((HashMap<String, i128>, u64), Compilado) {
    let c = compilar_nivel(nombre, fuente, nivel).unwrap_or_else(|e| panic!("{e}"));
    let mut emu = Emulador::nuevo(maquina(nombre)).unwrap();
    for (dir, w) in c.palabras() {
        emu.cargar_programa(dir, &[w]).unwrap();
    }
    let r = emu
        .ejecutar(50_000_000)
        .unwrap_or_else(|e| panic!("{e}\n{}", c.ensamblador()));
    let mut out = HashMap::new();
    for (i, g) in c.modulo.globales.iter().enumerate() {
        if g.tipo.es_escalar() {
            let a = c.disposicion.globales[i];
            out.insert(
                g.nombre.clone(),
                i128::from(emu.memoria().leer_privilegiado(a).unwrap()),
            );
        }
    }
    ((out, r.instrucciones), c)
}

/// Compara las globales escalares del compilado con las del oraculo.
pub fn igual_al_oraculo(nombre: &str, fuente: &str) {
    let esperado = oraculo(nombre, fuente);
    let (obtenido, _) = ejecutado(nombre, fuente);
    let mascara = (1i128 << maquina(nombre).bits_palabra) - 1;
    for (k, v) in &esperado.globales {
        // Las globales de apoyo del compilador no son del programa: el oraculo no las toca.
        if k.starts_with("__") {
            continue;
        }
        let w = (*v) & mascara;
        assert_eq!(
            obtenido[k], w,
            "global `{k}`: oraculo {v}, emulador {}",
            obtenido[k]
        );
    }
}

/// Resultado de ejecutar un programa compilado con la UART `serie` conectada.
pub struct Corrida {
    pub globales: HashMap<String, i128>,
    pub texto: String,
    pub instrucciones: u64,
}

pub fn correr(nombre: &str, fuente: &str, recibido: &[u8]) -> Corrida {
    let c = compilar_en(nombre, fuente).unwrap_or_else(|e| panic!("{e}"));
    let mut emu = Emulador::nuevo(maquina(nombre)).unwrap();
    for (dir, w) in c.palabras() {
        emu.cargar_programa(dir, &[w]).unwrap();
    }
    if !recibido.is_empty() {
        emu.recibir_en_uart("serie", recibido).unwrap();
    }
    let r = emu
        .ejecutar(5_000_000)
        .unwrap_or_else(|e| panic!("{e}\n{}", c.ensamblador()));
    let mut globales = HashMap::new();
    for (i, g) in c.modulo.globales.iter().enumerate() {
        if g.tipo.es_escalar() {
            let a = c.disposicion.globales[i];
            globales.insert(
                g.nombre.clone(),
                i128::from(emu.memoria().leer_privilegiado(a).unwrap()),
            );
        }
    }
    Corrida {
        globales,
        texto: emu.texto_de_uart("serie").unwrap(),
        instrucciones: r.instrucciones,
    }
}
