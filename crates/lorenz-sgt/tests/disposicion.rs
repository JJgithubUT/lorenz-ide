//! Disposicion fijada de una estructura con `@ n` (RF-SGT-007): el desplazamiento llega al
//! codigo generado y los solapes se rechazan.

mod comun;
use comun::*;

const MAQUINAS: [&str; 2] = ["risc-16", "sgt-16"];

fn estructura<'a>(
    c: &'a lorenz_sgt::compilar::Compilado,
    nombre: &str,
) -> &'a lorenz_sgt::hir::Estructura {
    c.modulo
        .estructuras
        .iter()
        .find(|e| e.nombre == nombre)
        .unwrap_or_else(|| panic!("no hay estructura {nombre}"))
}

#[test]
fn un_campo_fijado_queda_en_su_desplazamiento_y_el_siguiente_continua() {
    for m in MAQUINAS {
        let c = compilar_en(
            m,
            "estructura E { a: u16, b: u16 @ 4, c: u16 } var e: E; funcion principal() { }",
        )
        .unwrap();
        let e = estructura(&c, "E");
        let d: Vec<u64> = e.campos.iter().map(|k| k.desplazamiento).collect();
        assert_eq!(d, [0, 4, 5], "{m}");
        assert_eq!(e.tamano, 6, "{m}");
    }
}

#[test]
fn el_desplazamiento_puede_ser_una_constante() {
    let c = compilar_en(
        "risc-16",
        "const BASE: u16 = 3; estructura E { a: u16, b: u16 @ BASE + 1 } var e: E; funcion principal() { }",
    )
    .unwrap();
    assert_eq!(estructura(&c, "E").campos[1].desplazamiento, 4);
}

#[test]
fn los_campos_se_leen_y_escriben_en_el_lugar_fijado() {
    for m in MAQUINAS {
        let fuente = "estructura E { a: u16, b: u16 @ 3 } var e: E; var r: u16;
             funcion principal() { e.a = 7; e.b = 9; r = e.a + e.b; }";
        let c = compilar_en(m, fuente).unwrap();
        let mut emu = lorenz_emu::Emulador::nuevo(maquina(m)).unwrap();
        for (dir, w) in c.palabras() {
            emu.cargar_programa(dir, &[w]).unwrap();
        }
        emu.ejecutar(1_000_000).unwrap();
        let i = c
            .modulo
            .globales
            .iter()
            .position(|g| g.nombre == "e")
            .unwrap();
        let base = c.disposicion.globales[i];
        let leer = |o: u64| emu.memoria().leer_privilegiado(base + o).unwrap();
        assert_eq!((leer(0), leer(3)), (7, 9), "{m}");
        assert_eq!(leer(1), 0, "el hueco no se toca en {m}");
        igual_al_oraculo(m, fuente);
    }
}

#[test]
fn campos_fijados_en_orden_inverso_son_validos_si_no_se_solapan() {
    let c = compilar_en(
        "risc-16",
        "estructura E { a: u16 @ 2, b: u16 @ 0 } var e: E; funcion principal() { }",
    )
    .unwrap();
    let e = estructura(&c, "E");
    assert_eq!(
        (e.campos[0].desplazamiento, e.campos[1].desplazamiento),
        (2, 0)
    );
    assert_eq!(e.tamano, 3);
}

#[test]
fn dos_campos_que_se_solapan_se_rechazan() {
    let e = compilar_en(
        "risc-16",
        "estructura E { a: [u16; 3], b: u16 @ 2 } funcion principal() { }",
    )
    .unwrap_err();
    assert!(e.contains("se solapan"), "{e}");
}

#[test]
fn un_desplazamiento_que_no_es_constante_se_rechaza() {
    let e = compilar_en(
        "risc-16",
        "var n: u16; estructura E { a: u16 @ n } funcion principal() { }",
    )
    .unwrap_err();
    assert!(e.contains("`n`"), "{e}");
}

#[test]
fn el_tamano_de_un_arreglo_de_estructuras_usa_el_hueco() {
    let (g, _) = ejecutado(
        "risc-16",
        "estructura E { a: u16, b: u16 @ 2 } var t: [E; 3]; var r: u16;
         funcion principal() { var i: u16 = 0; mientras i < 3 { t[i].a = i; t[i].b = i + 10; i += 1; }
           r = t[0].b + t[1].a + t[2].b; }",
    );
    assert_eq!(g["r"], 10 + 1 + 12);
}
