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
    compilar("t.sgt", fuente, &maquina(nombre), Opciones::default()).map_err(|e| {
        e.iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

pub fn oraculo(nombre: &str, fuente: &str) -> Resultado {
    let m = maquina(nombre);
    let d = Destino::de(&m);
    let p = analizar_programa("t.sgt", fuente).unwrap();
    let h = analizar("t.sgt", &p, &d, Opciones::default()).unwrap();
    ejecutar(&h, &d, HashMap::<u64, VecDeque<u64>>::new(), 10_000_000).unwrap()
}

/// Compila, ejecuta en el emulador y devuelve el valor final de cada global escalar.
pub fn ejecutado(nombre: &str, fuente: &str) -> (HashMap<String, i128>, u64) {
    let c = compilar_en(nombre, fuente).unwrap_or_else(|e| panic!("{e}"));
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
    (out, r.instrucciones)
}

/// Compara las globales escalares del compilado con las del oraculo.
pub fn igual_al_oraculo(nombre: &str, fuente: &str) {
    let esperado = oraculo(nombre, fuente);
    let (obtenido, _) = ejecutado(nombre, fuente);
    for (k, v) in &esperado.globales {
        let w = (*v) & 0xFFFF;
        assert_eq!(
            obtenido[k], w,
            "global `{k}`: oraculo {v}, emulador {}",
            obtenido[k]
        );
    }
}
