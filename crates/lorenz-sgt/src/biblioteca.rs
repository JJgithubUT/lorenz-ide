//! Rutinas de apoyo escritas en SGT (RF-SGT-041).
//!
//! Cuando la maquina no tiene una instruccion para multiplicar, dividir, desplazar o saturar, el
//! compilador llama a estas funciones. Estan escritas solo con sumas, restas, operaciones de bits y
//! comparaciones, que toda maquina util tiene, y se compilan con el mismo compilador. Solo entran
//! en el programa las que se usan (ver [`crate::podar`]).
//!
//! Sus nombres empiezan por `__`, reservado para el compilador. Dividir entre cero detiene la
//! maquina (decision D-062).
//!
//! Ademas hay una biblioteca minima de memoria y de textos (`mem_*` y `cad_*`) que el programa
//! puede llamar; tambien se poda si no se usa. Un texto es un arreglo de enteros terminado en cero,
//! con un caracter por elemento.

use crate::ast::Programa;
use crate::diagnostico::Diagnostico;
use crate::sintaxis::analizar_programa;

pub const ARCHIVO: &str = "<biblioteca>";

const PLANTILLA: &str = r#"
var __resto: u{W};

funcion __detener() { }

funcion __mul(a: u{W}, b: u{W}) -> u{W} {
    var r: u{W} = 0;
    var x: u{W} = a;
    var m: u{W} = 1;
    mientras m != 0 {
        si (b & m) != 0 { r += x; }
        x += x;
        m += m;
    }
    devolver r;
}

funcion __shr1(x: u{W}) -> u{W} {
    var r: u{W} = 0;
    var bajo: u{W} = 1;
    var alto: u{W} = 2;
    mientras alto != 0 {
        si (x & alto) != 0 { r |= bajo; }
        bajo = alto;
        alto += alto;
    }
    devolver r;
}

funcion __shl(a: u{W}, k: u{W}) -> u{W} {
    si k >= {W} { devolver 0; }
    var r: u{W} = a;
    mientras k != 0 { r += r; k -= 1; }
    devolver r;
}

funcion __shr(a: u{W}, k: u{W}) -> u{W} {
    si k >= {W} { devolver 0; }
    var r: u{W} = a;
    mientras k != 0 { r = __shr1(r); k -= 1; }
    devolver r;
}

funcion __sar(a: i{W}, k: u{W}) -> i{W} {
    si k >= {W} {
        si a < 0 { devolver -1; }
        devolver 0;
    }
    var r: u{W} = a como u{W};
    var neg: bool = a < 0;
    mientras k != 0 {
        r = __shr1(r);
        si neg { r |= {TOP}; }
        k -= 1;
    }
    devolver r como i{W};
}

funcion __divmod(a: u{W}, b: u{W}) -> u{W} {
    si b == 0 { __detener(); }
    var r: u{W} = a;
    var q: u{W} = 0;
    si b > r {
        __resto = r;
        devolver 0;
    }
    var d: [u{W}; {W}];
    var k: [u{W}; {W}];
    var n: u{W} = 0;
    d[0] = b;
    k[0] = 1;
    mientras d[n] <= r - d[n] {
        d[n + 1] = d[n] + d[n];
        k[n + 1] = k[n] + k[n];
        n += 1;
    }
    bucle {
        si r >= d[n] {
            r -= d[n];
            q += k[n];
        }
        si n == 0 { romper; }
        n -= 1;
    }
    __resto = r;
    devolver q;
}

funcion __divu(a: u{W}, b: u{W}) -> u{W} {
    devolver __divmod(a, b);
}

funcion __modu(a: u{W}, b: u{W}) -> u{W} {
    __divmod(a, b);
    devolver __resto;
}

funcion __abs(a: i{W}) -> u{W} {
    si a < 0 { devolver 0 - (a como u{W}); }
    devolver a como u{W};
}

