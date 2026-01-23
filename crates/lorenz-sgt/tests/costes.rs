//! Coste en ciclos y presupuestos (RF-SGT-030, RF-SGT-031).
mod comun;
use comun::*;
use lorenz_emu::Emulador;

const MAQUINAS: [&str; 2] = ["risc-16", "sgt-16"];

fn ciclos_reales(maq: &str, fuente: &str) -> u64 {
    let c = compilar_en(maq, fuente).unwrap();
    let mut emu = Emulador::nuevo(maquina(maq)).unwrap();
    for (dir, w) in c.palabras() {
        emu.cargar_programa(dir, &[w]).unwrap();
    }
    emu.ejecutar(10_000_000).unwrap().ciclos
}

fn coste<'a>(c: &'a lorenz_sgt::compilar::Compilado, base: &str) -> &'a lorenz_sgt::costes::Coste {
    c.costes.iter().find(|k| k.base == base).unwrap()
}

#[test]
fn el_coste_de_codigo_recto_acota_lo_que_cuesta_ejecutarlo() {
    let vacio = "var a: u16; var b: u16; funcion principal() { }";
    let fuente = "var a: u16; var b: u16;
        funcion principal() { var x: u16 = 5; a = x + 3; si a > 4 { b = a ^ 2; } }";
    for m in MAQUINAS {
        let c = compilar_en(m, fuente).unwrap();
        let k = coste(&c, "principal");
        assert!(!k.bucles);
        let base = compilar_en(m, vacio).unwrap();
        // El arranque (limpiar los datos) no es de ninguna funcion: se compara solo lo que se anade.
        let real = ciclos_reales(m, fuente) - ciclos_reales(m, vacio);
        let estimado = k.total - coste(&base, "principal").total;
        assert!(real <= estimado, "{m}: real {real}, estimado {estimado}");
        assert!(
            estimado <= real * 4,
            "{m}: estimado {estimado} frente a {real}"
        );
    }
}

#[test]
fn una_llamada_suma_el_coste_de_la_llamada() {
    let fuente = "var a: u16;
        funcion g(x: u16) -> u16 { devolver x + 1; }
        funcion principal() { a = g(1); }";
    for m in MAQUINAS {
        let c = compilar_en(m, fuente).unwrap();
        let (p, g) = (coste(&c, "principal"), coste(&c, "g"));
        assert_eq!(p.total, p.propios + g.total, "{m}");
        assert!(g.total > 0);
    }
}

#[test]
fn los_bucles_se_senalan_y_se_heredan() {
    let fuente = "var a: u16;
        funcion h() { var i: u16 = 0; mientras i < 3 { i += 1; } }
        funcion principal() { h(); a = 1; }";
    for m in MAQUINAS {
        let c = compilar_en(m, fuente).unwrap();
        assert!(coste(&c, "h").bucles);
        assert!(coste(&c, "principal").bucles);
    }
}

#[test]
fn optimizar_baja_el_coste() {
    let fuente = "var a: u16;
        funcion principal() { var x: u16 = 7; si 1 < 2 { a = x + 0; } sino { a = x * 3; } }";
    for m in MAQUINAS {
        let sin = compilar_nivel(m, fuente, 0).unwrap();
        let con = compilar_nivel(m, fuente, 2).unwrap();
        assert!(
            coste(&con, "principal").total < coste(&sin, "principal").total,
            "{m}"
        );
    }
}

#[test]
fn un_presupuesto_holgado_se_acepta() {
    for m in MAQUINAS {
        compilar_en(
            m,
            "var a: u16; @presupuesto(10000) funcion principal() { a = 1; }",
        )
        .unwrap();
    }
}

#[test]
fn un_presupuesto_excedido_falla_en_terminos_de_la_maquina() {
    for m in MAQUINAS {
        let e = compilar_en(
            m,
            "var a: u16; @presupuesto(3) funcion principal() { a = 1; a += 2; }",
        )
        .unwrap_err();
        assert!(e.contains("presupuesto es de 3"), "{e}");
        assert!(e.contains("principal"), "{e}");
        assert!(e.contains("t.sgt:1:"), "{e}");
    }
}

#[test]
fn el_presupuesto_cuenta_lo_que_llama() {
    let e = compilar_en(
        "risc-16",
        "var a: u16;
         funcion g() { a = 1; a += 1; a += 2; a += 3; }
         @presupuesto(10) funcion principal() { g(); }",
    )
    .unwrap_err();
    assert!(e.contains("principal"), "{e}");
}

#[test]
fn el_presupuesto_debe_ser_una_constante_positiva() {
    for fuente in [
        "@presupuesto(0) funcion principal() { }",
        "@presupuesto() funcion principal() { }",
        "@presupuesto(1, 2) funcion principal() { }",
    ] {
        assert!(compilar_en("risc-16", fuente).is_err(), "{fuente}");
    }
}

#[test]
fn un_operador_resuelto_con_la_biblioteca_cuenta_su_rutina() {
    // `a * b` no tiene instruccion en risc-16: se resuelve llamando a `__mul`, que itera.
    let fuente = "var r: u16; var a: u16; var b: u16;
        funcion principal() { r = a * b; }";
    let c = compilar_en("risc-16", fuente).unwrap();
    let k = coste(&c, "principal");
    assert!(k.bucles);
    assert!(k.total > k.propios);
    // sgt-16 tiene MUL: ahi se queda en una instruccion.
    let c = compilar_en("sgt-16", fuente).unwrap();
    assert!(!coste(&c, "principal").bucles);
}
