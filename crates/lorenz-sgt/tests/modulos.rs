//! Modulos con importacion explicita de simbolos (RF-SGT-018).
mod comun;
use comun::*;

use std::collections::HashMap;

use lorenz_sgt::analisis::Opciones;
use lorenz_sgt::compilar::compilar_con;

/// Compila `principal` con los modulos dados (nombre y texto) y ejecuta el resultado.
fn con_modulos(
    maq: &str,
    principal: &str,
    modulos: &[(&str, &str)],
) -> Result<HashMap<String, i128>, String> {
    let m = maquina(maq);
    let mapa: HashMap<String, String> = modulos
        .iter()
        .map(|(n, t)| (n.to_string(), t.to_string()))
        .collect();
    let cargador = |n: &str| mapa.get(n).map(|t| (format!("{n}.sgt"), t.clone()));
    let c = compilar_con("t.sgt", principal, &m, Opciones::default(), &cargador).map_err(|e| {
        e.iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let mut emu = lorenz_emu::Emulador::nuevo(m).unwrap();
    for (d, w) in c.palabras() {
        emu.cargar_programa(d, &[w]).unwrap();
    }
    emu.ejecutar(5_000_000)
        .unwrap_or_else(|e| panic!("{e}\n{}", c.ensamblador()));
    let mut out = HashMap::new();
    for (i, g) in c.modulo.globales.iter().enumerate() {
        if g.tipo.es_escalar() {
            out.insert(g.nombre.clone(), leer_global(&emu, &c, i, &maquina(maq)));
        }
    }
    Ok(out)
}

const GEOM: &str = "
const ESCALA: u16 = 3;
estructura Punto { x: u16, y: u16 }
var contador: u16;
funcion auxiliar(a: u16) -> u16 { contador += 1; devolver a * ESCALA; }
funcion norma1(p: *Punto) -> u16 { devolver auxiliar((*p).x) + auxiliar((*p).y); }
funcion cuantas() -> u16 { devolver contador; }
";

#[test]
fn importa_funciones_estructuras_y_constantes() {
    for m in ["risc-16", "sgt-16"] {
        let g = con_modulos(
            m,
            "importar geom { Punto, norma1, cuantas, ESCALA };
             var r: u16; var n: u16; var e: u16; var p: Punto;
             funcion principal() { p.x = 4; p.y = 5; r = norma1(&p); n = cuantas(); e = ESCALA; }",
            &[("geom", GEOM)],
        )
        .unwrap();
        assert_eq!(g["r"], 27, "{m}");
        assert_eq!(g["n"], 2, "{m}");
        assert_eq!(g["e"], 3, "{m}");
    }
}

#[test]
fn lo_no_importado_es_privado() {
    let e = con_modulos(
        "risc-16",
        "importar geom { norma1 };
         var r: u16;
         funcion principal() { r = auxiliar(2); }",
        &[("geom", GEOM)],
    )
    .unwrap_err();
    assert!(e.contains("auxiliar"), "{e}");
}

#[test]
fn los_nombres_privados_de_un_modulo_no_chocan_con_los_del_programa() {
    let g = con_modulos(
        "risc-16",
        "importar geom { cuantas, norma1, Punto };
         var contador: u16; var r: u16; var p: Punto;
         funcion auxiliar(a: u16) -> u16 { devolver a + 100; }
         funcion principal() { contador = 7; p.x = 1; p.y = 1; r = norma1(&p) + auxiliar(1); r += cuantas(); }",
        &[("geom", GEOM)],
    )
    .unwrap();
    // norma1 = 3 + 3, auxiliar(1) = 101, cuantas() = 2: el `contador` del programa no se toca.
    assert_eq!(g["r"], 6 + 101 + 2);
    assert_eq!(g["contador"], 7);
}

#[test]
fn un_modulo_importa_a_otro() {
    let g = con_modulos(
        "risc-16",
        "importar b { doble_mas_uno };
         var r: u16;
         funcion principal() { r = doble_mas_uno(10); }",
        &[
            ("b", "importar a { doble }; funcion doble_mas_uno(x: u16) -> u16 { devolver doble(x) + 1; }"),
            ("a", "funcion doble(x: u16) -> u16 { devolver x + x; }"),
        ],
    )
    .unwrap();
    assert_eq!(g["r"], 21);
}

#[test]
fn dos_modulos_con_funciones_del_mismo_nombre() {
    // Cada modulo tiene su `valor`; el programa importa uno y llama al otro por otra via.
    let g = con_modulos(
        "risc-16",
        "importar a { valor };
         importar c { total };
         var r: u16;
         funcion principal() { r = valor() * 10 + total(); }",
        &[
            ("a", "funcion valor() -> u16 { devolver 1; }"),
            (
                "c",
                "importar b { dato }; funcion total() -> u16 { devolver dato() + 5; }",
            ),
            ("b", "funcion dato() -> u16 { devolver 2; }"),
        ],
    )
    .unwrap();
    assert_eq!(g["r"], 17);
}

#[test]
fn importar_dos_veces_el_mismo_nombre_es_un_error() {
    let e = con_modulos(
        "risc-16",
        "importar a { valor };
         importar b { valor };
         funcion principal() { }",
        &[
            ("a", "funcion valor() -> u16 { devolver 1; }"),
            ("b", "funcion valor() -> u16 { devolver 2; }"),
        ],
    )
    .unwrap_err();
    assert!(e.contains("ya esta importado"), "{e}");
}

#[test]
fn simbolo_que_el_modulo_no_define() {
    let e = con_modulos(
        "risc-16",
        "importar geom { nada };
         funcion principal() { }",
        &[("geom", GEOM)],
    )
    .unwrap_err();
    assert!(e.contains("no define `nada`"), "{e}");
    assert!(e.contains("norma1"), "{e}");
}

#[test]
fn modulo_que_no_existe() {
    let e = con_modulos(
        "risc-16",
        "importar fantasma { x }; funcion principal() { }",
        &[],
    )
    .unwrap_err();
    assert!(e.contains("no se encontro el modulo `fantasma`"), "{e}");
}

#[test]
fn ciclo_de_importaciones() {
    let e = con_modulos(
        "risc-16",
        "importar a { f }; funcion principal() { f(); }",
        &[
            ("a", "importar b { g }; funcion f() { g(); }"),
            ("b", "importar a { f }; funcion g() { }"),
        ],
    )
    .unwrap_err();
    assert!(e.contains("en ciclo: a -> b -> a"), "{e}");
}

#[test]
fn los_errores_citan_el_archivo_del_modulo() {
    let e = con_modulos(
        "risc-16",
        "importar roto { f }; funcion principal() { f(); }",
        &[("roto", "funcion f() { var x: u16 = verdadero; }")],
    )
    .unwrap_err();
    assert!(e.starts_with("roto.sgt:1:"), "{e}");
}

#[test]
fn un_modulo_puede_usar_la_biblioteca() {
    let g = con_modulos(
        "risc-16",
        "importar txt { largo };
         var r: u16;
         funcion principal() { r = largo(); }",
        &[(
            "txt",
            "var s: [u16; 8] = \"hola\"; funcion largo() -> u16 { devolver cad_longitud(&s[0]); }",
        )],
    )
    .unwrap();
    assert_eq!(g["r"], 4);
}

#[test]
fn la_ocho_bits_tambien_importa() {
    let g = con_modulos(
        "sgt-8",
        "importar m { suma3 };
         var r: u8;
         funcion principal() { r = suma3(1, 2, 3); }",
        &[(
            "m",
            "funcion suma3(a: u8, b: u8, c: u8) -> u8 { devolver a + b + c; }",
        )],
    )
    .unwrap();
    assert_eq!(g["r"], 6);
}
