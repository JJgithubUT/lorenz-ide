//! Pruebas del nucleo del depurador sobre el ejemplo de 8 bits.

use lorenz_dbg::{Depurador, Parada, Vigilar};
use lorenz_emu::Emulador;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};

const PROGRAMA: &str = "\
inicio: LDI A, 1
        LDI B, 2
suma:   ADD A, B
        ST [0x2000], A
        LD B, [0x2000]
        HLT
";

fn depurador(texto: &str) -> Depurador {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = lorenz_arch::cargar(&t).unwrap();
    let obj = Ensamblador::nuevo(&m)
        .unwrap()
        .ensamblar(&Fuente::nueva("prueba.s", texto))
        .unwrap();
    let mut emu = Emulador::nuevo(m).unwrap();
    for s in &obj.segmentos {
        emu.cargar_programa(s.direccion, &s.palabras).unwrap();
    }
    let mut d = Depurador::nuevo(emu);
    d.con_fuente(&obj, &[("prueba.s".to_string(), texto.to_string())]);
    d
}

#[test]
fn un_paso_ejecuta_una_instruccion() {
    let mut d = depurador(PROGRAMA);
    assert_eq!(d.paso(1), Parada::Paso { instrucciones: 1 });
    assert_eq!(d.emulador().estado().registro("A").unwrap(), 1);
}

#[test]
fn varios_pasos() {
    let mut d = depurador(PROGRAMA);
    assert_eq!(d.paso(3), Parada::Paso { instrucciones: 3 });
    assert_eq!(d.emulador().estado().registro("A").unwrap(), 3);
}

#[test]
fn continuar_llega_al_final() {
    let mut d = depurador(PROGRAMA);
    assert_eq!(d.continuar(1000), Parada::Detenida);
}

#[test]
fn ruptura_por_etiqueta() {
    let mut d = depurador(PROGRAMA);
    let dir = d.resolver("suma").unwrap();
    let id = d.poner_ruptura(dir, None).unwrap();
    assert_eq!(d.continuar(1000), Parada::Ruptura { id, direccion: dir });
    assert_eq!(d.rupturas()[0].aciertos, 1);
}

#[test]
fn ruptura_por_archivo_y_linea() {
    let d = depurador(PROGRAMA);
    assert_eq!(
        d.resolver("prueba.s:3").unwrap(),
        d.resolver("suma").unwrap()
    );
}

#[test]
fn linea_sin_codigo_usa_la_siguiente() {
    let d = depurador(&format!("; comentario\n\n{PROGRAMA}"));
    assert_eq!(d.resolver("prueba.s:1").unwrap(), 0);
}

#[test]
fn linea_pasada_el_final_es_error() {
    let d = depurador(PROGRAMA);
    assert!(d.resolver("prueba.s:99").is_err());
}

#[test]
fn ruptura_condicional() {
    let mut d = depurador(PROGRAMA);
    let dir = d.resolver("suma").unwrap();
    d.poner_ruptura(dir, Some("A == 99")).unwrap();
    assert_eq!(d.continuar(1000), Parada::Detenida);
    let mut d = depurador(PROGRAMA);
    let id = d.poner_ruptura(dir, Some("A == 1 && B == 2")).unwrap();
    assert_eq!(d.continuar(1000), Parada::Ruptura { id, direccion: dir });
}

#[test]
fn ruptura_desactivada_no_detiene() {
    let mut d = depurador(PROGRAMA);
    let dir = d.resolver("suma").unwrap();
    let id = d.poner_ruptura(dir, None).unwrap();
    d.activar(id, false).unwrap();
    assert_eq!(d.continuar(1000), Parada::Detenida);
}

#[test]
fn borrar_ruptura() {
    let mut d = depurador(PROGRAMA);
    let id = d.poner_ruptura(2, None).unwrap();
    d.borrar(id).unwrap();
    assert!(d.rupturas().is_empty());
    assert!(d.borrar(id).is_err());
}

#[test]
fn hasta_una_direccion() {
    let mut d = depurador(PROGRAMA);
    let dir = d.resolver("suma").unwrap();
    assert_eq!(d.hasta(dir, 1000), Parada::Llegada { direccion: dir });
}

