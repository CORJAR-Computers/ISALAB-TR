-- ============================================================================
-- ISALAB · Migración 0026 — Roles de clínica (RECEPCION y PELUQUERO)
--
-- La clínica integral necesita personal de recepción (check-in de visitas,
-- cobro) y peluqueros. Se amplía el CHECK de USERS.ROLE de 3 a 5 roles.
--
-- Firebird no permite ALTER de un CHECK: se localiza el constraint de la
-- columna ROLE en RDB$CHECK_CONSTRAINTS, se elimina y se recrea con nombre
-- fijo (CK_USERS_ROLE) para que esta migración sea idempotente en cualquier
-- instalación (0001 creó el constraint sin nombre explícito).
--
-- El rol ADMIN conserva todo; RECEPCION y PELUQUERO solo obtienen acceso a
-- los módulos de la clínica vía el mapa de capacidades de Rust (auth.rs).
-- ============================================================================

SET TERM ^ ;

-- Elimina los CHECK de la tabla USERS (en 0001 solo existe el de ROLE, sin
-- nombre explícito). Se usa RDB$RELATION_CONSTRAINTS (tabla de sistema real;
-- RDB$CONSTRAINTS no existe en Firebird) excluyendo nuestro constraint
-- nombrado, de modo que la migración sea idempotente.
EXECUTE BLOCK
AS
DECLARE VARIABLE CN VARCHAR(31);
BEGIN
  FOR SELECT TRIM(RDB$CONSTRAINT_NAME)
        FROM RDB$RELATION_CONSTRAINTS
       WHERE RDB$RELATION_NAME = 'USERS'
         AND RDB$CONSTRAINT_TYPE = 'CHECK'
         AND RDB$CONSTRAINT_NAME <> 'CK_USERS_ROLE'
       INTO :CN DO
  BEGIN
    EXECUTE STATEMENT 'ALTER TABLE USERS DROP CONSTRAINT ' || :CN;
  END
END^

SET TERM ; ^

ALTER TABLE USERS ADD CONSTRAINT CK_USERS_ROLE
    CHECK (ROLE IN ('ADMIN', 'VETERINARIO', 'AUXILIAR', 'RECEPCION', 'PELUQUERO'));
