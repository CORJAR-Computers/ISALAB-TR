-- ============================================================================
-- ISALAB · Migración 0032 — Estancia: recursos y estancias (fase 6)
--
-- Módulo de estancia de la clínica: guardería, hospedaje y hospitalización.
--
-- RESOURCES son los espacios físicos donde se alojan los pacientes (caniles,
-- gateras, pesebreras,Boxes de hospitalización). Cada recurso tiene una
-- capacidad (casi siempre 1) y un tipo.
--
-- STAYS es la estancia de un paciente: ingreso, salida, tipo de servicio,
-- tarifa diaria, recurso asignado y bitácora de cuidados (STAY_EVENTS:
-- comida, medicación, paseo, limpieza, nota, valoración veterinaria).
--
-- La estancia puede vincularse a la VISITA que la originó (check-in desde
-- recepción) pero no la reemplaza: la visita sigue siendo la columna vertebral
-- de la atención clínica.
-- ============================================================================

CREATE TABLE RESOURCES (
    ID          D_PK NOT NULL PRIMARY KEY,
    NAME        D_NAME NOT NULL,
    -- CANIL | GATO | RECEPERA | QUIRURGICO | AISLAMIENTO | PERRERA
    KIND        VARCHAR(20) CHARACTER SET UTF8 DEFAULT 'CANIL' NOT NULL
                CHECK (KIND IN ('CANIL', 'GATO', 'RECEPERA', 'QUIRURGICO',
                                'AISLAMIENTO', 'PERRERA')),
    CAPACITY    INTEGER DEFAULT 1 NOT NULL CHECK (CAPACITY > 0),
    ACTIVE      BOOLEAN DEFAULT TRUE NOT NULL,
    NOTES       D_NOTES,
    CREATED_AT  TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT  TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_RESOURCES_ID;

CREATE TABLE STAYS (
    ID            D_PK NOT NULL PRIMARY KEY,
    -- Código E-YYYY-NNNN (trigger BI_STAYS).
    CODE          D_CODE NOT NULL UNIQUE,
    PATIENT_ID    D_PK NOT NULL REFERENCES PATIENTS (ID) ON DELETE CASCADE,
    OWNER_ID      D_PK NOT NULL REFERENCES OWNERS (ID),
    -- Visita que originó el ingreso (opcional).
    VISIT_ID      D_PK REFERENCES VISITS (ID) ON DELETE SET NULL,
    -- Espacio asignado (NULL = sin asignar, p. ej. guardería sin canil fijo).
    RESOURCE_ID   D_PK REFERENCES RESOURCES (ID) ON DELETE SET NULL,
    -- GUARDERIA | HOSPEDAJE | HOSPITALIZACION
    KIND          VARCHAR(20) CHARACTER SET UTF8 DEFAULT 'GUARDERIA' NOT NULL
                  CHECK (KIND IN ('GUARDERIA', 'HOSPEDAJE', 'HOSPITALIZACION')),
    -- ABIERTA | CERRADA | CANCELADA
    STATUS        VARCHAR(20) CHARACTER SET UTF8 DEFAULT 'ABIERTA' NOT NULL
                  CHECK (STATUS IN ('ABIERTA', 'CERRADA', 'CANCELADA')),
    CHECK_IN_AT   TIMESTAMP NOT NULL,
    CHECK_OUT_AT  TIMESTAMP,
    -- Tarifa por día (COP); NULL = sin cobro (p. ej. hospitalización incluida).
    DAILY_RATE    D_AMOUNT DEFAULT 0 NOT NULL,
    NOTES         D_NOTES,
    CREATED_BY    D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    CREATED_AT    TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT    TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_STAYS_ID;

CREATE TABLE STAY_EVENTS (
    ID          D_PK NOT NULL PRIMARY KEY,
    STAY_ID     D_PK NOT NULL REFERENCES STAYS (ID) ON DELETE CASCADE,
    -- COMIDA | MEDICACION | PASEO | LIMPIEZA | NOTA | VETERINARIO
    KIND        VARCHAR(20) CHARACTER SET UTF8 DEFAULT 'NOTA' NOT NULL
                CHECK (KIND IN ('COMIDA', 'MEDICACION', 'PASEO', 'LIMPIEZA',
                                'NOTA', 'VETERINARIO')),
    OCCURRED_AT TIMESTAMP NOT NULL,
    NOTES       D_NOTES,
    CREATED_BY  D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    CREATED_AT  TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_STAY_EVENTS_ID;

CREATE INDEX IX_STAYS_STATUS ON STAYS (STATUS);
CREATE INDEX IX_STAYS_PATIENT ON STAYS (PATIENT_ID);
CREATE INDEX IX_STAYS_RESOURCE ON STAYS (RESOURCE_ID);
CREATE INDEX IX_STAYS_CHECK_IN ON STAYS (CHECK_IN_AT);
CREATE INDEX IX_STAY_EVENTS_STAY ON STAY_EVENTS (STAY_ID, OCCURRED_AT);

SET TERM ^ ;

CREATE TRIGGER BI_STAYS FOR STAYS BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_STAYS_ID, 1);
    IF (NEW.CODE IS NULL) THEN
        NEW.CODE = 'E-' || EXTRACT(YEAR FROM NEW.CHECK_IN_AT) || '-' ||
                   LPAD(NEW.ID, 4, '0');
END^

CREATE TRIGGER BI_STAY_EVENTS FOR STAY_EVENTS BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_STAY_EVENTS_ID, 1);
END^

CREATE TRIGGER BI_RESOURCES FOR RESOURCES BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_RESOURCES_ID, 1);
END^

SET TERM ; ^
