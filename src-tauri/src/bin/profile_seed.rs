//! Fixture de perfilado: genera una base Firebird temporal con ~100k
//! resultados de laboratorio para medir los planes y tiempos de las queries
//! de producción (`list_results`, delta check, worklist).
//!
//! Escala simulada: ~3 años de un laboratorio ocupado — 2 000 pacientes,
//! 3 400 muestras (30 analitos promedio) y ~102 000 resultados, más ~600
//! EVENT_LOG para que la poda inicial de la 0023 tenga material.
//!
//! Uso:  cargo run --bin profile_seed <ruta.fdb>
//! (la base se deja en disco para perfilarla con isql; bórrala al terminar)

use std::path::PathBuf;
use std::time::Instant;

use isalab_lib::db;
use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

fn main() {
    let Some(db_path) = std::env::args().nth(1) else {
        eprintln!("uso: profile_seed <ruta.fdb>");
        std::process::exit(2);
    };
    let db_path = PathBuf::from(db_path);
    let _ = std::fs::remove_file(&db_path);

    let fbclient = resolve_fbclient();
    println!("fbclient: {}", fbclient.display());

    let t0 = Instant::now();
    let mut conn =
        db::create_database(&db_path, &fbclient).expect("no se pudo crear la base de perfilado");

    let version =
        isalab_lib::db::migrations::run_migrations(&mut conn).expect("migraciones fallaron");
    println!(
        "migraciones aplicadas (schema v{version}) en {:?}",
        t0.elapsed()
    );

    seed(&mut conn);

    println!("listo en {:?} → {}", t0.elapsed(), db_path.display());
}

/// Resuelve fbclient.dll igual que los tests (test_helpers.rs):
/// FIREBIRD_DIR → binaries/firebird → ../Firebird-5.0.3... del repo.
fn resolve_fbclient() -> PathBuf {
    let candidates: &[PathBuf] = &[
        std::env::var("FIREBIRD_DIR")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_default(),
        PathBuf::from("binaries/firebird"),
        PathBuf::from("../Firebird-5.0.3.1683-0-windows-x64"),
    ];
    for dir in candidates {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let dll = dir.join("fbclient.dll");
        if dll.exists() {
            return dll;
        }
    }
    panic!("fbclient.dll no encontrado (FIREBIRD_DIR, binaries/firebird, ../Firebird-5.0.3...)");
}

