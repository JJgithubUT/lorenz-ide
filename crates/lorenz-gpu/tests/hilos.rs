use lorenz_gpu::*;

fn cfg(extra: &str) -> ConfigHilos {
    let t = format!("[gpu]\n[gpu.hilos]\n{extra}");
    cargar_config(&t).unwrap().unwrap().hilos
}

fn correr(fuente: &str, extra: &str) -> EjecucionHilos {
    let k = ensamblar_kernel(fuente).unwrap();
    let (g, h) = k.lanzar.unwrap();
    let mut e = EjecucionHilos::nueva(cfg(extra), k, g, h).unwrap();
    e.ejecutar(1_000_000, false, 0).unwrap();
    e
}

const SUMA: &str = "
.global 24
.dato 0 1 2 3 4 5 6 7 8
.dato 8 10 20 30 40 50 60 70 80
.lanzar 2 4
        gid r0
        ld  r1, [r0+0]
        ld  r2, [r0+8]
        add r3, r1, r2
        st  [r0+16], r3
        exit
";

#[test]
fn la_suma_de_vectores_da_el_resultado() {
    let e = correr(SUMA, "tamano_warp = 4\n");
    assert_eq!(&e.global()[16..24], &[11, 22, 33, 44, 55, 66, 77, 88]);
    let m = e.metricas();
    assert_eq!((m.hilos, m.grupos), (8, 2));
    assert_eq!(m.emisiones, 12, "2 warps x 6 instrucciones");
    assert_eq!(m.emisiones_divergentes, 0);
    assert_eq!(m.utilizacion_carriles(4), 1.0);
    assert_eq!(m.accesos_globales, 6);
}

#[test]
fn el_resultado_no_depende_de_la_forma_de_la_unidad() {
    let a = correr(SUMA, "tamano_warp = 4\nunidades = 1\n");
    let b = correr(SUMA, "tamano_warp = 2\nunidades = 4\nlatencia_global = 3\n");
    let c = correr(SUMA, "tamano_warp = 8\nunidades = 2\n");
    assert_eq!(a.global(), b.global());
    assert_eq!(a.global(), c.global());
    assert_ne!(a.metricas().ciclos, b.metricas().ciclos);
}

#[test]
fn mas_unidades_reparten_los_grupos_y_acortan_el_tiempo() {
    let una = correr(SUMA, "tamano_warp = 4\nunidades = 1\n")
        .metricas()
        .ciclos;
    let dos = correr(SUMA, "tamano_warp = 4\nunidades = 2\n")
        .metricas()
        .ciclos;
    assert!(dos < una, "{dos} < {una}");
}

#[test]
fn la_divergencia_se_ve_en_la_mascara_y_en_las_metricas() {
    let src = "
.global 8
.lanzar 1 4
        tid r0
        and r1, r0, 1
        bz  r1, par
        li  r2, 100         ; solo los impares
        bra fin
par:    li  r2, 200         ; solo los pares
fin:    st  [r0+0], r2
        exit
";
    let k = ensamblar_kernel(src).unwrap();
    let mut e = EjecucionHilos::nueva(cfg("tamano_warp = 4\nunidades = 1\n"), k, 1, 4).unwrap();
    let em = e.ejecutar(1000, true, 100).unwrap();
    assert_eq!(&e.global()[..4], &[200, 100, 200, 100]);
    let por_texto = |t: &str| em.iter().find(|x| x.texto.starts_with(t)).unwrap();
    assert_eq!(por_texto("li  r2, 100").mascara, 0b1010);
    assert_eq!(por_texto("li  r2, 200").mascara, 0b0101);
    assert!(por_texto("li  r2, 100").divergente);
    assert_eq!(por_texto("st").mascara, 0b1111, "se reagrupan");
    assert!(!por_texto("st").divergente);
    let m = e.metricas();
    assert!(m.emisiones_divergentes >= 3);
    assert!(m.utilizacion_carriles(4) < 1.0);
}

#[test]
fn sin_divergencia_la_utilizacion_es_total_con_hilos_de_menos_no() {
    // 6 hilos con warp de 4: el segundo warp lleva solo 2 hilos.
    let src = ".global 8\n.lanzar 1 6\n gid r0\n st [r0+0], r0\n exit\n";
    let k = ensamblar_kernel(src).unwrap();
    let mut e = EjecucionHilos::nueva(cfg("tamano_warp = 4\n"), k, 1, 6).unwrap();
    e.ejecutar(100, false, 0).unwrap();
    assert_eq!(&e.global()[..6], &[0, 1, 2, 3, 4, 5]);
    assert!(e.metricas().utilizacion_carriles(4) < 1.0);
}

const REDUCCION: &str = "
.global 16
.dato 0 3 1 4 1 5 9 2 6
.lanzar 1 8
        tid  r0
        ld   r1, [r0+0]
        sts  [r0+0], r1       ; a la memoria compartida
        bar
        li   r4, 4            ; paso
bucle:  slt  r5, r0, r4       ; solo suman los hilos con tid < paso
        bz   r5, siguiente
        add  r6, r0, r4
        lds  r7, [r0+0]
        lds  r8, [r6+0]
        add  r7, r7, r8
        sts  [r0+0], r7
siguiente:
        bar
        shr  r4, r4, 1
        bnz  r4, bucle
        seq  r9, r0, 0
        bz   r9, fin
        lds  r1, [r0+0]
        st   [r0+8], r1       ; el hilo 0 publica el total
