//! Ejecucion de las ordenes de la linea de comandos.

use std::io::{BufRead, Write};

use lorenz_arch::modelo::TipoDispositivo;
use lorenz_emu::{Emulador, ErrorEjecucion};

use crate::args::{interpretar, Disparo, Orden, Volcado};
use crate::guion::{leer_guion, Accion, Evento};
use crate::programa::escribir_palabras;

/// El programa termino bien: la maquina se detuvo.
pub const SALIDA_OK: i32 = 0;
/// La ejecucion fallo: acceso invalido, instruccion desconocida, division entre cero...
pub const SALIDA_ERROR_EJECUCION: i32 = 1;
/// Argumentos mal escritos, archivo ilegible o arquitectura invalida.
pub const SALIDA_ERROR_USO: i32 = 2;
/// Se alcanzo el limite de instrucciones sin que la maquina se detuviera.
pub const SALIDA_LIMITE: i32 = 3;
/// `comparar` encontro una diferencia entre las dos maquinas.
pub const SALIDA_DIFIEREN: i32 = 4;
/// Alguna restriccion de diseno no se cumple (`restricciones`, `variantes`).
pub const SALIDA_NO_CUMPLE: i32 = 5;

const AYUDA: &str = "\
Lorenz: emulador de maquinas definidas por el usuario

