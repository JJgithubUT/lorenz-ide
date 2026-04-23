//! `lorenz kernel`, `lorenz depurar-kernel` y `lorenz comparar-gpu`: los hilos de la unidad
//! grafica (RF-GPU-013 a RF-GPU-016, RF-GPU-018, RF-GPU-019).

use std::io::{BufRead, Write};

use lorenz_emu::Emulador;
use lorenz_gpu::{
    cargar_config_archivo, ensamblar_kernel, ConfigGpu, EjecucionHilos, Emision, MetricasHilos,
};

use crate::ciclos::cargar_con_lineas;
use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_OK};

/// Instrucciones de warp que se permiten por omision antes de cortar un kernel.
pub const LIMITE_EMISIONES: u64 = 50_000_000;
/// Lineas de traza que se imprimen como mucho.
const TOPE_TRAZA: usize = 20_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcionesKernel {
    pub limite: u64,
    pub grupos: Option<usize>,
    pub hilos: Option<usize>,
    pub traza: bool,
    /// Tramos `(desde, cantidad)` de la memoria global a imprimir.
    pub volcados: Vec<(u64, u64)>,
}

impl Default for OpcionesKernel {
    fn default() -> Self {
        Self {
            limite: LIMITE_EMISIONES,
            grupos: None,
            hilos: None,
            traza: false,
            volcados: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcionesComparar {
    /// Instrucciones de la CPU como maximo.
    pub limite: u64,
    pub direccion: u64,
    /// `(desde en la memoria de la CPU, desde en la global de la GPU, cantidad)`.
    pub resultado: Option<(u64, u64, u64)>,
}

impl Default for OpcionesComparar {
    fn default() -> Self {
        Self {
            limite: crate::args::LIMITE_POR_OMISION,
            direccion: 0,
            resultado: None,
        }
    }
}

fn escribir(w: &mut dyn Write, t: &str) {
    let _ = w.write_all(t.as_bytes());
}

fn uso(errores: &mut dyn Write, m: String) -> i32 {
    escribir(errores, &format!("{m}\n"));
    SALIDA_ERROR_USO
}

fn mascara_texto(m: u64, ancho: usize) -> String {
    (0..ancho)
        .map(|i| if m >> i & 1 == 1 { '#' } else { '.' })
        .collect()
}

fn linea_de_emision(e: &Emision, warp: usize) -> String {
    format!(
        "ciclo {:>5}  u{}  g{}.w{}  pc={:<3} [{}]{} {}",
        e.ciclo,
        e.unidad,
        e.grupo,
        e.warp,
        e.pc,
        mascara_texto(e.mascara, warp),
        if e.divergente { " divergente" } else { "" },
        e.texto
    )
}

pub fn texto_de_metricas(m: &MetricasHilos, config: &ConfigGpu) -> String {
    let w = config.hilos.tamano_warp;
    let mut s = String::new();
    s.push_str(&format!(
        "lanzamiento: {} hilos en {} grupos (warp de {w}, {} unidades)\n",
        m.hilos, m.grupos, config.hilos.unidades
    ));
    s.push_str(&format!("ciclos de la GPU: {}\n", m.ciclos));
    s.push_str(&format!(
        "instrucciones de warp: {} ({} con el warp dividido)\n",
        m.emisiones, m.emisiones_divergentes
    ));
    s.push_str(&format!(
        "uso de carriles: {:.1}%\n",
        m.utilizacion_carriles(w) * 100.0
    ));
    s.push_str(&format!("ciclos ociosos: {}\n", m.ciclos_ociosos));
    s.push_str(&format!(
        "accesos: {} a memoria global, {} a compartida; {} barreras\n",
        m.accesos_globales, m.accesos_compartidos, m.barreras
    ));
    s
}

struct Preparado {
    config: ConfigGpu,
    ejecucion: EjecucionHilos,
}

fn preparar(
    ruta_maquina: &str,
    ruta_kernel: &str,
    grupos: Option<usize>,
    hilos: Option<usize>,
) -> Result<Preparado, String> {
    let config = match cargar_config_archivo(ruta_maquina) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return Err(format!(
                "{ruta_maquina}: la descripcion no declara una unidad grafica (falta [gpu])"
            ))
        }
        Err(e) => return Err(format!("{ruta_maquina}: {e}")),
    };
    let texto = std::fs::read_to_string(ruta_kernel).map_err(|e| format!("{ruta_kernel}: {e}"))?;
    let kernel = ensamblar_kernel(&texto).map_err(|e| format!("{ruta_kernel}: {e}"))?;
    let (g, h) = match (grupos, hilos, kernel.lanzar) {
        (Some(g), Some(h), _) => (g, h),
        (g, h, Some((kg, kh))) => (g.unwrap_or(kg), h.unwrap_or(kh)),
        _ => {
            return Err(format!(
                "{ruta_kernel}: no se sabe cuantos hilos lanzar: pon `.lanzar GRUPOS HILOS` en el kernel o usa --grupos y --hilos"
            ))
        }
    };
    let ejecucion = EjecucionHilos::nueva(config.hilos, kernel, g, h)
        .map_err(|e| format!("{ruta_kernel}: {e}"))?;
    Ok(Preparado { config, ejecucion })
}

pub fn kernel_cli(
    ruta_maquina: &str,
    ruta_kernel: &str,
    opciones: &OpcionesKernel,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let mut p = match preparar(ruta_maquina, ruta_kernel, opciones.grupos, opciones.hilos) {
        Ok(p) => p,
        Err(e) => return uso(errores, e),
    };
    let w = p.config.hilos.tamano_warp;
    let r = p
        .ejecucion
        .ejecutar(opciones.limite, opciones.traza, TOPE_TRAZA);
    let traza = match r {
        Ok(t) => t,
        Err(lorenz_gpu::ErrorGpu::LimiteDeEmisiones { limite }) => {
            escribir(
                errores,
                &format!("el kernel no termino tras {limite} instrucciones de warp\n"),
            );
            return SALIDA_LIMITE;
        }
        Err(e) => {
            escribir(errores, &format!("fallo durante la ejecucion: {e}\n"));
            return SALIDA_ERROR_EJECUCION;
        }
    };
    if opciones.traza {
        for e in &traza {
            escribir(salida, &format!("{}\n", linea_de_emision(e, w)));
        }
        if p.ejecucion.metricas().emisiones as usize > TOPE_TRAZA {
            escribir(
                salida,
                &format!("... la traza se corta tras {TOPE_TRAZA} lineas\n"),
            );
        }
    }
    escribir(
        salida,
        &texto_de_metricas(&p.ejecucion.metricas(), &p.config),
    );
    for (desde, cantidad) in &opciones.volcados {
        let g = p.ejecucion.global();
        let fin = (*desde + *cantidad).min(g.len() as u64);
        if *desde >= fin {
            return uso(
                errores,
                format!(
                    "--volcar {desde}:{cantidad}: fuera de la memoria global de {} palabras",
                    g.len()
                ),
            );
        }
        escribir(salida, &format!("global[{desde}..{fin}]:"));
        for v in &g[*desde as usize..fin as usize] {
            escribir(salida, &format!(" {v}"));
        }
        escribir(salida, "\n");
    }
    SALIDA_OK
}

pub fn comparar_cli(
    ruta_maquina: &str,
    ruta_programa: &str,
    ruta_kernel: &str,
    opciones: &OpcionesComparar,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let maquina = match lorenz_arch::cargar_archivo(ruta_maquina) {
        Ok(m) => m,
        Err(e) => return uso(errores, format!("{ruta_maquina}: {e}")),
    };
    let mut p = match preparar(ruta_maquina, ruta_kernel, None, None) {
        Ok(p) => p,
        Err(e) => return uso(errores, e),
    };
    let segmentos = match cargar_con_lineas(&maquina, ruta_programa, opciones.direccion) {
        Ok((s, _, _)) => s,
        Err(e) => return uso(errores, e),
    };
    let mut emu = match Emulador::nuevo(maquina) {
        Ok(e) => e,
        Err(e) => return uso(errores, format!("{ruta_maquina}: {e}")),
    };
    for (d, w) in &segmentos {
        if let Err(e) = emu.cargar_programa(*d, w) {
            return uso(errores, format!("{ruta_programa}: {e}"));
        }
    }
    let cpu = match emu.ejecutar(opciones.limite) {
        Ok(r) => r,
        Err(lorenz_emu::ErrorEjecucion::LimiteDeInstrucciones { limite }) => {
            escribir(
                errores,
                &format!("la CPU no termino tras {limite} instrucciones\n"),
            );
            return SALIDA_LIMITE;
        }
        Err(e) => {
            escribir(errores, &format!("fallo de la CPU: {e}\n"));
            return SALIDA_ERROR_EJECUCION;
        }
    };
    if let Err(e) = p.ejecucion.ejecutar(LIMITE_EMISIONES, false, 0) {
        escribir(errores, &format!("fallo de la GPU: {e}\n"));
        return SALIDA_ERROR_EJECUCION;
    }
    let m = p.ejecucion.metricas();
    escribir(
        salida,
        &format!(
            "CPU: {} ciclos, {} instrucciones\n",
            cpu.ciclos, cpu.instrucciones
        ),
    );
    escribir(
        salida,
        &format!(
            "GPU: {} ciclos, {} hilos en {} grupos (uso de carriles {:.1}%)\n",
            m.ciclos,
            m.hilos,
            m.grupos,
            m.utilizacion_carriles(p.config.hilos.tamano_warp) * 100.0
        ),
    );
    if m.ciclos > 0 && cpu.ciclos > 0 {
        let r = cpu.ciclos as f64 / m.ciclos as f64;
        escribir(
            salida,
            &if r >= 1.0 {
                format!("la GPU tarda {:.2} veces menos\n", r)
            } else {
                format!("la CPU tarda {:.2} veces menos\n", 1.0 / r)
            },
        );
    }
    let mut codigo = SALIDA_OK;
    if let Some((cpu_desde, gpu_desde, cantidad)) = opciones.resultado {
        let mascara = emu.memoria().mascara_palabra();
        let g = p.ejecucion.global();
        let mut distintas = Vec::new();
        for k in 0..cantidad {
            let a = emu.memoria().leer_privilegiado(cpu_desde + k).unwrap_or(0);
            let b = g.get((gpu_desde + k) as usize).copied().unwrap_or(0) as u64 & mascara;
            if a != b {
                distintas.push((k, a, b));
            }
        }
        if distintas.is_empty() {
            escribir(
                salida,
                &format!("resultados: iguales en {cantidad} palabras\n"),
            );
        } else {
            escribir(
                salida,
                &format!(
                    "resultados: {} palabras distintas de {cantidad}\n",
                    distintas.len()
                ),
            );
            for (k, a, b) in distintas.iter().take(8) {
                escribir(
                    salida,
                    &format!("  posicion {k}: la CPU da {a} y la GPU {b}\n"),
                );
            }
            codigo = SALIDA_ERROR_EJECUCION;
        }
    }
    codigo
}

fn lista_de_ayuda() -> &'static str {
    "  paso [N]            ejecuta N instrucciones de warp (1 por omision)\n\
     \x20 continuar           corre hasta el final\n\
     \x20 hilo G.W.L          registros, contador y estado de un hilo (grupo.warp.carril)\n\
     \x20 grupos              grupos que estan corriendo\n\
     \x20 memoria D [N]       memoria global\n\
     \x20 compartida G [D N]  memoria compartida de un grupo en curso\n\
     \x20 info                metricas hasta ahora\n\
     \x20 fin                 termina\n"
}