fin:    exit
";

#[test]
fn la_memoria_compartida_y_la_barrera_permiten_reducir() {
    for w in [2, 4, 8] {
        let e = correr(REDUCCION, &format!("tamano_warp = {w}\nunidades = 1\n"));
        assert_eq!(e.global()[8], 3 + 1 + 4 + 1 + 5 + 9 + 2 + 6, "warp {w}");
        assert!(e.metricas().barreras >= 4);
        assert!(e.metricas().accesos_compartidos > 0);
    }
}

#[test]
fn la_memoria_compartida_es_propia_de_cada_grupo() {
    let src = "
.global 4
.lanzar 2 1
        bid r0
        add r1, r0, 10
        sts [r2+0], r1        ; r2 vale 0: compartida[0] = 10 + grupo
        lds r3, [r2+0]
        st  [r0+0], r3
        exit
";
    let e = correr(src, "unidades = 1\n");
    assert_eq!(&e.global()[..2], &[10, 11]);
}

#[test]
fn mas_warps_esconden_la_latencia_de_memoria() {
    // Cada warp hace un ld y poco mas; con mas warps por grupo la unidad no espera tanto.
    let src = |h: usize| {
        format!(".global 64\n.lanzar 1 {h}\n gid r0\n ld r1, [r0+0]\n add r1, r1, 1\n st [r0+0], r1\n exit\n")
    };
    let ciclos_por_hilo = |h: usize| {
        let k = ensamblar_kernel(&src(h)).unwrap();
        let mut e = EjecucionHilos::nueva(
            cfg("tamano_warp = 4\nunidades = 1\nlatencia_global = 20\n"),
            k,
            1,
            h,
        )
        .unwrap();
        e.ejecutar(10_000, false, 0).unwrap();
        e.metricas().ciclos as f64 / h as f64
    };
    assert!(ciclos_por_hilo(32) < ciclos_por_hilo(4) / 2.0);
}

#[test]
fn los_ciclos_cuentan_la_latencia_y_el_ocio() {
    let e = correr(
        ".global 4\n.lanzar 1 1\n li r0, 0\n ld r1, [r0+0]\n exit\n",
        "tamano_warp = 1\nunidades = 1\nlatencia_global = 10\n",
    );
    let m = e.metricas();
    assert_eq!(m.emisiones, 3);
    assert_eq!(
        m.ciclos_ociosos, 9,
        "espera 10 ciclos por el ld, uno ya cuenta como emision"
    );
    assert_eq!(m.ciclos, 12);
}

#[test]
fn errores_de_ejecucion_senalan_el_hilo() {
    let src = ".global 4\n.lanzar 1 2\n tid r0\n ld r1, [r0+3]\n exit\n";
    let k = ensamblar_kernel(src).unwrap();
    let mut e = EjecucionHilos::nueva(cfg("tamano_warp = 2\n"), k, 1, 2).unwrap();
    let m = e.ejecutar(100, false, 0).unwrap_err().to_string();
    assert!(
        m.contains("hilo 1") && m.contains("linea 4") && m.contains("direccion 4"),
        "{m}"
    );
    let k = ensamblar_kernel(".lanzar 1 1\n li r0, 5\n div r1, r0, 0\n exit\n").unwrap();
    let mut e = EjecucionHilos::nueva(cfg(""), k, 1, 1).unwrap();
    assert!(e
        .ejecutar(100, false, 0)
        .unwrap_err()
        .to_string()
        .contains("division por cero"));
}

#[test]
fn un_kernel_que_no_termina_se_corta() {
    let k = ensamblar_kernel(".lanzar 1 1\nbucle: bra bucle\n").unwrap();
    let mut e = EjecucionHilos::nueva(cfg(""), k, 1, 1).unwrap();
    assert_eq!(
        e.ejecutar(500, false, 0).unwrap_err(),
        ErrorGpu::LimiteDeEmisiones { limite: 500 }
    );
}

#[test]
fn el_ensamblador_senala_la_linea() {
    for (t, frag) in [
        ("li r0\n", "linea 1: `li` lleva 2 operandos"),
        ("li r99, 1\n", "no es un registro"),
        ("saltar r0\n", "no existe"),
        ("bra nada\n", "etiqueta `nada` no existe"),
        ("a: li r0,1\na: exit\n", "ya existe"),
        (".lanzar 0 1\nexit\n", "no tiene los argumentos"),
        ("ld r0, r1\n", "direccion"),
        ("; vacio\n", "no tiene instrucciones"),
    ] {
        let e = ensamblar_kernel(t).unwrap_err().to_string();
        assert!(e.contains(frag), "{t:?} -> {e}");
    }
}

#[test]
fn la_configuracion_de_hilos_se_valida() {
    for (t, frag) in [
        ("tamano_warp = 0\n", "tamano_warp"),
        ("unidades = 0\n", "unidades"),
        ("memoria_global = 0\n", "memoria_global"),
        ("latencia_global = 0\n", "latencia_global"),
        ("warp = 3\n", "warp"),
    ] {
        let e = cargar_config(&format!("[gpu]\n[gpu.hilos]\n{t}"))
            .unwrap_err()
            .to_string();
        assert!(e.contains(frag), "{t:?} -> {e}");
    }
    assert_eq!(
        cargar_config("[gpu]\n").unwrap().unwrap().hilos,
        ConfigHilos::default()
    );
}
