//! Unidades de trabajo transaccionales para operaciones multipaso.
//!
//! Con una sola base central y 3 PCs escribiendo, las operaciones que tocan
//! varias tablas (factura + ítems, visita + líneas, movimiento + validación de
//! stock) no pueden quedar a medias: o se aplica todo o nada.
//!
//! [`in_transaction`] envuelve un bloque en `BEGIN … COMMIT/ROLLBACK` sobre la
//! misma conexión que ya usa el repositorio, así que las funciones internas
//! (`next_id`, INSERTs, lecturas de verificación) no cambian de firma.

use std::cell::Cell;

use rsfbclient::SimpleConnection;

use crate::error::AppError;

thread_local! {
    /// Profundidad de `in_transaction` en este hilo.
    ///
    /// Una operación transaccional puede llamar a otra (p. ej. el alta de
    /// peluquería crea una visita): la llamada interna debe **unirse** a la
    /// transacción abierta, no abrir la suya (un COMMIT interno rompería la
    /// atomicidad del bloque externo).
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Transacciones abiertas en este hilo. La usa el pool antes de reciclar una
/// conexión: si una operación entró en pánico, el bloque quedó a medias y la
/// profundidad sin restaurar.
pub(crate) fn open_depth() -> u32 {
    DEPTH.get()
}

/// Marca el hilo como sin transacción tras el rollback defensivo del pool.
pub(crate) fn clear_depth() {
    DEPTH.set(0);
}

/// Ejecuta `f` dentro de una transacción: COMMIT si `f` termina bien, ROLLBACK
/// si devuelve un error (o si entra en pánico, cubierto por el rollback
/// defensivo del pool).
///
/// Si ya hay una transacción abierta en este hilo, `f` simplemente se une a
/// ella: el COMMIT/ROLLBACK lo hace el nivel más externo. Los errores internos
/// se propagan y el nivel externo deshace todo el trabajo.
///
/// El contexto de error de un COMMIT/ROLLBACK fallido se envuelve con detalle
/// para distinguirlo del fallo original de `f`.
pub fn in_transaction<T>(
    conn: &mut SimpleConnection,
    f: impl FnOnce(&mut SimpleConnection) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let top = DEPTH.get() == 0;

    if top {
        conn.begin_transaction()
            .map_err(|e| AppError::Db(format!("No se pudo iniciar la transacción: {e}")))?;
    }

    DEPTH.set(DEPTH.get().saturating_add(1));
    let result = f(conn);
    DEPTH.set(DEPTH.get().saturating_sub(1));

    if !top {
        // Nivel interno: comparte la transacción del llamador.
        return result;
    }

    match result {
        Ok(value) => {
            conn.commit()
                .map_err(|e| AppError::Db(format!("No se pudo confirmar la transacción: {e}")))?;
            Ok(value)
        }
        Err(e) => {
            // El rollback no debe enmascarar el error original.
            if let Err(rb) = conn.rollback() {
                eprintln!("[db] rollback tras error falló también: {rb}");
            }
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsfbclient::prelude::*;

    use crate::test_helpers::{cleanup_test_db, setup_test_db};

    #[test]
    fn commits_all_or_nothing() {
        let (mut conn, db_path) = setup_test_db();

        // Todo el bloque se aplica.
        in_transaction(&mut conn, |c| {
            c.execute(
                "INSERT INTO SPECIES (ID, CODE, NAME) VALUES (8001, 'T01', 'Tx commit')",
                (),
            )
            .map_err(AppError::from)?;
            c.execute(
                "INSERT INTO SPECIES (ID, CODE, NAME) VALUES (8002, 'T02', 'Tx commit 2')",
                (),
            )
            .map_err(AppError::from)?;
            Ok(())
        })
        .expect("bloque correcto");

        let count: Option<(i32,)> = conn
            .query_first("SELECT COUNT(*) FROM SPECIES WHERE CODE LIKE 'T0%'", ())
            .expect("contar");
        assert_eq!(count.map(|(n,)| n).unwrap_or(0), 2);

        // El bloque fallido no deja la primera fila aplicada.
        let err = in_transaction(&mut conn, |c| {
            c.execute(
                "INSERT INTO SPECIES (ID, CODE, NAME) VALUES (8003, 'T03', 'Tx rollback')",
                (),
            )
            .map_err(AppError::from)?;
            c.execute("INSERT INTO SPECIES (ID, CODE) VALUES (8004)", ())
                .map_err(AppError::from)?;
            Ok(())
        })
        .expect_err("segunda sentencia inválida");
        // El error es el del INSERT inválido (no un COMMIT fallido ni un Ok).
        assert!(matches!(&err, AppError::Db(_)), "{err}");

        let count: Option<(i32,)> = conn
            .query_first("SELECT COUNT(*) FROM SPECIES WHERE CODE LIKE 'T0%'", ())
            .expect("contar");
        assert_eq!(
            count.map(|(n,)| n).unwrap_or(0),
            2,
            "el rollback deshace todo el bloque"
        );

        // La conexión vuelve a un modo usable: una escritura suelta se aplica.
        conn.execute(
            "INSERT INTO SPECIES (ID, CODE, NAME) VALUES (8005, 'T05', 'Tx libre')",
            (),
        )
        .expect("escritura tras la transacción");

        drop(conn);
        cleanup_test_db(&db_path);
    }

    #[test]
    fn nested_calls_join_the_outer_transaction() {
        let (mut conn, db_path) = setup_test_db();

        in_transaction(&mut conn, |c| {
            c.execute(
                "INSERT INTO SPECIES (ID, CODE, NAME) VALUES (8101, 'N01', 'Nested')",
                (),
            )
            .map_err(AppError::from)?;
            // La llamada interna ve los cambios no confirmados de la externa
            // (misma transacción) y no cierra la transacción del llamador.
            in_transaction(c, |c2| {
                let count: Option<(i32,)> = c2
                    .query_first("SELECT COUNT(*) FROM SPECIES WHERE CODE = 'N01'", ())
                    .map_err(AppError::from)?;
                assert_eq!(count.map(|(n,)| n).unwrap_or(0), 1);
                Ok(())
            })
            .expect("nivel interno");
            Ok(())
        })
        .expect("bloque anidado");

        let count: Option<(i32,)> = conn
            .query_first("SELECT COUNT(*) FROM SPECIES WHERE CODE = 'N01'", ())
            .expect("contar");
        assert_eq!(count.map(|(n,)| n).unwrap_or(0), 1);

        drop(conn);
        cleanup_test_db(&db_path);
    }

    /// Un pánico dentro del bloque no puede dejar la transacción viva: la
    /// profundidad queda marcada para que el pool deshaga el trabajo al
    /// reciclar la conexión.
    #[test]
    fn a_panic_leaves_the_transaction_for_the_pool_to_roll_back() {
        let (mut conn, db_path) = setup_test_db();

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = in_transaction::<()>(&mut conn, |c| {
                c.execute(
                    "INSERT INTO SPECIES (ID, CODE, NAME) VALUES (8201, 'P01', 'Panic')",
                    (),
                )
                .map_err(AppError::from)?;
                panic!("fallo simulado dentro de la transacción");
            });
        }));
        assert!(panicked.is_err());
        assert_eq!(open_depth(), 1, "la transacción sigue abierta");

        // Lo que hace `DbPool::release` con una conexión en ese estado.
        conn.rollback().expect("rollback defensivo");
        clear_depth();

        assert_eq!(open_depth(), 0);
        let count: Option<(i32,)> = conn
            .query_first("SELECT COUNT(*) FROM SPECIES WHERE CODE = 'P01'", ())
            .expect("contar");
        assert_eq!(
            count.map(|(n,)| n).unwrap_or(0),
            0,
            "el INSERT no confirmado se deshace"
        );

        drop(conn);
        cleanup_test_db(&db_path);
    }
}
