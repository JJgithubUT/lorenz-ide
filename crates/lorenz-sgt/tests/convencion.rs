//! La convencion de llamada documentada coincide con la que el compilador usa (RF-SGT-027).
mod comun;
use comun::*;
use lorenz_sgt::convencion::describir;

#[test]
fn describe_los_registros_que_el_compilador_usa_de_verdad() {
    for m in ["risc-16", "sgt-16"] {
        let doc = describir(&maquina(m)).unwrap();
        let c = compilar_en(
            m,
            "var a: u16; funcion g(x: u16) -> u16 { devolver x + 1; } funcion principal() { a = g(3); }",
        )
        .unwrap();
        // El valor devuelto vuelve en el primer registro de la tabla.
        let retorno = doc
            .lines()
            .find(|l| l.contains("valor de retorno"))
            .and_then(|l| l.split('`').nth(1))
            .unwrap();
        assert!(
            c.ensamblador().contains(retorno),
            "{m}: el registro de retorno `{retorno}` no aparece en el codigo generado"
        );
    }
}

#[test]
fn los_documentos_de_docs_estan_al_dia() {
    for m in ["risc-16", "sgt-16"] {
        let ruta = format!(
            "{}/../../docs/sgt/convencion-{m}.md",
            env!("CARGO_MANIFEST_DIR")
        );
        let escrito = std::fs::read_to_string(&ruta).unwrap_or_default();
        assert_eq!(
            escrito,
            describir(&maquina(m)).unwrap(),
            "{ruta} esta desactualizado: regeneralo con `lorenz convencion ejemplos/{m}/maquina.toml -o docs/sgt/convencion-{m}.md`"
        );
    }
}

#[test]
fn una_maquina_sin_registros_suficientes_no_tiene_convencion() {
    let e = describir(&maquina("pila-8")).unwrap_err();
    assert!(e.contains("registros"), "{e}");
}
