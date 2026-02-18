//! Enteros de varias palabras (RF-SGT-001, RF-SGT-002): u32 en 16 bits y u16 en 8 bits.
mod comun;
use comun::*;

fn dif(maq: &str, fuente: &str) {
    igual_al_oraculo(maq, fuente);
}

fn ambas(fuente16: &str, fuente8: &str) {
    dif("risc-16", fuente16);
    dif("sgt-16", fuente16);
    dif("sgt-8", fuente8);
}

#[test]
fn suma_con_acarreo() {
    ambas(
        "var a: u32; var b: u32; var c: u32;
         funcion principal() { var x: u32 = 0x0001FFFF; var y: u32 = 0x00000001;
           a = x + y; b = 0xFFFFFFFF; b += 2; c = x - y; }",
        "var a: u16; var b: u16; var c: u16;
         funcion principal() { var x: u16 = 0x01FF; var y: u16 = 1;
           a = x + y; b = 0xFFFF; b += 2; c = x - y; }",
    );
}

#[test]
fn resta_con_prestamo() {
    ambas(
        "var a: u32; var b: u32; var c: u32;
         funcion principal() { var x: u32 = 0x00010000; var y: u32 = 1;
           a = x - y; b = y - x; c = 0 - y; }",
        "var a: u16; var b: u16; var c: u16;
         funcion principal() { var x: u16 = 0x0100; var y: u16 = 1;
           a = x - y; b = y - x; c = 0 - y; }",
    );
}

#[test]
fn bits() {
    ambas(
        "var a: u32; var b: u32; var c: u32; var d: u32;
         funcion principal() { var x: u32 = 0x12345678; var y: u32 = 0x0F0F00FF;
           a = x & y; b = x | y; c = x ^ y; d = ~x; }",
        "var a: u16; var b: u16; var c: u16; var d: u16;
         funcion principal() { var x: u16 = 0x1234; var y: u16 = 0x0FF0;
           a = x & y; b = x | y; c = x ^ y; d = ~x; }",
    );
}

#[test]
fn comparaciones() {
    ambas(
        "var r: u16;
         funcion principal() { var a: u32 = 0x00010000; var b: u32 = 0x0000FFFF; var c: u32 = 0x00010000;
           si a < b { r += 1; } si a <= b { r += 2; } si a > b { r += 4; } si a >= b { r += 8; }
           si a == c { r += 16; } si a != c { r += 32; } si b < c { r += 64; } si b >= c { r += 128; }
           si a <= c { r += 256; } si a >= c { r += 512; } }",
        "var r: u8;
         funcion principal() { var a: u16 = 0x0100; var b: u16 = 0x00FF; var c: u16 = 0x0100;
           si a < b { r += 1; } si a <= b { r += 2; } si a > b { r += 4; } si a >= b { r += 8; }
           si a == c { r += 16; } si a != c { r += 32; } si b < c { r += 64; } si b >= c { r += 128; } }",
    );
}

#[test]
fn comparaciones_con_signo() {
    ambas(
        "var r: u16;
         funcion principal() { var a: i32 = -5; var b: i32 = 70000; var c: i32 = -70000;
           si a < b { r += 1; } si a > b { r += 2; } si c < a { r += 4; } si c >= a { r += 8; }
           si b > c { r += 16; } si a <= a { r += 32; } si c < 0 { r += 64; } }",
        "var r: u8;
         funcion principal() { var a: i16 = -5; var b: i16 = 300; var c: i16 = -300;
           si a < b { r += 1; } si a > b { r += 2; } si c < a { r += 4; } si c >= a { r += 8; }
           si b > c { r += 16; } si a <= a { r += 32; } si c < 0 { r += 64; } }",
    );
}

#[test]
fn desplazamientos_por_constante() {
    ambas(
        "var a: u32; var b: u32; var c: u32; var d: u32; var e: i32; var f: i32; var g: u32;
         funcion principal() { var x: u32 = 0x12345678; var s: i32 = -0x12345678;
           a = x << 4; b = x >> 4; c = x << 20; d = x >> 20; e = s >> 3; f = s >> 17; g = x << 16; }",
        "var a: u16; var b: u16; var c: u16; var d: u16; var e: i16; var f: i16; var g: u16;
         funcion principal() { var x: u16 = 0x1234; var s: i16 = -0x1234;
           a = x << 4; b = x >> 4; c = x << 11; d = x >> 11; e = s >> 3; f = s >> 9; g = x << 8; }",
    );
}