pub fn depurar_kernel_cli(
    ruta_maquina: &str,
    ruta_kernel: &str,
    guion: Option<&str>,
    entrada: &mut dyn BufRead,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let mut p = match preparar(ruta_maquina, ruta_kernel, None, None) {
        Ok(p) => p,
        Err(e) => return uso(errores, e),
    };
    let w = p.config.hilos.tamano_warp;
    let (mut lector, es_guion): (Box<dyn BufRead>, bool) = match guion {
        Some(r) => match std::fs::File::open(r) {
            Ok(f) => (Box::new(std::io::BufReader::new(f)), true),
            Err(e) => return uso(errores, format!("{r}: no se pudo leer: {e}")),
        },
        None => (Box::new(entrada), false),
    };
    escribir(
        salida,
        &format!("{ruta_kernel} cargado; escribe `ayuda` para ver las ordenes\n"),
    );
    loop {
        escribir(salida, "(gpu) ");
        let _ = salida.flush();
        let mut linea = String::new();
        if matches!(lector.read_line(&mut linea), Ok(0) | Err(_)) {
            escribir(salida, "\n");
            break;
        }
        let linea = linea.trim();
        if es_guion {
            if linea.is_empty() || linea.starts_with('#') {
                escribir(salida, "\n");
                continue;
            }
            escribir(salida, &format!("{linea}\n"));
        }
        let mut it = linea.split_whitespace();
        let orden = it.next().unwrap_or("");
        let args: Vec<&str> = it.collect();
        let num = |t: &str| t.parse::<usize>().ok();
        match orden {
            "" => {}
            "fin" | "salir" => break,
            "ayuda" => escribir(salida, lista_de_ayuda()),
            "paso" | "continuar" => {
                let n = if orden == "continuar" {
                    usize::MAX
                } else {
                    args.first().and_then(|a| num(a)).unwrap_or(1)
                };
                for _ in 0..n {
                    match p.ejecucion.paso() {
                        Ok(Some(e)) => {
                            if orden == "paso" {
                                escribir(salida, &format!("{}\n", linea_de_emision(&e, w)));
                            }
                        }
                        Ok(None) => {
                            escribir(salida, "el kernel termino\n");
                            break;
                        }
                        Err(e) => {
                            escribir(salida, &format!("fallo: {e}\n"));
                            break;
                        }
                    }
                    if p.ejecucion.metricas().emisiones >= LIMITE_EMISIONES {
                        escribir(salida, "se alcanzo el limite de instrucciones de warp\n");
                        break;
                    }
                }
            }
            "hilo" => {
                let partes: Vec<Option<usize>> = args
                    .first()
                    .map_or(Vec::new(), |a| a.split('.').map(num).collect());
                if let [Some(g), Some(wp), Some(l)] = partes[..] {
                    match p.ejecucion.registros_de(g, wp, l) {
                        Some((regs, pc, vivo)) => {
                            escribir(
                                salida,
                                &format!(
                                    "hilo {g}.{wp}.{l}: pc={pc}, {}\n",
                                    if vivo { "vivo" } else { "terminado" }
                                ),
                            );
                            for (i, r) in regs.iter().enumerate() {
                                escribir(salida, &format!("  r{i:<2} = {r}\n"));
                            }
                        }
                        None => escribir(
                            salida,
                            "ese hilo no esta en curso ahora (grupo terminado o sin empezar)\n",
                        ),
                    }
                } else {
                    escribir(salida, "uso: hilo GRUPO.WARP.CARRIL\n");
                }
            }
            "grupos" => {
                let c = p.ejecucion.en_curso();
                if c.is_empty() {
                    escribir(salida, "ningun grupo en curso\n");
                }
                for (u, g, ws) in c {
                    escribir(salida, &format!("unidad {u}: grupo {g} ({ws} warps)\n"));
                }
            }
            "memoria" => {
                let d = args.first().and_then(|a| num(a)).unwrap_or(0);
                let n = args.get(1).and_then(|a| num(a)).unwrap_or(8);
                let g = p.ejecucion.global();
                let fin = (d + n).min(g.len());
                if d >= fin {
                    escribir(
                        salida,
                        &format!("fuera de la memoria global de {} palabras\n", g.len()),
                    );
                } else {
                    escribir(salida, &format!("global[{d}..{fin}]: {:?}\n", &g[d..fin]));
                }
            }
            "compartida" => {
                let g = args.first().and_then(|a| num(a));
                let d = args.get(1).and_then(|a| num(a)).unwrap_or(0);
                let n = args.get(2).and_then(|a| num(a)).unwrap_or(8);
                match g.and_then(|g| p.ejecucion.compartida_de(g)) {
                    Some(c) => {
                        let fin = (d + n).min(c.len());
                        escribir(
                            salida,
                            &format!("compartida[{d}..{fin}]: {:?}\n", &c[d.min(fin)..fin]),
                        );
                    }
                    None => escribir(
                        salida,
                        "uso: compartida GRUPO [DESDE N]; el grupo debe estar en curso\n",
                    ),
                }
            }
            "info" => escribir(
                salida,
                &texto_de_metricas(&p.ejecucion.metricas(), &p.config),
            ),
            otra => escribir(
                salida,
                &format!("la orden `{otra}` no existe; escribe `ayuda`\n"),
            ),
        }
    }
    SALIDA_OK
}
