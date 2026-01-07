//! El programa compilado, ejecutado en el emulador, coincide con el interprete de referencia.

mod comun;
use comun::*;

#[test]
fn programa_vacio_se_detiene() {
    ejecutado("risc-16", "funcion principal() { }");
}

#[test]
fn asignar_una_global() {
    let (g, _) = ejecutado("risc-16", "var x: u16; funcion principal() { x = 42; }");
    assert_eq!(g["x"], 42);
}

#[test]
fn suma_de_1_a_10() {
    let (g, _) = ejecutado(
        "risc-16",
        "var s: u16; funcion principal() { var i: u16 = 1; mientras i <= 10 { s += i; i += 1; } }",
    );
    assert_eq!(g["s"], 55);
}

fn dif(fuente: &str) {
    igual_al_oraculo("risc-16", fuente);
}

#[test]
fn aritmetica_basica() {
    dif("var a: u16; var b: u16; var c: u16; var d: u16; var e: u16;
         funcion principal() { var x: u16 = 1000; var y: u16 = 234;
           a = x + y; b = x - y; c = y - x; d = (x & 0xFF) | (y ^ 0x0F0F); e = ~x; }");
}

#[test]
fn constantes_grandes_y_negativas() {
    dif("var a: u16; var b: i16; var c: u16; var d: i16;
         funcion principal() { a = 0xABCD; b = -1; c = 40000; d = -32768; }");
}

#[test]
fn comparaciones_sin_signo() {
    dif("var r: u16;
         funcion principal() { var a: u16 = 5; var b: u16 = 60000;
           si a < b { r += 1; } si a <= b { r += 2; } si a > b { r += 4; } si a >= b { r += 8; }
           si a == b { r += 16; } si a != b { r += 32; } si b > a { r += 64; } si b >= a { r += 128; } }");
}

#[test]
fn comparaciones_con_signo() {
    dif("var r: u16;
         funcion principal() { var a: i16 = -5; var b: i16 = 3;
           si a < b { r += 1; } si a <= b { r += 2; } si a > b { r += 4; } si a >= b { r += 8; }
           si a == b { r += 16; } si a != b { r += 32; } si b > a { r += 64; } si b >= a { r += 128; } }");
}

#[test]
fn valores_booleanos() {
    dif("var a: bool; var b: bool; var c: bool; var d: bool;
         funcion principal() { var x: u16 = 3; var y: u16 = 9;
           a = x < y; b = x > y; c = x == 3 && y != 9; d = !(x == 3) || y == 9; }");
}

#[test]
fn bucles_romper_y_continuar() {
    dif("var s: u16; var n: u16;
         funcion principal() { var i: u16 = 0;
           bucle { i += 1; si i > 20 { romper; } si i % 2 == 0 { continuar; } s += i; n += 1; } }"
        .replace(
            "si i % 2 == 0 { continuar; }",
            "si (i & 1) == 0 { continuar; }",
        )
        .as_str());
}

#[test]
fn arreglos_y_estructuras() {
    dif(
        "estructura P { x: u16, y: u16 } var t: [P; 4]; var s: u16; var u: [u16; 5];
         funcion principal() { var i: u16 = 0;
           mientras i < 4 { t[i].x = i + 1; t[i].y = t[i].x + 10; i += 1; }
           i = 0; mientras i < 4 { s += t[i].y; i += 1; }
           u[0] = 7; u[4] = 9; s += u[0] + u[4]; }",
    );
}

#[test]
fn punteros() {
    dif("var x: u16 = 7; var y: u16; var v: [u16; 3];
         funcion principal() { var p: *u16 = &x; *p += 5; y = *p;
           var q: *u16 = &v[0]; *q = 1; *(q + 1) = 2; q[2] = 3; }");
}

#[test]
fn llamadas() {
    dif("var r: u16;
         funcion suma(a: u16, b: u16) -> u16 { devolver a + b; }
         funcion doble(a: u16) -> u16 { devolver suma(a, a); }
         funcion principal() { r = doble(suma(1, 2)) + doble(10); }");
}

#[test]
fn llamadas_con_temporales_vivos() {
    dif("var r: u16;
         funcion f(a: u16) -> u16 { var t: u16 = a + 1; devolver t + t; }
         funcion principal() { var x: u16 = 5; r = x + f(x) + (x + f(x + 1)); }");
}

#[test]
fn segun() {
    dif("var r: u16;
         funcion principal() { var i: u16 = 0;
           mientras i < 6 { segun i { 1 => { r += 1; } 3 => { r += 10; } 4 => { r += 100; } otro => { r += 1000; } } i += 1; } }");
}

#[test]
fn llamada_sin_valor_y_efectos() {
    dif("var c: u16;
         funcion cuenta() { c += 1; }
         funcion principal() { cuenta(); cuenta(); cuenta(); }");
}

#[test]
fn globales_con_valor_inicial() {
    dif(
        "var a: u16 = 11; var b: u16; var c: i16 = -3; var d: bool = verdadero;
         funcion principal() { b = a + 1; }",
    );
}

#[test]
fn valores_concretos_esperados() {
    let (g, _) = ejecutado(
        "risc-16",
        "var r: u16; var s: i16; var t: u16;
         funcion suma(a: u16, b: u16) -> u16 { devolver a + b; }
         funcion doble(a: u16) -> u16 { devolver suma(a, a); }
         funcion principal() { r = doble(suma(1, 2)) + doble(10); s = -7; t = 0xABCD; }",
    );
    assert_eq!(g["r"], 26);
    assert_eq!(g["s"], 0xFFF9);
    assert_eq!(g["t"], 0xABCD);
}

#[test]
fn multiplicar_dividir_y_modulo() {
    dif(
        "var a: u16; var b: u16; var c: u16; var d: u16; var e: u16; var f: u16;
         funcion principal() { var x: u16 = 1234; var y: u16 = 57;
           a = x * y; b = x / y; c = x % y; d = 60000 / x; e = x * 0; f = 7 / 8; }",
    );
}

#[test]
fn mul_div_con_signo() {
    dif(
        "var a: i16; var b: i16; var c: i16; var d: i16; var e: i16; var f: i16;
         funcion principal() { var x: i16 = -1234; var y: i16 = 57;
           a = x * y; b = x / y; c = x % y; d = x / -7; e = (0 - x) % -10; f = -32768 / -1; }",
    );
}

#[test]
fn desplazamientos() {
    dif("var a: u16; var b: u16; var c: u16; var d: i16; var e: i16; var f: u16; var g: u16;
         funcion principal() { var x: u16 = 0xA5C3; var k: u16 = 5; var n: i16 = -1234;
           a = x << k; b = x >> k; c = x >> 16; d = n >> 3; e = n >> 20; f = 1 << 15; g = x << 16; }");
}

#[test]
fn saturacion_compilada() {
    dif("var a: u16; var b: u16; var c: i16; var d: i16; var e: i16; var f: i16;
         funcion principal() { var x: u16 = 65000; var y: u16 = 1000; var p: i16 = 32000; var q: i16 = -32000;
           a = x +| y; b = y -| x; c = p +| 1000; d = q -| 1000; e = p -| q; f = q +| q; }");
}

#[test]
fn division_entre_cero_detiene() {
    let (g, _) = ejecutado(
        "risc-16",
        "var a: u16 = 1; funcion principal() { var z: u16 = 0; a = 5 / z; a = 9; }",
    );
    assert_eq!(g["a"], 1, "la maquina debia detenerse antes de asignar");
}

#[test]
fn nombres_reservados() {
    let e = compilar_en("risc-16", "funcion __mul() { } funcion principal() { }").unwrap_err();
    assert!(e.contains("reservado"), "{e}");
}

#[test]
fn la_biblioteca_solo_entra_si_se_usa() {
    let sin = compilar_en("risc-16", "var a: u16; funcion principal() { a = 1 + 2; }").unwrap();
    assert!(!sin.ensamblador().contains("f___"), "{}", sin.ensamblador());
    let con = compilar_en(
        "risc-16",
        "var a: u16; funcion principal() { var x: u16 = 3; a = x * x; }",
    )
    .unwrap();
    assert!(con.ensamblador().contains("f___mul:"));
    assert!(!con.ensamblador().contains("f___divmod:"));
}