Uso:
  lorenz validar <maquina.toml>
  lorenz ejecutar <maquina.toml> <programa.hex | fuente.s> [--limite N] [--direccion A] [--traza | --traza-maquina]
                  [--pantalla] [--vigilar] [--cobertura] [--teclas TEXTO] [--serie TEXTO]
                  [--volcar DESDE:CANTIDAD[:ARCHIVO]] [--interrumpir LINEA@INSTRUCCION]
                  [--guion ARCHIVO]
  lorenz compilar <maquina.toml> <fuente.sgt> [-O0|-O1|-O2|-O3] [-o PROGRAMA.hex] [--asm ARCHIVO] [--lineas ARCHIVO] [--costes ARCHIVO]
  lorenz cotejar <fuente.sgt> <maquina.toml> <maquina.toml>... [-O0|-O1|-O2|-O3] [--limite N]
  lorenz ciclos <maquina.toml> <programa.hex | fuente.s | fuente.sgt> [--limite N] [--direccion A]
               [--vista CICLOS] [--diagrama INSTRUCCIONES] [--perfil] [--cache]
               [--calor] [--calor-html ARCHIVO] [--historial ARCHIVO [--etiqueta TEXTO]] [--vigilar]
  lorenz comparar-ciclos <maquina-a.toml> <maquina-b.toml> <programa> [--limite N] [--direccion A]
  lorenz historial-ciclos <historial.toml>
  lorenz sugerir <maquina.toml> <programa>... [--limite N] [--direccion A] [--minima N] [--maxima N]
               [--cuantas N]
  lorenz nucleos <maquina.toml> <programa>... [--limite N] [--direccion A]
  lorenz kernel <maquina.toml> <kernel.kgpu> [--grupos N] [--hilos N] [--traza] [--volcar DESDE:CANTIDAD] [--limite N]
  lorenz depurar-kernel <maquina.toml> <kernel.kgpu> [--guion ARCHIVO]
  lorenz comparar-gpu <maquina.toml> <programa> <kernel.kgpu> [--resultado DIR_CPU:DIR_GPU:CANTIDAD] [--limite N]
  lorenz gpu <maquina.toml> <programa | guion.gpu> [--limite N] [--direccion A] [--imagen ARCHIVO.png|.ppm]
             [--ampliar N] [--pixel X,Y]... [--texto]
  lorenz nuevo <plantilla | carpeta-plantilla> <destino> [--nombre NOMBRE]
  lorenz plantillas
  lorenz ejercicio listar <carpeta-de-ejercicios>
  lorenz ejercicio ver <ejercicio> [--pista N]
  lorenz ejercicio verificar <ejercicio> [--solucion ARCHIVO]
  lorenz desafio ver <desafio>
  lorenz desafio evaluar <desafio> <solucion>
  lorenz desafio comparar <desafio> <solucion>...
  lorenz recientes [listar | abrir N | olvidar N|todos] [--en ARCHIVO]
  lorenz proyecto <carpeta | proyecto.toml> [--arquitectura NOMBRE]
  lorenz tarea <carpeta | proyecto.toml> [NOMBRE] [--arquitectura NOMBRE]
  lorenz paquete crear <carpeta> [-o ARCHIVO.lpkg]
  lorenz paquete verificar|info <archivo.lpkg>
  lorenz paquete instalar <archivo.lpkg> [--en DIR] [--repositorio DIR]...
  lorenz paquete listar [--en DIR]
  lorenz restricciones <maquina.toml> <programa> [--limite N] [--direccion A]
  lorenz variantes <maquina.toml> <programa> [--limite N] [--direccion A] [--bus] [--cuantas N]
  lorenz convencion <maquina.toml> [-o CONVENCION.md]
  lorenz ensamblar <maquina.toml> <fuente.s> [-o PROGRAMA.hex] [--mapa ARCHIVO] [--listado ARCHIVO]
                   [--simbolos ARCHIVO] [--objeto ARCHIVO]
  lorenz enlazar <maquina.toml> <objeto.toml | fuente.s>... [-o PROGRAMA.hex] [--mapa ARCHIVO]
                 [--simbolos ARCHIVO] [--objeto ARCHIVO]
  lorenz exportar <verilog|pdf|logisim> <maquina.toml> [programa] -o SALIDA
  lorenz importar <tabla> [-o maquina.toml]
  lorenz documentar <maquina.toml> [-o DOCUMENTACION.md] [--vigilar]
  lorenz desensamblar <maquina.toml> <programa.hex> [--direccion A] [--simbolos ARCHIVO]
  lorenz depurar <maquina.toml> <programa.hex | fuente.s> [--guion ARCHIVO]
  lorenz probar <pruebas.toml> [--nombre TEXTO] [--etiqueta ETIQUETA] [--hilos N] [--json]
                [--salida RESULTADOS.json] [--depurar PRUEBA] [--cobertura]
                [--comparar ANTERIOR.json] [--historial] [--umbral PORCENTAJE]
  lorenz conformidad <maquina.toml> [--instruccion MNEMONICO] [--casos N] [--repeticiones N]
                     [--json] [--salida INFORME.json]
  lorenz aleatorio <maquina.toml> [--semilla N] [--programas N] [--longitud N] [--limite N]
                   [--sin-reducir] [--json] [--salida INFORME.json]
  lorenz compatibilidad <anterior.toml> <nueva.toml> <programa.hex | fuente.s> [--direccion A] [--json]
  lorenz comparar <maquina-a.toml> <maquina-b.toml> <programa.hex> [--limite N] [--direccion A]
  lorenz ayuda
  lorenz version

Codigos de salida:
  0  la maquina se detuvo
  1  fallo durante la ejecucion
  2  argumentos, archivos o arquitectura invalidos
  3  se alcanzo el limite de instrucciones
  4  comparar: las dos maquinas difieren
  6  probar --comparar: ninguna prueba se rompio pero alguna gasta mas ciclos que antes
  5  probar: alguna prueba no se pudo preparar (el programa no ensambla, falta un archivo...)
     (probar usa 1 si alguna prueba falla y 0 si todas pasan; conformidad usa 1 si algun caso falla;
      aleatorio usa 1 si el emulador y la referencia divergen;
      compatibilidad usa 1 si algun binario anterior deja de valer)
";

fn escribir(salida: &mut dyn Write, texto: &str) {
    // Si la salida esta cerrada (por ejemplo una tuberia), no hay a quien avisar.
    let _ = salida.write_all(texto.as_bytes());
}

