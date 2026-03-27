//! El interprete de referencia de SGT.

use std::collections::{HashMap, VecDeque};

use lorenz_arch::carga::cargar_archivo;
use lorenz_sgt::analisis::{analizar, Opciones};
use lorenz_sgt::destino::Destino;
use lorenz_sgt::interprete::{ejecutar, ErrorEjecucion, Resultado};
use lorenz_sgt::sintaxis::analizar_programa;

fn correr_en(
    maq: &str,
    fuente: &str,
    entradas: &[(u64, &[u64])],
) -> Result<Resultado, ErrorEjecucion> {
    let ruta = format!(
        "{}/../../ejemplos/{maq}/maquina.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    let d = Destino::de(&cargar_archivo(ruta).unwrap());
    let p = analizar_programa("t.sgt", fuente).unwrap_or_else(|e| panic!("{e}"));
    let m = analizar("t.sgt", &p, &d, Opciones::default()).unwrap_or_else(|e| {
        panic!(
            "{}",
            e.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    let mut q: HashMap<u64, VecDeque<u64>> = HashMap::new();
    for (p, vs) in entradas {
        q.insert(*p, vs.iter().copied().collect());
    }
    ejecutar(&m, &d, q, 1_000_000)
}

fn correr(fuente: &str) -> Resultado {
    correr_en("ejemplo-8-serie", fuente, &[]).unwrap()
}

fn salidas(fuente: &str) -> Vec<u64> {
    correr(fuente).salidas.iter().map(|(_, v)| *v).collect()
}

#[test]
fn suma_de_1_a_10() {
    let s = salidas(
        "funcion principal() { var s: u8 = 0; var i: u8 = 1; mientras i <= 10 { s += i; i += 1; } salida(0x10, s); }",
    );
    assert_eq!(s, vec![55]);
}

#[test]
fn desbordamiento_da_la_vuelta() {
    assert_eq!(
        salidas("funcion principal() { var a: u8 = 250; a += 10; salida(0x10, a); }"),
        vec![4]
    );
}

#[test]
fn saturacion() {
    assert_eq!(
        salidas("funcion principal() { var a: u8 = 250; salida(0x10, a +| 10); }"),
        vec![255]
    );
}

#[test]
fn con_signo() {
    let r =
        salidas("funcion principal() { var a: i8 = -100; a -= 100; salida(0x10, (a como u8)); }");
    assert_eq!(r, vec![56]);
}

#[test]
fn llamadas_y_retorno() {
    let r = salidas(
        "funcion doble(x: u8) -> u8 { devolver x + x; } funcion principal() { salida(0x10, doble(doble(5))); }",
    );
    assert_eq!(r, vec![20]);
}

#[test]
fn multipalabra() {
    // u16 sobre palabras de 8 bits.
    let r = correr("var g: u16; funcion principal() { g = 300; g += 300; salida(0x10, (g >> 8) como u8); salida(0x10, g como u8); }");
    assert_eq!(
        r.salidas.iter().map(|x| x.1).collect::<Vec<_>>(),
        vec![2, 88]
    );
    assert_eq!(r.globales["g"], 600);
}

#[test]
fn arreglos_estructuras_y_punteros() {
    let r = salidas(
        "estructura P { x: u8, y: u8 } var t: [P; 3]; funcion principal() { var i: u8 = 0; mientras i < 3 { t[i].x = i; t[i].y = i * 2; i += 1; } var p: *u8 = &t[2].y; salida(0x10, *p); }",
    );
    assert_eq!(r, vec![4]);
}

#[test]
fn punteros_a_globales() {
    let r = salidas(
        "var x: u8 = 7; funcion principal() { var p: *u8 = &x; *p += 1; salida(0x10, x); }",
    );
    assert_eq!(r, vec![8]);
}

#[test]
fn cortocircuito() {
    let r = salidas(
        "var n: u8; funcion toca() -> bool { n += 1; devolver verdadero; } funcion principal() { si falso && toca() { } si verdadero || toca() { } salida(0x10, n); }",
    );
    assert_eq!(r, vec![0]);
}

#[test]
fn segun_y_romper() {
    let r = salidas(
        "funcion principal() { var i: u8 = 0; bucle { segun i { 0 => { salida(0x10, 10); } 1 => { salida(0x10, 11); } otro => { romper; } } i += 1; } }",
    );
    assert_eq!(r, vec![10, 11]);
}

#[test]
fn continuar_en_mientras() {
    let r = salidas(
        "funcion principal() { var i: u8 = 0; mientras i < 5 { i += 1; si i == 3 { continuar; } salida(0x10, i); } }",
    );
    assert_eq!(r, vec![1, 2, 4, 5]);
}

#[test]
fn entrada_de_puertos() {
    let r = correr_en(
        "ejemplo-8-serie",
        "funcion principal() { var a: u8 = entrada(0x10); var b: u8 = entrada(0x10); salida(0x10, a + b); }",
        &[(0x10, &[3, 4])],
    )
    .unwrap();
    assert_eq!(r.salidas, vec![(0x10, 7)]);
}

#[test]
fn division_por_cero_en_ejecucion() {
    let e = correr_en(
        "ejemplo-8-serie",
        "funcion principal() { var z: u8 = 0; var a: u8 = 5 / z; }",
        &[],
    )
    .unwrap_err();
    assert!(matches!(e, ErrorEjecucion::DivisionPorCero(1)));
}

#[test]
fn limite_de_pasos() {
    let e = correr_en("ejemplo-8-serie", "funcion principal() { bucle { } }", &[]).unwrap_err();
    assert_eq!(e, ErrorEjecucion::LimitePasos(1_000_000));
}

#[test]
fn indice_fuera_de_arreglo_en_ejecucion() {
    let e = correr_en(
        "ejemplo-8-serie",
        "var t: [u8; 2]; funcion principal() { var i: u8 = 2; t[i] = 1; }",
        &[],
    )
    .unwrap_err();
    assert!(matches!(
        e,
        ErrorEjecucion::FueraDeArreglo {
            indice: 2,
            largo: 2,
            ..
        }
    ));
}

#[test]
fn ensamblador_en_linea_no_se_interpreta() {
    let e = correr_en(
        "ejemplo-8-serie",
        "funcion principal() { asm(\"NOP\"); }",
        &[],
    )
    .unwrap_err();
    assert!(matches!(e, ErrorEjecucion::NoInterpretable(_)));
}