#[test]
fn limite_de_instrucciones() {
    let mut d = depurador("a: ADD A, B\n JZ a\n");
    assert_eq!(d.continuar(10), Parada::Limite { instrucciones: 10 });
}

#[test]
fn observacion_de_escritura() {
    let mut d = depurador(PROGRAMA);
    let id = d.observar(0x2000, Vigilar::Escritura).unwrap();
    match d.continuar(1000) {
        Parada::Observacion {
            id: i,
            acceso,
            valor,
        } => {
            assert_eq!(i, id);
            assert_eq!(acceso.direccion, 0x2000);
            assert_eq!(valor, Some(3));
        }
        otra => panic!("{otra:?}"),
    }
}

#[test]
fn observacion_de_lectura_ignora_la_escritura() {
    let mut d = depurador(PROGRAMA);
    d.observar(0x2000, Vigilar::Lectura).unwrap();
    match d.continuar(1000) {
        Parada::Observacion { acceso, .. } => {
            assert_eq!(acceso.pc, d.resolver("prueba.s:5").unwrap());
        }
        otra => panic!("{otra:?}"),
    }
}

#[test]
fn observar_fuera_de_memoria_es_error() {
    let mut d = depurador(PROGRAMA);
    assert!(d.observar(0x0FFF_FFFF, Vigilar::Acceso).is_err());
}

#[test]
fn quitar_la_observacion_deja_de_vigilar() {
    let mut d = depurador(PROGRAMA);
    let id = d.observar(0x2000, Vigilar::Acceso).unwrap();
    d.borrar(id).unwrap();
    assert_eq!(d.continuar(1000), Parada::Detenida);
}

#[test]
fn fallo_indica_la_instruccion() {
    let mut d = depurador("ST [0x0010], A\nHLT\n");
    match d.continuar(10) {
        Parada::Fallo {
            direccion,
            instruccion,
            ..
        } => {
            assert_eq!(direccion, 0);
            assert!(instruccion.unwrap().starts_with("ST"));
        }
        otra => panic!("{otra:?}"),
    }
}

#[test]
fn evaluar_expresiones() {
    let mut d = depurador(PROGRAMA);
    d.paso(2);
    assert_eq!(d.evaluar("A + B").unwrap(), 3);
    assert_eq!(d.evaluar("suma").unwrap(), 4);
}

#[test]
fn salir_sin_pila_es_error() {
    let m = lorenz_arch::cargar(
        r#"
[maquina]
nombre = "sinpila"
formato = "0.1"
bits_palabra = 8
bits_direccion = 8
endianness = "little"
arranque = 0x00

[[registro]]
nombre = "PC"
bits = 8
rol = "pc"

[[region]]
nombre = "ram"
desde = 0x00
hasta = 0xFF
acceso = "rw"

[[instruccion]]
mnemonico = "NOP"
codigo = "0000 0000"
"#,
    )
    .unwrap();
    let mut d = Depurador::nuevo(Emulador::nuevo(m).unwrap());
    assert!(d.salir(10).is_err());
}

#[test]
fn se_pueden_recoger_las_instrucciones_ejecutadas() {
    let mut d = depurador(PROGRAMA);
    assert!(d.tomar_pasos().is_empty(), "sin pedirlo no se recoge nada");
    d.recoger_pasos(true);
    d.paso(2);
    let primeros = d.tomar_pasos();
    assert_eq!(
        primeros
            .iter()
            .map(|p| p.mnemonico.as_str())
            .collect::<Vec<_>>(),
        ["LDI", "LDI"]
    );
    assert_eq!(primeros[0].direccion, 0);
    assert!(d.tomar_pasos().is_empty(), "tomar vacia la lista");
    d.continuar(100);
    let resto: Vec<_> = d.tomar_pasos().into_iter().map(|p| p.mnemonico).collect();
    assert_eq!(resto, ["ADD", "ST", "LD", "HLT"]);
    d.recoger_pasos(false);
    assert!(d.tomar_pasos().is_empty());
}
