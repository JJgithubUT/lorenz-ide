//! SGT en una maquina de 8 bits con banderas, dos direcciones y sin salto por registro (`sgt-8`).
mod comun;
use comun::*;

fn dif(fuente: &str) {
    igual_al_oraculo("sgt-8", fuente);
}

#[test]
fn suma_con_un_bucle() {
    let (g, _) = ejecutado(
        "sgt-8",
        "var s: u8; funcion principal() { var i: u8 = 1; mientras i <= 10 { s += i; i += 1; } }",
    );
    assert_eq!(g["s"], 55);
}

#[test]
fn aritmetica_y_bits() {
    dif("var a: u8; var b: u8; var c: u8; var d: u8; var e: u8;
         funcion principal() { var x: u8 = 100; var y: u8 = 23;
           a = x + y; b = x - y; c = y - x; d = (x & 0x0F) | (y ^ 0x55); e = ~x; }");
}

#[test]
fn comparaciones_sin_signo() {
    dif("var r: u8;
         funcion principal() { var a: u8 = 5; var b: u8 = 200;
           si a < b { r += 1; } si a <= b { r += 2; } si a > b { r += 4; } si a >= b { r += 8; }
           si a == b { r += 16; } si a != b { r += 32; } si b > a { r += 64; } si b >= a { r += 128; } }");
}

#[test]
fn comparaciones_de_iguales() {
    dif("var r: u8;
         funcion principal() { var a: u8 = 7; var b: u8 = 7;
           si a < b { r += 1; } si a <= b { r += 2; } si a > b { r += 4; } si a >= b { r += 8; }
           si a == b { r += 16; } si a != b { r += 32; } }");
}

#[test]
fn comparaciones_con_signo() {
    dif("var r: u8;
         funcion principal() { var a: i8 = -5; var b: i8 = 100;
           si a < b { r += 1; } si a <= b { r += 2; } si a > b { r += 4; } si a >= b { r += 8; }
           si b < a { r += 16; } si b > a { r += 32; } }");
}

#[test]
fn booleanos_como_valores() {
    dif("var a: bool; var b: bool; var c: bool; var d: bool;
         funcion principal() { var x: u8 = 3; var y: u8 = 9;
           a = x < y; b = x > y; c = x == 3 && y != 9; d = !(x == 3) || y == 9; }");
}

#[test]
fn bucles_romper_y_continuar() {
    dif("var s: u8; var n: u8;
         funcion principal() { var i: u8 = 0;
           bucle { i += 1; si i > 20 { romper; } si (i & 1) == 0 { continuar; } s += i; n += 1; } }");
}

#[test]
fn segun() {
    dif("var r: u8;
         funcion principal() { var i: u8 = 0;
           mientras i < 6 { segun i { 1 => { r += 1; } 3 => { r += 10; } 4 => { r += 100; } otro => { r += 1; } } i += 1; } }");
}

#[test]
fn estructuras_y_arreglos_con_indice_constante() {
    dif(
        "estructura P { x: u8, y: u8 } var p: P; var v: [u8; 4]; var s: u8;
         funcion principal() { p.x = 3; p.y = 4; v[0] = p.x; v[3] = p.y + 1; s = v[0] + v[3]; }",
    );
}

#[test]
fn escribe_por_la_uart() {
    let c = correr(
        "sgt-8",
        "funcion principal() { salida(0x10, 72); salida(0x10, 105); }",
        b"",
    );
    assert_eq!(c.texto, "Hi");
}

#[test]
fn salto_largo_con_banderas() {
    // El cuerpo del bucle es largo, pero sgt-8 salta a cualquier direccion: igual se cumple.
    let mut cuerpo = String::new();
    for k in 0..40 {
        cuerpo.push_str(&format!("a = a + {};\n", k % 7 + 1));
    }
    dif(&format!(
        "var a: u8; var n: u8;
         funcion principal() {{ var i: u8 = 0; mientras i < 3 {{ {cuerpo} i += 1; n += 1; }} }}"
    ));
}

#[test]
fn llamadas_sin_salto_por_registro() {
    dif("var a: u8; var b: u8;
         funcion sig(x: u8) -> u8 { devolver x + 1; }
         funcion principal() { a = sig(3); b = sig(sig(a)); }");
}

#[test]
fn funcion_llamada_desde_muchos_sitios() {
    let mut cuerpo = String::new();
    for k in 0..12 {
        cuerpo.push_str(&format!("r = r + inc({});\n", k));
    }
    dif(&format!(
        "var r: u8;
         funcion inc(x: u8) -> u8 {{ devolver x + 1; }}
         funcion principal() {{ {cuerpo} }}"
    ));
}

#[test]
fn funciones_que_llaman_a_funciones() {
    dif("var r: u8;
         funcion a(x: u8) -> u8 { devolver x + 1; }
         funcion b(x: u8) -> u8 { devolver a(x) + a(x + 1); }
         funcion c(x: u8) -> u8 { devolver b(x) + b(a(x)); }
         funcion principal() { r = c(2); }");
}

#[test]
fn llamada_en_un_bucle() {
    dif("var s: u8;
         funcion doble(x: u8) -> u8 { devolver x + x; }
         funcion principal() { var i: u8 = 0; mientras i < 10 { s += doble(i); i += 1; } }");
}

#[test]
fn funcion_sin_valor() {
    dif("var n: u8;
         funcion tick() { n += 1; }
         funcion principal() { tick(); tick(); tick(); }");
}

#[test]
fn multiplicar_y_dividir_con_la_biblioteca() {
    dif("var p: u8; var q: u8; var r: u8; var d: u8;
         funcion principal() { var a: u8 = 13; var b: u8 = 7; p = a * b; q = a / b; r = a % b; d = a << 2; }");
}
