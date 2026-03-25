//! Exportar la traza (RF-DEP-029).

use lorenz_dbg::exportar::{a_csv, a_json, exportar, Formato};
use lorenz_dbg::Depurador;
use lorenz_emu::Emulador;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};

fn con_traza(texto: &str) -> Depurador {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = lorenz_arch::cargar(&t).unwrap();
    let obj = Ensamblador::nuevo(&m)
        .unwrap()
        .ensamblar(&Fuente::nueva("p.s", texto))
        .unwrap();
    let mut emu = Emulador::nuevo(m).unwrap();
    for s in &obj.segmentos {
        emu.cargar_programa(s.direccion, &s.palabras).unwrap();
    }
    let mut d = Depurador::nuevo(emu);
    d.grabar(true);
    d.continuar(100);
    d
}

const P: &str = "LDI A, 200\nLDI B, 100\nADD A, B\nST [0x2000], A\nHLT\n";

#[test]
fn csv_tiene_cabecera_y_una_fila_por_paso() {
    let d = con_traza(P);
    let s = a_csv(d.traza().unwrap());
    let filas: Vec<&str> = s.lines().collect();
    assert_eq!(
        filas[0],
        "instante,pc,mnemonico,ciclos,registros,banderas,escrituras"
    );
    assert_eq!(filas.len(), 6);
    assert!(filas[1].starts_with("0,0x0,LDI,"));
    assert!(filas[1].contains("A:0x0>0xc8"));
}

#[test]
fn csv_incluye_escrituras_y_banderas() {
    let d = con_traza(P);
    let s = a_csv(d.traza().unwrap());
    assert!(s.contains("0x2000:0x0>0x2c"), "{s}");
    assert!(s.contains("C:0>1"), "{s}");
}

#[test]
fn csv_protege_las_comas() {
    let d = con_traza(P);
    let s = a_csv(d.traza().unwrap());
    // Los campos con varios cambios llevan ';' y no rompen las columnas.
    for fila in s.lines().skip(1) {
        assert_eq!(fila.matches(',').count(), 6, "{fila}");
    }
}

#[test]
fn json_tiene_la_estructura_esperada() {
    let d = con_traza(P);
    let s = a_json(d.traza().unwrap());
    assert!(s.starts_with("{\"formato\":\"lorenz-traza-0.1\",\"pasos\":["));
    assert!(s.trim_end().ends_with("]}"));
    assert_eq!(s.matches("\"instante\"").count(), 5);
    assert_eq!(s.matches('{').count(), s.matches('}').count());
    assert_eq!(s.matches('[').count(), s.matches(']').count());
}

#[test]
fn traza_vacia() {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = lorenz_arch::cargar(&t).unwrap();
    let mut d = Depurador::nuevo(Emulador::nuevo(m).unwrap());
    d.grabar(true);
    assert_eq!(a_csv(d.traza().unwrap()).lines().count(), 1);
    assert!(a_json(d.traza().unwrap()).contains("\"pasos\":[]"));
}

#[test]
fn el_formato_sale_de_la_extension() {
    assert_eq!(Formato::de_archivo("a/b/traza.CSV"), Some(Formato::Csv));
    assert_eq!(Formato::de_archivo("t.json"), Some(Formato::Json));
    assert_eq!(Formato::de_archivo("t.txt"), None);
    assert_eq!(Formato::de_archivo("sinextension"), None);
    let d = con_traza(P);
    assert_eq!(
        exportar(d.traza().unwrap(), Formato::Csv),
        a_csv(d.traza().unwrap())
    );
}
