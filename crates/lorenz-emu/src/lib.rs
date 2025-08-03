//! Emulador funcional de Lorenz.
//!
//! Ejecuta programas sobre una maquina descrita con `lorenz-arch`. Todo lo que hace depende de la
//! descripcion: ningun opcode, registro ni mapa de memoria esta fijado en el codigo.
//!
//! El emulador es determinista: la misma maquina, el mismo programa y las mismas entradas producen
//! siempre el mismo resultado (RF-EMU-016).