#[test]
fn desplazamientos_variables() {
    ambas(
        "var a: u32; var b: u32; var c: i32;
         funcion principal() { var x: u32 = 0x00F0F0F1; var n: u32 = 7; var s: i32 = -100000; var m: i32 = 7;
           a = x << n; b = x >> n; c = s >> m; }",
        "var a: u16; var b: u16; var c: i16;
         funcion principal() { var x: u16 = 0x00F1; var n: u16 = 5; var s: i16 = -1000; var m: i16 = 5;
           a = x << n; b = x >> n; c = s >> m; }",
    );
}

#[test]
fn multiplicar() {
    ambas(
        "var a: u32; var b: u32; var c: i32;
         funcion principal() { var x: u32 = 123456; var y: u32 = 789; var s: i32 = -1234;
           a = x * y; b = x * 0; c = s * 1000; }",
        "var a: u16; var b: u16; var c: i16;
         funcion principal() { var x: u16 = 1234; var y: u16 = 37; var s: i16 = -123;
           a = x * y; b = x * 0; c = s * 100; }",
    );
}

#[test]
fn dividir_y_resto() {
    ambas(
        "var a: u32; var b: u32; var c: i32; var d: i32; var e: i32; var f: i32;
         funcion principal() { var x: u32 = 1234567; var y: u32 = 1000; var s: i32 = -1234567; var t: i32 = 1000;
           a = x / y; b = x % y; c = s / t; d = s % t; e = s / -t; f = x as_i32(); }"
            .replace("f = x as_i32();", "f = x como i32;")
            .as_str(),
        "var a: u16; var b: u16; var c: i16; var d: i16;
         funcion principal() { var x: u16 = 12345; var y: u16 = 100; var s: i16 = -12345; var t: i16 = 100;
           a = x / y; b = x % y; c = s / t; d = s % t; }",
    );
}

#[test]
fn funciones_con_parametros_y_resultado_anchos() {
    ambas(
        "var r: u32; var s: u32;
         funcion suma3(a: u32, b: u32) -> u32 { devolver a + b; }
         funcion doble(x: u32) -> u32 { devolver x + x; }
         funcion principal() { r = suma3(70000, 65535); s = doble(doble(r)) + 1; }",
        "var r: u16; var s: u16;
         funcion suma3(a: u16, b: u16) -> u16 { devolver a + b; }
         funcion doble(x: u16) -> u16 { devolver x + x; }
         funcion principal() { r = suma3(300, 255); s = doble(doble(r)) + 1; }",
    );
}

#[test]
fn conversiones() {
    ambas(
        "var a: u32; var b: i32; var c: u16; var d: u32; var e: i32;
         funcion principal() { var p: u16 = 40000; var q: i16 = -5; var x: u32 = 0x12345678;
           a = p como u32; b = q como i32; c = x como u16; d = q como u32; e = (q como i32) * 3; }",
        "var a: u16; var b: i16; var c: u8; var d: u16;
         funcion principal() { var p: u8 = 200; var q: i8 = -5; var x: u16 = 0x1234;
           a = p como u16; b = q como i16; c = x como u8; d = q como u16; }",
    );
}

#[test]
fn arreglos_de_enteros_anchos() {
    let f = "var v: [u32; 4]; var r: u32; var s: u32; var t: i32;
         funcion principal() { var i: u16 = 0;
           mientras i < 4 { v[i] = (i como u32) * 70000 + 1; i += 1; }
           r = v[3] + v[1]; s = v[2] - v[0]; v[1] += 5; t = (v[1] como i32) - (v[3] como i32); }";
    dif("risc-16", f);
    dif("sgt-16", f);
}