funcion __divs(a: i{W}, b: i{W}) -> i{W} {
    var q: u{W} = __divmod(__abs(a), __abs(b));
    si (a < 0) != (b < 0) { devolver (0 - q) como i{W}; }
    devolver q como i{W};
}

funcion __mods(a: i{W}, b: i{W}) -> i{W} {
    __divmod(__abs(a), __abs(b));
    si a < 0 { devolver (0 - __resto) como i{W}; }
    devolver __resto como i{W};
}

funcion __sumasat_u(a: u{W}, b: u{W}) -> u{W} {
    var r: u{W} = a + b;
    si r < a { devolver {MAX}; }
    devolver r;
}

funcion __restasat_u(a: u{W}, b: u{W}) -> u{W} {
    si a < b { devolver 0; }
    devolver a - b;
}

funcion __sumasat_s(a: i{W}, b: i{W}) -> i{W} {
    var r: i{W} = a + b;
    si (a >= 0) == (b >= 0) && (r >= 0) != (a >= 0) {
        si a >= 0 { devolver {MAXS}; }
        devolver -{TOP};
    }
    devolver r;
}

funcion __restasat_s(a: i{W}, b: i{W}) -> i{W} {
    var r: i{W} = a - b;
    si (a >= 0) != (b >= 0) && (r >= 0) != (a >= 0) {
        si a >= 0 { devolver {MAXS}; }
        devolver -{TOP};
    }
    devolver r;
}

funcion mem_llenar(destino: *u{W}, valor: u{W}, n: u{W}) {
    var i: u{W} = 0;
    mientras i < n {
        destino[i] = valor;
        i += 1;
    }
}

funcion mem_copiar(destino: *u{W}, origen: *u{W}, n: u{W}) {
    var i: u{W} = 0;
    mientras i < n {
        destino[i] = origen[i];
        i += 1;
    }
}

funcion mem_comparar(a: *u{W}, b: *u{W}, n: u{W}) -> i{W} {
    var i: u{W} = 0;
    mientras i < n {
        si a[i] != b[i] {
            si a[i] < b[i] { devolver -1; }
            devolver 1;
        }
        i += 1;
    }
    devolver 0;
}

funcion cad_longitud(t: *u{W}) -> u{W} {
    var n: u{W} = 0;
    mientras t[n] != 0 { n += 1; }
    devolver n;
}

funcion cad_copiar(destino: *u{W}, origen: *u{W}) -> u{W} {
    var i: u{W} = 0;
    mientras origen[i] != 0 {
        destino[i] = origen[i];
        i += 1;
    }
    destino[i] = 0;
    devolver i;
}

funcion cad_comparar(a: *u{W}, b: *u{W}) -> i{W} {
    var i: u{W} = 0;
    mientras a[i] != 0 && a[i] == b[i] { i += 1; }
    si a[i] == b[i] { devolver 0; }
    si a[i] < b[i] { devolver -1; }
    devolver 1;
}
"#;

/// Texto de la biblioteca para palabras de `w` bits.
pub fn fuente(w: u32) -> String {
    let top = 1u128 << (w - 1);
    PLANTILLA
        .replace("{W}", &w.to_string())
        .replace("{TOP}", &top.to_string())
        .replace("{MAXS}", &(top - 1).to_string())
        .replace("{MAX}", &((top << 1) - 1).to_string())
}

pub fn programa(w: u32) -> Result<Programa, Diagnostico> {
    analizar_programa(ARCHIVO, &fuente(w))
}

/// Funciones de la biblioteca que el programa puede llamar por su nombre.
pub const PUBLICAS: [&str; 6] = [
    "mem_llenar",
    "mem_copiar",
    "mem_comparar",
    "cad_longitud",
    "cad_copiar",
    "cad_comparar",
];

/// Si el nombre es de la biblioteca de apoyo o de la biblioteca publica.
pub fn es_reservado(nombre: &str) -> bool {
    nombre.starts_with("__") || PUBLICAS.contains(&nombre)
}
