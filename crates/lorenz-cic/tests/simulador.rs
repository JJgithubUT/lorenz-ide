//! El simulador ciclo a ciclo sobre programas reales (RF-CIC-001, RF-CIC-002, RF-CIC-012,
//! RF-CIC-014, RF-CIC-025, RF-CIC-031).

mod comun;
use comun::*;

use lorenz_emu::Emulador;

/// LDI A,3 ; LDI B,1 ; ADD A,B ; HLT
const SUMA: &[u8] = &[LDI_A, 3, LDI_B, 1, ADD_AB, HLT];

#[test]
fn sin_pipeline_declarado_coincide_con_el_modo_funcional() {
    let mut s = simulador("", SUMA);
    s.ejecutar(1000).unwrap();
    let mut f =
        Emulador::nuevo(lorenz_arch::cargar(&format!("{}\n", texto_maquina())).unwrap()).unwrap();
    let palabras: Vec<u64> = SUMA.iter().map(|b| u64::from(*b)).collect();
    f.cargar_programa(0, &palabras).unwrap();
    f.ejecutar(1000).unwrap();
    assert_eq!(s.ciclos(), f.estado().ciclos());
    assert_eq!(s.metricas().instrucciones, f.estado().instrucciones());
    assert_eq!(s.emulador().estado().registro("A").unwrap(), 4);
}

#[test]
fn la_simulacion_no_cambia_el_resultado_del_programa() {
    let mut s = simulador(CINCO, SUMA);
    s.ejecutar(1000).unwrap();
    assert_eq!(s.emulador().estado().registro("A").unwrap(), 4);
    assert!(s.detenida());
}

#[test]
fn con_pipeline_hay_mas_instrucciones_en_vuelo_que_ciclos_por_instruccion() {
    let mut s = simulador(CINCO, &[HLT]);
    s.ejecutar(10).unwrap();
    // Una sola instruccion tarda las cinco etapas.
    assert_eq!(s.ciclos(), 5);
    assert_eq!(s.metricas().cpi(), Some(5.0));
}

#[test]
fn una_dependencia_detiene_sin_adelantamiento_y_no_con_el() {
    let programa = [LDI_A, 1, LDI_B, 2, ADD_AB, ADD_CA, HLT];
    let con = {
        let mut s = simulador(&format!("{CINCO}adelantamiento = true\n"), &programa);
        s.ejecutar(100).unwrap();
        s.metricas()
    };
    let sin = {
        let mut s = simulador(&format!("{CINCO}adelantamiento = false\n"), &programa);
        s.ejecutar(100).unwrap();
        s.metricas()
    };
    assert!(sin.ciclos > con.ciclos, "{sin:?} {con:?}");
    assert!(sin.esperas_datos > con.esperas_datos);
}

#[test]
fn el_salto_de_un_bucle_se_acierta_mejor_prediciendo_que_se_toma() {
    // inicio: ADD A,B ; JZ inicio   (Z queda encendida: salta siempre)
    let bucle = [ADD_AB, 0x60, 0x00, 0x00];
    let medir = |pred: &str| {
        let mut s = simulador(&format!("{CINCO}prediccion = \"{pred}\"\n"), &bucle);
        s.ejecutar(200).unwrap();
        s.metricas()
    };
    let no = medir("no-tomado");
    let si = medir("tomado");
    let dos = medir("2-bits");
    assert_eq!(no.saltos_acertados, 0);
    assert_eq!(si.saltos_acertados, si.saltos);
    assert!(si.ciclos < no.ciclos);
    assert!(dos.saltos_acertados > dos.saltos / 2, "{dos:?}");
    assert!(no.descartadas > 0 && si.descartadas == 0);
}

#[test]
fn una_memoria_lenta_alarga_la_ejecucion() {
    // LD B,[0x2000] tres veces.
    let lectura: Vec<u8> = vec![0x44, 0x00, 0x20, 0x44, 0x00, 0x20, 0x44, 0x00, 0x20, HLT];
    let rapida = {
        let mut s = simulador_con_ram_lenta(CINCO, 1, &lectura);
        s.ejecutar(100).unwrap();
        s.metricas()
    };
    let lenta = {
        let mut s = simulador_con_ram_lenta(CINCO, 6, &lectura);
        s.ejecutar(100).unwrap();
        s.metricas()
    };
    assert!(lenta.ciclos > rapida.ciclos + 5, "{lenta:?} {rapida:?}");
    assert!(lenta.ciclos_de_memoria > rapida.ciclos_de_memoria);
}