#[test]
fn estructuras_con_campos_anchos() {
    ambas(
        "estructura P { a: u16, b: u32, c: u16 }
         var p: P; var q: [P; 2]; var r: u32; var s: u32;
         funcion principal() {
           p.b = 70000 + 3; p.a = 7; p.c = 9; q[1].b = p.b * 2; q[0].b = q[1].b - p.b;
           r = p.b; s = q[0].b + (p.a como u32); }",
        "estructura P { a: u8, b: u16, c: u8 }
         var p: P; var r: u16; var s: u16;
         funcion principal() { p.b = 300 + 3; p.a = 7; p.c = 9; r = p.b; s = r * 2 + (p.a como u16); }",
    );
}

#[test]
fn condiciones_con_efectos_en_bucles() {
    ambas(
        "var n: u32; var total: u32;
         funcion toca() -> u32 { n += 1; devolver n; }
         funcion principal() { mientras toca() < 5 { total += n * 100000; } }",
        "var n: u16; var total: u16;
         funcion toca() -> u16 { n += 1; devolver n; }
         funcion principal() { mientras toca() < 5 { total += n * 100; } }",
    );
}

#[test]
fn globales_con_valor_inicial() {
    ambas(
        "var a: u32 = 0xDEADBEEF; var b: i32 = -2; var c: u32;
         funcion principal() { c = a + 1; }",
        "var a: u16 = 0xBEEF; var b: i16 = -2; var c: u16;
         funcion principal() { c = a + 1; }",
    );
}

#[test]
fn cuatro_palabras() {
    ambas(
        "var a: u64; var b: u64; var c: i64; var d: u64; var e: u64; var f: u64; var r: u16;
         funcion principal() { var x: u64 = 0x0123456789ABCDEF; var y: u64 = 0xFEDCBA9876543210; var s: i64 = -0x123456789AB;
           a = x + y; b = y - x; c = s * 3; d = x ^ (y >> 9); e = x * 1000003; f = y / 12345 + y % 1000;
           si x < y { r += 1; } si s < 0 { r += 2; } si x != y { r += 4; } c = c >> 5; }",
        "var a: u32; var b: u32; var c: i32; var d: u32; var e: u32; var f: u32; var r: u8;
         funcion principal() { var x: u32 = 0x01234567; var y: u32 = 0xFEDCBA98; var s: i32 = -0x123456;
           a = x + y; b = y - x; c = s * 3; d = x ^ (y >> 9); e = x * 1003; f = y / 12345 + y % 1000;
           si x < y { r += 1; } si s < 0 { r += 2; } si x != y { r += 4; } c = c >> 5; }",
    );
}

#[test]
fn en_todos_los_niveles_de_optimizacion() {
    let f16 = "var a: u32; var b: u32; var c: i32; var d: u32;
         funcion f(x: u32, y: u32) -> u32 { si x > y { devolver x - y; } devolver y - x + 1; }
         funcion principal() { var x: u32 = 100000; var s: i32 = -300000;
           a = f(x, 7) + f(3, x); b = (x << 3) | (x >> 2); c = s / 7 + (s % 7); d = x * 0 + (x & 0xFFFF) + 0; }";
    let f8 = "var a: u16; var b: u16; var c: i16; var d: u16;
         funcion f(x: u16, y: u16) -> u16 { si x > y { devolver x - y; } devolver y - x + 1; }
         funcion principal() { var x: u16 = 1000; var s: i16 = -3000;
           a = f(x, 7) + f(3, x); b = (x << 3) | (x >> 2); c = s / 7 + (s % 7); d = x * 0 + (x & 0xFF) + 0; }";
    for (m, f) in [("risc-16", f16), ("sgt-16", f16), ("sgt-8", f8)] {
        let esperado = oraculo(m, f);
        for nivel in 0..=3 {
            let ((g, _), c) = ejecutado_nivel(m, f, nivel);
            for (k, v) in esperado
                .globales
                .iter()
                .filter(|(k, _)| !k.starts_with("__"))
            {
                let mascara = (1i128 << bits_de_global(&c, k, maquina(m).bits_palabra)) - 1;
                assert_eq!(g[k], *v & mascara, "nivel {nivel} en {m}: global `{k}`");
            }
        }
    }
}