/// Ejecuta la linea de comandos y devuelve el codigo de salida.
pub fn ejecutar_cli(args: &[String], salida: &mut dyn Write, errores: &mut dyn Write) -> i32 {
    ejecutar_cli_con_entrada(args, &mut std::io::empty(), salida, errores)
}

/// Igual que [`ejecutar_cli`], pero las ordenes que leen del usuario (`depurar`) lo hacen de
/// `entrada`.
pub fn ejecutar_cli_con_entrada(
    args: &[String],
    entrada: &mut dyn BufRead,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let orden = match interpretar(args) {
        Ok(o) => o,
        Err(e) => {
            escribir(errores, &format!("error: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    match orden {
        Orden::Ayuda => {
            escribir(salida, AYUDA);
            SALIDA_OK
        }
        Orden::Version => {
            escribir(salida, &format!("lorenz {}\n", env!("CARGO_PKG_VERSION")));
            SALIDA_OK
        }
        Orden::Validar { maquina } => validar(&maquina, salida, errores),
        Orden::Probar {
            pruebas,
            nombre,
            etiqueta,
            hilos,
            json,
            salida: archivo,
            depurar,
            cobertura,
            comparar,
            historial,
            umbral,
        } => crate::probar::probar_cli(
            &pruebas,
            &crate::probar::OpcionesProbar {
                nombre,
                etiqueta,
                hilos,
                json,
                salida: archivo,
                depurar,
                cobertura,
                comparar,
                historial,
                umbral,
            },
            entrada,
            salida,
            errores,
        ),
        Orden::Conformidad {
            maquina,
            instruccion,
            casos,
            repeticiones,
            json,
            salida: archivo,
        } => crate::conformidad::conformidad_cli(
            &maquina,
            &crate::conformidad::OpcionesConformidad {
                instruccion,
                casos,
                repeticiones,
                json,
                salida: archivo,
            },
            salida,
            errores,
        ),
        Orden::Aleatorio {
            maquina,
            semilla,
            programas,
            longitud,
            limite,
            sin_reducir,
            json,
            salida: archivo,
        } => crate::aleatorio::aleatorio_cli(
            &maquina,
            &crate::aleatorio::OpcionesAleatorio {
                semilla,
                programas,
                longitud,
                limite,
                sin_reducir,
                json,
                salida: archivo,
            },
            salida,
            errores,
        ),
        Orden::Compatibilidad {
            anterior,
            nueva,
            programa,
            direccion,
            json,
        } => crate::compatibilidad::compatibilidad_cli(
            &anterior, &nueva, &programa, direccion, json, salida, errores,
        ),
        Orden::Depurar {
            maquina,
            programa,
            guion,
        } => crate::depurar::depurar_cli(
            &maquina,
            &programa,
            guion.as_deref(),
            entrada,
            salida,
            errores,
        ),
        Orden::Compilar {
            maquina,
            fuente,
            opciones,
        } => crate::compilar::compilar_cli(&maquina, &fuente, &opciones, salida, errores),
        Orden::Ensamblar {
            maquina,
            fuente,
            opciones,
        } => crate::ensamblar::ensamblar_cli(&maquina, &fuente, &opciones, salida, errores),
        Orden::Cotejar {
            fuente,
            maquinas,
            opciones,
        } => crate::cotejar::cotejar_cli(&fuente, &maquinas, &opciones, salida, errores),
        Orden::Ciclos {
            maquina,
            programa,
            opciones,
        } => crate::ciclos::ciclos_cli(&maquina, &programa, &opciones, salida, errores),
        Orden::CompararCiclos {
            maquina_a,
            maquina_b,
            programa,
            opciones,
        } => crate::ciclos::comparar_ciclos_cli(
            &maquina_a, &maquina_b, &programa, &opciones, salida, errores,
        ),
        Orden::Sugerir {
            maquina,
            programas,
            opciones,
        } => crate::sugerir::sugerir_cli(&maquina, &programas, &opciones, salida, errores),
        Orden::Nucleos {
            maquina,
            programas,
            opciones,
        } => crate::nucleos::nucleos_cli(&maquina, &programas, &opciones, salida, errores),
        Orden::Gpu {
            maquina,
            programa,
            opciones,
        } => crate::gpu::gpu_cli(&maquina, &programa, &opciones, salida, errores),
        Orden::Ejercicio { accion } => crate::ejercicios::ejercicio_cli(&accion, salida, errores),
        Orden::Desafio { accion } => crate::desafios::desafio_cli(&accion, salida, errores),
        Orden::Recientes { accion, en } => {
            crate::recientes::recientes_cli(&accion, en.as_deref(), salida, errores)
        }
        Orden::Plantillas => crate::nuevo::plantillas_cli(salida),
        Orden::Proyecto { ruta, arquitectura } => {
            crate::proyecto::proyecto_cli(&ruta, arquitectura.as_deref(), salida, errores)
        }
        Orden::Tarea {
            ruta,
            nombre,
            arquitectura,
        } => crate::proyecto::tarea_cli(
            &ruta,
            nombre.as_deref(),
            arquitectura.as_deref(),
            salida,
            errores,
        ),
        Orden::Paquete { accion, opciones } => {
            crate::paquete::paquete_cli(&accion, &opciones, salida, errores)
        }
        Orden::Kernel {
            maquina,
            kernel,
            opciones,
        } => crate::kernel::kernel_cli(&maquina, &kernel, &opciones, salida, errores),
        Orden::DepurarKernel {
            maquina,
            kernel,
            guion,
        } => crate::kernel::depurar_kernel_cli(
            &maquina,
            &kernel,
            guion.as_deref(),
            entrada,
            salida,
            errores,
        ),
        Orden::CompararGpu {
            maquina,
            programa,
            kernel,
            opciones,
        } => crate::kernel::comparar_cli(&maquina, &programa, &kernel, &opciones, salida, errores),
        Orden::Restricciones {
            maquina,
            programa,
            opciones,
        } => crate::variantes::restricciones_cli(&maquina, &programa, &opciones, salida, errores),
        Orden::Variantes {
            maquina,
            programa,
            opciones,
        } => crate::variantes::variantes_cli(&maquina, &programa, &opciones, salida, errores),
        Orden::HistorialCiclos { archivo } => {
            crate::historial_ciclos::historial_cli(&archivo, salida, errores)
        }
        Orden::Nuevo {
            plantilla,
            destino,
            nombre,
        } => crate::nuevo::nuevo_cli(&plantilla, &destino, nombre.as_deref(), salida, errores),
        Orden::Convencion {
            maquina,
            salida: archivo,
        } => crate::compilar::convencion_cli(&maquina, archivo.as_deref(), salida, errores),
        Orden::Exportar {
            formato,
            maquina,
            programa,
            salida: destino,
        } => crate::exportar::exportar_cli(
            formato,
            &maquina,
            programa.as_deref(),
            destino.as_deref(),
            salida,
            errores,
        ),
        Orden::Importar {
            archivo,
            salida: destino,
        } => crate::importar::importar_cli(&archivo, destino.as_deref(), salida, errores),
        Orden::Documentar {
            maquina,
            salida: archivo,
            vigilar,
        } => crate::documentar::documentar_cli(
            &maquina,
            archivo.as_deref(),
            vigilar,
            salida,
            errores,
        ),
        Orden::Enlazar {
            maquina,
            entradas,
            opciones,
        } => crate::enlazar::enlazar_cli(&maquina, &entradas, &opciones, salida, errores),
        Orden::Desensamblar {
            maquina,
            programa,
            direccion,
            simbolos,
        } => crate::ensamblar::desensamblar_cli(
            &maquina,
            &programa,
            direccion,
            simbolos.as_deref(),
            salida,
            errores,
        ),
        Orden::Comparar {
            maquina_a,
            maquina_b,
            programa,
            limite,
            direccion,
        } => crate::comparar::comparar_cli(
            [&maquina_a, &maquina_b],
            &programa,
            limite,
            direccion,
            salida,
            errores,
        ),
        Orden::Ejecutar {
            maquina,
            programa,
            limite,
            traza,
            traza_maquina,
            pantalla,
            vigilar,
            cobertura,
            teclas,
            serie,
            volcados,
            disparos,
            guion,
            direccion,
        } => {
            let opciones = Opciones {
                limite,
                traza,
                traza_maquina,
                pantalla,
                vigilar,
                cobertura,
                teclas,
                serie,
                volcados,
                disparos,
                guion,
                direccion,
            };
            ejecutar(&maquina, &programa, &opciones, salida, errores)
        }
    }
}

fn validar(ruta: &str, salida: &mut dyn Write, errores: &mut dyn Write) -> i32 {
    match lorenz_arch::cargar_archivo(ruta) {
        Ok(m) => {
            escribir(
                salida,
                &format!(
                    "{ruta}: valida ('{}', {} instrucciones, {} regiones)\n",
                    m.nombre,
                    m.instrucciones.len(),
                    m.regiones.len()
                ),
            );
            SALIDA_OK
        }
        Err(e) => {
            escribir(errores, &format!("{ruta}: {e}\n"));
            SALIDA_ERROR_USO
        }
    }
}

/// Opciones de `lorenz ejecutar`.
struct Opciones {
    limite: u64,
    traza: bool,
    traza_maquina: bool,
    pantalla: bool,
    vigilar: bool,
    cobertura: bool,
    teclas: Option<String>,
    serie: Option<String>,
    volcados: Vec<Volcado>,
    disparos: Vec<Disparo>,
    guion: Option<String>,
    direccion: Option<u64>,
}

fn ejecutar(
    ruta_maquina: &str,
    ruta_programa: &str,
    opciones: &Opciones,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let Opciones {
        limite,
        traza,
        direccion,
        ..
    } = *opciones;
    let maquina = match lorenz_arch::cargar_archivo(ruta_maquina) {
        Ok(m) => m,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let inicio = direccion.unwrap_or(maquina.arranque);
    let segmentos = match crate::ensamblar::cargar_segmentos(&maquina, ruta_programa, inicio) {
        Ok(s) => s,
        Err(e) => {
            escribir(errores, &format!("{e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    let digitos = (maquina.bits_direccion as usize).div_ceil(4);
    let mut emulador = match Emulador::nuevo(maquina) {
        Ok(e) => e,
        Err(e) => {
            escribir(errores, &format!("{ruta_maquina}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    };
    if let Err(e) =
        lorenz_emu::programa::precargar_regiones(&mut emulador, &carpeta_de(ruta_maquina))
    {
        escribir(errores, &format!("{ruta_maquina}: {e}\n"));
        return SALIDA_ERROR_USO;
    }
    if opciones.vigilar {
        emulador.memoria_mut().activar_seguimiento();
        emulador.estado_mut().activar_seguimiento();
    }
    for (desde, palabras) in &segmentos {
        if let Err(e) = emulador.cargar_programa(*desde, palabras) {
            escribir(errores, &format!("{ruta_programa}: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    }
    emulador.estado_mut().fijar_pc(inicio);
    if let Some(teclas) = &opciones.teclas {
        let teclado = emulador
            .maquina()
            .dispositivos
            .iter()
            .find(|d| d.tipo == TipoDispositivo::Teclado)
            .map(|d| d.nombre.clone());
        let Some(teclado) = teclado else {
            escribir(
                errores,
                "error: la maquina no tiene teclado para --teclas\n",
            );
            return SALIDA_ERROR_USO;
        };
        for byte in teclas.bytes() {
            if let Err(e) = emulador.pulsar_tecla(&teclado, u64::from(byte)) {
                escribir(errores, &format!("error: {e}\n"));
                return SALIDA_ERROR_USO;
            }
        }
    }

    if let Some(texto) = &opciones.serie {
        let uart = emulador
            .maquina()
            .dispositivos
            .iter()
            .find(|d| d.tipo == TipoDispositivo::Uart)
            .map(|d| d.nombre.clone());
        let Some(uart) = uart else {
            escribir(errores, "error: la maquina no tiene UART para --serie\n");
            return SALIDA_ERROR_USO;
        };
        if let Err(e) = emulador.recibir_en_uart(&uart, texto.as_bytes()) {
            escribir(errores, &format!("error: {e}\n"));
            return SALIDA_ERROR_USO;
        }
    }

    // Eventos de la linea de comandos y del guion, ordenados por instruccion; a igual momento
    // conservan el orden en que se dieron.
    let mut eventos: Vec<Evento> = opciones
        .disparos
        .iter()
        .map(|d| Evento {
            instruccion: d.instruccion,
            accion: Accion::Interrupcion(d.linea),
        })
        .collect();
    if let Some(ruta) = &opciones.guion {
        let texto = match std::fs::read_to_string(ruta) {
            Ok(t) => t,
            Err(e) => {
                escribir(errores, &format!("{ruta}: no se pudo leer: {e}\n"));
                return SALIDA_ERROR_USO;
            }
        };
        match leer_guion(&texto) {
            Ok(g) => eventos.extend(g),
            Err(e) => {
                escribir(errores, &format!("{ruta}: {e}\n"));
                return SALIDA_ERROR_USO;
            }
        }
    }
    eventos.sort_by_key(|e| e.instruccion);
    let mut siguiente_evento = 0;
    let mut ejecutadas = 0u64;
    let mut cobertura = opciones
        .cobertura
        .then(|| lorenz_tst::cobertura::Cobertura::nueva(emulador.maquina()));
    let codigo = 'ejecucion: loop {
        if emulador.estado().detenida() {
            break SALIDA_OK;
        }
        if ejecutadas >= limite {
            let e = ErrorEjecucion::LimiteDeInstrucciones { limite };
            escribir(errores, &format!("error: {e}\n"));
            break SALIDA_LIMITE;
        }
        while siguiente_evento < eventos.len()
            && eventos[siguiente_evento].instruccion <= ejecutadas
        {
            if let Err(mensaje) = aplicar(&mut emulador, &eventos[siguiente_evento].accion) {
                escribir(errores, &format!("error: {mensaje}\n"));
                break 'ejecucion SALIDA_ERROR_USO;
            }
            siguiente_evento += 1;
        }
        match emulador.paso() {
            Ok(Some(p)) => {
                ejecutadas += 1;
                if let Some(c) = &mut cobertura {
                    c.registrar(ruta_programa, &p);
                }
                if traza && opciones.traza_maquina {
                    escribir(
                        salida,
                        &format!(
                            "pc={:#0ancho$x} op={} ciclos={} salto={}\n",
                            p.direccion,
                            p.mnemonico,
                            p.ciclos,
                            u8::from(p.salto),
                            ancho = digitos + 2
                        ),
                    );
                } else if traza {
                    escribir(
                        salida,
                        &format!(
                            "{:0ancho$x}  {:<6} {} ciclos{}\n",
                            p.direccion,
                            p.mnemonico,
                            p.ciclos,
                            if p.salto { "  salto" } else { "" },
                            ancho = digitos
                        ),
                    );
                }
            }
            Ok(None) => break SALIDA_OK,
            Err(e) => {
                escribir(
                    errores,
                    &format!(
                        "error en {:0ancho$x}: {e}\n",
                        emulador.estado().pc(),
                        ancho = digitos
                    ),
                );
                break SALIDA_ERROR_EJECUCION;
            }
        }
    };
    resumen(&mut emulador, digitos, salida);
    lecturas_sin_inicializar(&mut emulador, digitos, salida);
    interrupciones_atendidas(&emulador, digitos, salida);
    texto_de_uarts(&mut emulador, salida);
    for volcado in &opciones.volcados {
        if let Err(codigo) = volcar(&emulador, volcado, salida, errores) {
            return codigo;
        }
    }
    if opciones.pantalla {
        dibujar_pantallas(&emulador, salida);
    }
    if let Some(c) = &cobertura {
        escribir(salida, &format!("\n{}", c.informe().a_texto()));
    }
    codigo
}

/// Carpeta que contiene un archivo, para resolver las rutas relativas que este declara.
pub fn carpeta_de(ruta: &str) -> std::path::PathBuf {
    std::path::Path::new(ruta).parent().map_or_else(
        || std::path::PathBuf::from("."),
        std::path::Path::to_path_buf,
    )
}

/// Nombre del primer dispositivo de un tipo.
fn primer_dispositivo(emulador: &Emulador, tipo: TipoDispositivo) -> Option<String> {
    emulador
        .maquina()
        .dispositivos
        .iter()
        .find(|d| d.tipo == tipo)
        .map(|d| d.nombre.clone())
}

/// Aplica un evento de entrada a la maquina.
fn aplicar(emulador: &mut Emulador, accion: &Accion) -> Result<(), String> {
    match accion {
        Accion::Interrupcion(linea) => emulador
            .levantar_interrupcion(*linea)
            .map_err(|e| e.to_string()),
        Accion::Tecla(codigo) => {
            let teclado = primer_dispositivo(emulador, TipoDispositivo::Teclado)
                .ok_or("la maquina no tiene teclado para el guion")?;
            emulador
                .pulsar_tecla(&teclado, *codigo)
                .map_err(|e| e.to_string())
        }
        Accion::Serie(bytes) => {
            let uart = primer_dispositivo(emulador, TipoDispositivo::Uart)
                .ok_or("la maquina no tiene UART para el guion")?;
            emulador
                .recibir_en_uart(&uart, bytes)
                .map_err(|e| e.to_string())
        }
    }
}

/// Vuelca un rango de memoria a la salida o a un archivo, en el formato que `lorenz ejecutar`
/// sabe volver a cargar.
fn volcar(
    emulador: &Emulador,
    volcado: &Volcado,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> Result<(), i32> {
    let cantidad = usize::try_from(volcado.cantidad).unwrap_or(usize::MAX);
    let palabras = match emulador.memoria().volcar(volcado.desde, cantidad) {
        Ok(p) => p,
        Err(e) => {
            escribir(errores, &format!("error al volcar: {e}\n"));
            return Err(SALIDA_ERROR_USO);
        }
    };
    let bits = emulador.maquina().bits_palabra;
    let texto = escribir_palabras(&palabras, bits, 16);
    match &volcado.archivo {
        Some(ruta) => {
            if let Err(e) = std::fs::write(ruta, &texto) {
                escribir(errores, &format!("{ruta}: no se pudo escribir: {e}\n"));
                return Err(SALIDA_ERROR_USO);
            }
        }
        None => {
            let digitos = (emulador.maquina().bits_direccion as usize).div_ceil(4);
            escribir(
                salida,
                &format!(
                    "volcado desde {:0digitos$x}, {} palabras:\n{texto}",
                    volcado.desde,
                    palabras.len()
                ),
            );
        }
    }
    Ok(())
}

/// Avisa de las lecturas de memoria que nadie habia escrito (RF-EMU-026).
fn lecturas_sin_inicializar(emulador: &mut Emulador, digitos: usize, salida: &mut dyn Write) {
    for l in emulador.estado_mut().tomar_lecturas_sin_inicializar() {
        escribir(
            salida,
            &format!(
                "aviso: la instruccion en {:0digitos$x} leyo el registro {}, que nadie habia escrito\n",
                l.pc, l.registro
            ),
        );
    }
    for l in emulador.memoria().lecturas_sin_inicializar() {
        escribir(
            salida,
            &format!(
                "aviso: la instruccion en {:0digitos$x} leyo {:0digitos$x}, que nadie habia escrito\n",
                l.pc, l.direccion
            ),
        );
    }
}

/// Lista las interrupciones que la maquina atendio, una por linea (RF-EMU-039).
fn interrupciones_atendidas(emulador: &Emulador, digitos: usize, salida: &mut dyn Write) {
    for r in emulador.interrupciones_atendidas() {
        escribir(
            salida,
            &format!(
                "interrupcion: linea {} en el ciclo {} (instruccion {}), interrumpio {:0digitos$x}, manejador {:0digitos$x}\n",
                r.linea, r.ciclo, r.instruccion, r.pc_interrumpido, r.manejador
            ),
        );
    }
    let perdidas = emulador.interrupciones_sin_registrar();
    if perdidas > 0 {
        escribir(
            salida,
            &format!("interrupciones sin registrar: {perdidas}\n"),
        );
    }
}

/// Imprime lo que cada UART transmitio, si transmitio algo.
fn texto_de_uarts(emulador: &mut Emulador, salida: &mut dyn Write) {
    let nombres: Vec<String> = emulador
        .maquina()
        .dispositivos
        .iter()
        .filter(|d| d.tipo == TipoDispositivo::Uart)
        .map(|d| d.nombre.clone())
        .collect();
    for nombre in nombres {
        if let Ok(texto) = emulador.texto_de_uart(&nombre) {
            if !texto.is_empty() {
                let final_de_linea = if texto.ends_with('\n') { "" } else { "\n" };
                escribir(salida, &format!("uart {nombre}:\n{texto}{final_de_linea}"));
            }
        }
    }
}

fn dibujar_pantallas(emulador: &Emulador, salida: &mut dyn Write) {
    let nombres: Vec<String> = emulador
        .maquina()
        .dispositivos
        .iter()
        .filter(|d| matches!(d.tipo, TipoDispositivo::Pantalla { .. }))
        .map(|d| d.nombre.clone())
        .collect();
    for nombre in nombres {
        match emulador.pantalla(&nombre) {
            Ok(imagen) => escribir(salida, &format!("pantalla {nombre}:\n{}", imagen.a_texto())),
            Err(e) => escribir(salida, &format!("pantalla {nombre}: {e}\n")),
        }
    }
}

fn resumen(emulador: &mut Emulador, digitos: usize, salida: &mut dyn Write) {
    let estado = emulador.estado();
    let mut texto = String::new();
    texto.push_str(&format!(
        "detenida: {}\ninstrucciones: {}\nciclos: {}\npc: {:0ancho$x}\n",
        if estado.detenida() { "si" } else { "no" },
        estado.instrucciones(),
        estado.ciclos(),
        estado.pc(),
        ancho = digitos
    ));
    let registros: Vec<String> = estado
        .nombres()
        .iter()
        .map(|n| format!("{n}={:#x}", estado.registro(n).unwrap_or(0)))
        .collect();
    texto.push_str(&format!("registros: {}\n", registros.join(" ")));
    let banderas: Vec<String> = estado
        .nombres_de_banderas()
        .iter()
        .map(|n| format!("{n}={}", u8::from(estado.bandera(n).unwrap_or(false))))
        .collect();
    if !banderas.is_empty() {
        texto.push_str(&format!("banderas: {}\n", banderas.join(" ")));
    }
    let puertos = emulador.puertos().numeros();
    for p in puertos {
        let escritos = emulador
            .puertos_mut()
            .salida(p)
            .map(<[u64]>::to_vec)
            .unwrap_or_default();
        if !escritos.is_empty() {
            let lista: Vec<String> = escritos.iter().map(|v| format!("{v:#x}")).collect();
            texto.push_str(&format!("salida del puerto {p:#x}: {}\n", lista.join(" ")));
        }
    }
    escribir(salida, &texto);
}
