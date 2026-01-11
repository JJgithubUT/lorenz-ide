//! Reconocimiento de instrucciones a partir de su semantica.

use lorenz_arch::carga::cargar_archivo;
use lorenz_arch::semantica::ast::OpBinario;
use lorenz_sgt::seleccion::*;

fn perfil(maq: &str) -> Perfil {
    let ruta = format!(
        "{}/../../ejemplos/{maq}/maquina.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    Perfil::de(&cargar_archivo(ruta).unwrap())
}

fn forma<'a>(p: &'a Perfil, mn: &str) -> &'a Forma {
    &p.instrucciones
        .iter()
        .find(|i| i.mnemonico == mn)
        .unwrap()
        .forma
}

fn r(n: &str) -> Reg {
    Reg::Operando(n.into())
}

#[test]
fn risc_16_tres_direcciones() {
    let p = perfil("risc-16");
    assert_eq!(
        forma(&p, "ADD"),
        &Forma::Alu {
            op: OpBinario::Suma,
            d: r("rd"),
            a: r("rs"),
            b: Fuente::Reg(r("rt"))
        }
    );
    assert_eq!(
        forma(&p, "ADDI"),
        &Forma::Alu {
            op: OpBinario::Suma,
            d: r("rd"),
            a: r("rs"),
            b: Fuente::Imm("imm".into())
        }
    );
    assert_eq!(
        forma(&p, "SLT"),
        &Forma::Alu {
            op: OpBinario::Menor,
            d: r("rd"),
            a: r("rs"),
            b: Fuente::Reg(r("rt"))
        }
    );
    assert_eq!(
        forma(&p, "LI"),
        &Forma::CargarImm {
            d: r("rd"),
            v: "imm".into()
        }
    );
    assert_eq!(
        forma(&p, "LW"),
        &Forma::Leer {
            d: r("rd"),
            dir: Dir::Indirecta {
                base: r("rs"),
                desp: Some("off".into())
            }
        }
    );
    assert_eq!(
        forma(&p, "SW"),
        &Forma::Escribir {
            s: r("rd"),
            dir: Dir::Indirecta {
                base: r("rs"),
                desp: Some("off".into())
            }
        }
    );
    assert_eq!(
        forma(&p, "J"),
        &Forma::Saltar(Salto::Absoluto("dir".into()))
    );
    assert_eq!(forma(&p, "JR"), &Forma::SaltarReg(r("rs")));
    assert_eq!(
        forma(&p, "BEQ"),
        &Forma::SaltarSi {
            cond: Cond::Comparar(OpBinario::Igual, r("rs"), r("rt")),
            salto: Salto::Relativo("off".into())
        }
    );
    assert_eq!(forma(&p, "HLT"), &Forma::Detener);
    assert_eq!(forma(&p, "NOP"), &Forma::Nada);
}

#[test]
fn ejemplo_8_dos_direcciones_y_bandera() {
    let p = perfil("ejemplo-8");
    assert_eq!(
        forma(&p, "ADD"),
        &Forma::Alu {
            op: OpBinario::Suma,
            d: r("rd"),
            a: r("rd"),
            b: Fuente::Reg(r("rs"))
        }
    );
    assert_eq!(
        forma(&p, "MOV"),
        &Forma::Mover {
            d: r("rd"),
            s: r("rs")
        }
    );
    assert_eq!(
        forma(&p, "LD"),
        &Forma::Leer {
            d: r("rd"),
            dir: Dir::Absoluta("dir".into())
        }
    );
    assert_eq!(
        forma(&p, "ST"),
        &Forma::Escribir {
            s: r("rs"),
            dir: Dir::Absoluta("dir".into())
        }
    );
    assert_eq!(
        forma(&p, "JZ"),
        &Forma::SaltarSi {
            cond: Cond::Bandera("Z".into(), true),
            salto: Salto::Absoluto("dir".into())
        }
    );
    // RETI asigna el contador de programa y habilita las interrupciones.
    assert_eq!(forma(&p, "RETI"), &Forma::RetornoInterrupcion);
    assert_eq!(forma(&p, "EI"), &Forma::Interrupciones(true));
    assert_eq!(forma(&p, "DI"), &Forma::Interrupciones(false));
}

#[test]
fn maquina_de_pila_queda_fuera() {
    let p = perfil("pila-8");
    assert_eq!(forma(&p, "ADD"), &Forma::Otra);
    assert_eq!(
        forma(&p, "JMP"),
        &Forma::Saltar(Salto::Absoluto("dir".into()))
    );
    assert!(p.detener().is_some());
}

#[test]
fn puertos_de_entrada_y_salida() {
    let p = perfil("ejemplo-8-serie");
    assert_eq!(
        forma(&p, "IN"),
        &Forma::Entrada {
            d: r("rd"),
            puerto: Puerto::Fijo(0x10)
        }
    );
    assert_eq!(
        forma(&p, "OUT"),
        &Forma::Salida {
            s: r("rs"),
            puerto: Puerto::Fijo(0x10)
        }
    );
}
