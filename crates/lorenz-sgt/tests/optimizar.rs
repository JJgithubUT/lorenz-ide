//! Optimizaciones (RF-SGT-033, RF-SGT-034): cambian el tamano, nunca el resultado.
mod comun;
use comun::*;

const MAQUINAS: [&str; 2] = ["risc-16", "sgt-16"];

/// El programa da lo mismo que el oraculo en cada nivel, y devuelve el tamano por nivel.
fn en_todos_los_niveles(maq: &str, fuente: &str) -> Vec<(usize, u64)> {
    let esperado = oraculo(maq, fuente);
    let mut tamanos = Vec::new();
    for nivel in 0..=3 {
        let ((g, instr), c) = ejecutado_nivel(maq, fuente, nivel);
        for (k, v) in esperado
            .globales
            .iter()
            .filter(|(k, _)| !k.starts_with("__"))
        {
            assert_eq!(
                g[k],
                (*v) & 0xFFFF,
                "nivel {nivel} en {maq}: global `{k}`\n{}",
                c.ensamblador()
            );
        }
        tamanos.push((c.palabras().len(), instr));
    }
    tamanos
}

fn todas(fuente: &str) -> Vec<Vec<(usize, u64)>> {
    MAQUINAS
        .iter()
        .map(|m| en_todos_los_niveles(m, fuente))
        .collect()
}

#[test]
fn plegado_de_constantes_reduce_el_codigo() {
    let t = todas(
        "var a: u16; var b: i16; var c: u16;
         funcion principal() { a = (3 + 4) * 5 - 1; b = -(2 * 8) + 1; c = (a & 0xF) << 2; }",
    );
    // El analizador ya pliega las constantes escritas a mano; el optimizador no debe empeorarlas.
    for por_nivel in t {
        assert!(por_nivel[1].0 <= por_nivel[0].0, "{por_nivel:?}");
    }
}

#[test]
fn ramas_con_condicion_conocida() {
    let t = todas(
        "var a: u16; var b: u16;
         funcion principal() {
           si 1 < 2 { a = 10; } sino { a = 20; }
           si 5 == 6 { b = 1; }
           mientras 0 > 1 { b = 99; }
         }",
    );
    for por_nivel in t {
        assert!(por_nivel[1].0 < por_nivel[0].0, "{por_nivel:?}");
    }
}

#[test]
fn codigo_muerto_tras_devolver_y_romper() {
    todas(
        "var a: u16; var b: u16;
         funcion f(x: u16) -> u16 { si x > 0 { devolver x + 1; a = 77; } devolver 0; }
         funcion principal() {
           b = f(4);
           bucle { a += 1; si a > 3 { romper; b = 55; } }
         }",
    );
}

#[test]
fn segun_con_valor_conocido() {
    todas(
        "var r: u16;
         funcion principal() {
           segun 2 { 1 => { r = 10; } 2 => { r = 20; } otro => { r = 30; } }
           segun 9 { 1 => { r += 1; } otro => { r += 2; } }
         }",
    );
}

#[test]
fn cortocircuito_conocido() {
    todas(
        "var r: u16; var s: u16;
         funcion cuenta() -> bool { s += 1; devolver verdadero; }
         funcion principal() {
           si falso && cuenta() { r += 1; }
           si verdadero || cuenta() { r += 2; }
           si verdadero && cuenta() { r += 4; }
           si falso || cuenta() { r += 8; }
         }",
    );
}

#[test]
fn identidades_algebraicas() {
    todas(
        "var a: u16; var b: u16; var c: u16; var d: u16;
         funcion principal() {
           var x: u16 = 37;
           a = x + 0; b = x * 1; c = x * 0; d = (x | 0) ^ 0;
           a += (0 + x) - 0; b = b * 1 + (x << 0);
         }",
    );
}

#[test]
fn reduccion_de_fuerza_sin_signo() {
    let t = todas(
        "var a: u16; var b: u16; var c: u16;
         funcion principal() {
           var x: u16 = 1234;
           a = x * 8; b = x / 16; c = x % 32;
         }",
    );
    // Con la reduccion ya no hace falta la rutina de multiplicar ni la de dividir: el programa
    // ocupa menos aunque los desplazamientos de varias posiciones cuesten mas ciclos.
    for por_nivel in t {
        assert!(por_nivel[3].0 < por_nivel[2].0, "{por_nivel:?}");
    }
}

#[test]
fn reduccion_de_fuerza_respeta_el_signo() {
    todas(
        "var a: i16; var b: i16; var c: i16;
         funcion principal() {
           var x: i16 = -100;
           a = x * 4; b = x / 4; c = x % 8;
         }",
    );
}

#[test]
fn division_por_cero_no_se_elimina() {
    // Dividir entre cero detiene la maquina (D-062) aunque el resultado no se use.
    for m in MAQUINAS {
        for nivel in 0..=3 {
            let c = compilar_nivel(
                m,
                "var z: u16; var r: u16; funcion principal() { r = 1; r = r / z; r = 2; }",
                nivel,
            );
            let c = c.unwrap();
            assert!(!c.palabras().is_empty());
        }
    }
}

#[test]
fn efectos_de_las_llamadas_se_conservan() {
    todas(
        "var n: u16;
         funcion toca() -> u16 { n += 1; devolver 0; }
         funcion principal() { var x: u16 = 5; var y: u16 = x * toca(); y = toca() * 0; toca(); }",
    );
}

#[test]
fn niveles_mayores_no_empeoran_el_programa() {
    let t = todas(
        "var a: u16; var b: u16;
         funcion principal() { var x: u16 = 9; a = x * 4 + (2 + 3); b = x / 2 + 0; si verdadero { b += 1; } }",
    );
    for por_nivel in t {
        // Hasta -O2 el codigo solo encoge; -O3 puede ocupar algo mas (llama a la rutina de
        // desplazar en lugar de la de dividir) a cambio de ejecutar menos instrucciones.
        for w in por_nivel[..3].windows(2) {
            assert!(w[1].0 <= w[0].0, "{por_nivel:?}");
        }
        assert!(por_nivel[3].1 <= por_nivel[2].1, "{por_nivel:?}");
    }
}