#[test]
fn la_cache_acelera_un_programa_que_lee_la_misma_direccion() {
    let mut lectura: Vec<u8> = [0x44, 0x00, 0x20].repeat(6);
    lectura.push(HLT);
    let cache = "[cache]\ntamano = 16\nlinea = 2\nvias = 2\nlatencia_acierto = 1\n";
    let sin = {
        let mut s = simulador_con_ram_lenta(CINCO, 6, &lectura);
        s.ejecutar(100).unwrap();
        s.metricas()
    };
    let con = {
        let mut s = simulador_con_ram_lenta(&format!("{CINCO}{cache}"), 6, &lectura);
        s.ejecutar(100).unwrap();
        s.metricas()
    };
    let c = con.cache.unwrap();
    assert_eq!((c.fallos_lectura, c.aciertos_lectura), (1, 5), "{c:?}");
    assert!(con.ciclos < sin.ciclos, "{con:?} {sin:?}");
}

#[test]
fn avanzar_y_retroceder_un_ciclo_muestra_lo_mismo() {
    let mut s = simulador(CINCO, &[ADD_AB, ADD_AB, ADD_AB, HLT]);
    let v0 = s.vista(0).unwrap();
    assert_eq!(v0.etapas[0].1.as_ref().unwrap().mnemonico, "ADD");
    assert!(v0.etapas[1].1.is_none(), "el pipeline empieza vacio");
    let v1 = s.avanzar_ciclo().unwrap();
    let v2 = s.avanzar_ciclo().unwrap();
    assert_eq!(s.retroceder_ciclo().unwrap(), v1);
    assert_eq!(s.retroceder_ciclo().unwrap(), v0);
    assert_eq!(s.retroceder_ciclo().unwrap(), v0, "no baja de cero");
    assert_ne!(v1, v2);
}

#[test]
fn la_vista_marca_las_detenciones_y_su_causa() {
    let mut s = simulador(
        &format!("{CINCO}adelantamiento = false\n"),
        &[ADD_AB, ADD_CA, HLT],
    );
    s.ejecutar(10).unwrap();
    let mut con_espera = None;
    for c in 0..s.ciclos() {
        let v = s.vista(c).unwrap();
        if v.texto().contains("espera datos") {
            con_espera = Some(v.texto());
            break;
        }
    }
    let t = con_espera.expect("alguna etapa debe esperar por datos");
    assert!(t.contains("espera datos (A)"), "{t}");
}

#[test]
fn los_saltos_mal_predichos_dejan_instrucciones_descartadas_en_la_vista() {
    let bucle = [ADD_AB, 0x60, 0x00, 0x00];
    let mut s = simulador(&format!("{CINCO}prediccion = \"no-tomado\"\n"), &bucle);
    s.ejecutar(6).unwrap();
    let hay = (0..s.ciclos()).any(|c| s.vista(c).unwrap().texto().contains("(descartada)"));
    assert!(hay);
}

#[test]
fn el_perfil_reparte_todos_los_ciclos() {
    let mut s = simulador(CINCO, &[LDI_A, 3, ADD_AB, ADD_AB, ADD_CA, HLT]);
    s.ejecutar(100).unwrap();
    let perfil = s.perfil();
    assert_eq!(perfil.iter().map(|f| f.ciclos).sum::<u64>(), s.ciclos());
    assert_eq!(perfil.iter().map(|f| f.ejecuciones).sum::<u64>(), 5);
}

#[test]
fn el_diagrama_tiene_una_fila_por_instruccion() {
    let mut s = simulador(CINCO, SUMA);
    s.ejecutar(100).unwrap();
    let d = s.diagrama(0, 10);
    assert_eq!(d.lines().count(), 1 + 4, "{d}");
    assert!(d.contains("LDI@0x0000"), "{d}");
}

#[test]
fn el_mismo_programa_da_las_mismas_metricas() {
    let a = {
        let mut s = simulador(CINCO, SUMA);
        s.ejecutar(100).unwrap();
        s.metricas()
    };
    let b = {
        let mut s = simulador(CINCO, SUMA);
        s.ejecutar(100).unwrap();
        s.metricas()
    };
    assert_eq!(a, b);
}

#[test]
fn el_simulador_guarda_los_accesos_a_datos_de_la_ultima_instruccion() {
    // LD [0x00] lee una palabra de la direccion 0; ADD no toca datos
    let mut s = simulador(CINCO, &[0x44, 0x00, 0x20, ADD_AB, HLT]);
    s.ejecutar_instruccion().unwrap();
    let a = s.ultimos_accesos();
    assert_eq!(a.len(), 1);
    assert!(!a[0].escritura);
    s.ejecutar_instruccion().unwrap();
    assert!(s.ultimos_accesos().is_empty());
}

#[test]
fn intercambiar_el_bus_deja_el_anterior_en_su_sitio() {
    use lorenz_cic::pipeline::Bus;
    let mut s = simulador(CINCO, &[ADD_AB, HLT]);
    let mut otro = Bus::default();
    otro.reservar(0, 7);
    s.intercambiar_bus(&mut otro);
    // ahora `otro` es el bus que tenia el simulador: vacio
    assert_eq!(otro.ciclos_ocupado(), 0);
    s.intercambiar_bus(&mut otro);
    assert_eq!(otro.ciclos_ocupado(), 7);
}