fn seed(conn: &mut SimpleConnection) {
    let t = Instant::now();

    // El seed de demo de la 0005 estorbó a los tests (filas sueltas); aquí no
    // lo necesitamos y en cambio nos conviene un volumen limpio y controlado.
    // Mismo orden que test_helpers (hijos antes que padres) más lo que
    // agregaron migraciones posteriores; cada DELETE es best-effort.
    for stmt in [
        "DELETE FROM EVENT_LOG",
        "DELETE FROM RESULT_ATTACHMENTS",
        "DELETE FROM SAMPLE_EVENTS",
        "DELETE FROM LAB_ORDER_ITEMS",
        "DELETE FROM LAB_ORDERS",
        "DELETE FROM INVOICE_ITEMS",
        "DELETE FROM INVOICES",
        "DELETE FROM SURGERIES",
        "DELETE FROM VACCINES",
        "DELETE FROM LAB_RESULTS",
        "DELETE FROM SAMPLES",
        "DELETE FROM CONSULTATIONS",
        "DELETE FROM PATIENTS",
        "DELETE FROM OWNERS",
    ] {
        conn.execute(stmt, ()).ok();
    }

    // Catálogo: los 44 analitos sembrados por 0021 ya traen sus rangos
    // (perfil General, por especie) — la migración es parte del fixture.

    // 2 000 propietarios y pacientes.
    conn.execute(
        "EXECUTE BLOCK AS DECLARE VARIABLE i INTEGER; BEGIN
           i = 1;
           WHILE (i <= 2000) DO BEGIN
             INSERT INTO OWNERS (ID, DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME, PHONE)
             VALUES (:i, 'CC', 'DOC' || :i, 'Propietario ' || :i, '3000000000');
             INSERT INTO PATIENTS (ID, OWNER_ID, SPECIES_ID, BREED_ID, NAME, SEX, BIRTH_DATE, ACTIVE)
             VALUES (:i, :i, 1, 1, 'Paciente ' || :i, 'M', DATEADD(-400 - MOD(:i, 800) DAY TO CURRENT_DATE), TRUE);
             i = i + 1;
           END
         END",
        (),
    )
    .expect("propietarios y pacientes");

    // 3 400 muestras en ~3 años, repartidas entre los pacientes. Un 20 %
    // queda RECIBIDA/EN_PROCESO para que la bandeja de trabajo tenga cuerpo.
    conn.execute(
        "EXECUTE BLOCK AS DECLARE VARIABLE i INTEGER; DECLARE VARIABLE pid INTEGER;
         BEGIN
           i = 1;
           WHILE (i <= 3400) DO BEGIN
             pid = 1 + MOD(i * 7, 2000);
             INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (:i,
                     'M-2023-' || LPAD(:i, 6, '0'),
                     :pid,
                     1,
                     DATEADD(-1095 + (:i * 1095 / 3400) DAY TO CURRENT_TIMESTAMP),
                     CASE WHEN MOD(:i, 5) = 0 THEN 'RECIBIDA'
                          WHEN MOD(:i, 5) = 1 THEN 'EN_PROCESO'
                          ELSE 'FINALIZADA' END);
             i = i + 1;
           END
         END",
        (),
    )
    .expect("muestras");

    // ~102k resultados: 30 analitos (los primeros del catálogo) por muestra.
    conn.execute(
        "EXECUTE BLOCK AS DECLARE VARIABLE s INTEGER; DECLARE VARIABLE a INTEGER;
         DECLARE VARIABLE gid INTEGER;
         BEGIN
           gid = 1;
           s = 1;
           WHILE (s <= 3400) DO BEGIN
             a = 1;
             WHILE (a <= 30) DO BEGIN
               INSERT INTO LAB_RESULTS (ID, SAMPLE_ID, ANALYTE_ID, RESULT_VALUE, STATUS, ANALYZED_AT)
               VALUES (:gid, :s, :a,
                       1 + MOD(:s * :a, 120),
                       CASE WHEN MOD(:s * :a, 17) = 0 THEN 'ALTO'
                            WHEN MOD(:s * :a, 23) = 0 THEN 'BAJO'
                            ELSE 'NORMAL' END,
                       DATEADD(-1000 + (:s * 1000 / 3400) DAY TO CURRENT_TIMESTAMP));
               gid = gid + 1;
               a = a + 1;
             END
             s = s + 1;
           END
         END",
        (),
    )
    .expect("resultados de laboratorio");

    // EVENT_LOG: los triggers AI_SAMPLES ya insertaron una fila por muestra;
    // esparcimos su antigüedad (hasta 90 días) para que la poda de la 0023
    // tenga material real que borrar.
    conn.execute(
        "UPDATE EVENT_LOG
            SET CREATED_AT = DATEADD(-MOD(ID, 90) DAY TO CURRENT_TIMESTAMP)
          WHERE CREATED_AT > CURRENT_TIMESTAMP - 90",
        (),
    )
    .expect("event log");

    // Estadísticas del optimizador frescas (así trabaja una instalación
    // real después de usar la app; sin esto los planes serían pesimistas).
    conn.execute("EXECUTE PACKAGE RDB$ADMIN", ()).ok();
    for table in [
        "LAB_RESULTS",
        "SAMPLES",
        "PATIENTS",
        "EVENT_LOG",
        "REFERENCE_RANGES",
        "ANALYTES",
    ] {
        let _ = conn.execute("SET STATISTICS INDEX ALL", ());
        let _ = conn.execute(&format!("UPDATE STATISTICS {table}"), ());
    }

    let counts: Vec<(String, i64)> = conn
        .query(
            "SELECT 'LAB_RESULTS', COUNT(*) FROM LAB_RESULTS
             UNION ALL SELECT 'SAMPLES', COUNT(*) FROM SAMPLES
             UNION ALL SELECT 'PATIENTS', COUNT(*) FROM PATIENTS
             UNION ALL SELECT 'EVENT_LOG', COUNT(*) FROM EVENT_LOG",
            (),
        )
        .expect("conteos");
    for (name, count) in counts {
        println!("  {name}: {count}");
    }
    println!("seed en {:?}", t.elapsed());
}
