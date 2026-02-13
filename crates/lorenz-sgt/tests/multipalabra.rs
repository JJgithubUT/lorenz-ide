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
