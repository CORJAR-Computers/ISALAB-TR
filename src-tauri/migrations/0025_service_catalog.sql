-- ============================================================================
-- ISALAB · Migración 0025 — Catálogo de servicios y precios (fase 1 clínica)
--
-- El laboratorio tenía facturación de texto libre (INVOICE_ITEMS.DESCRIPTION).
-- Con la clínica integral el catálogo de servicios (SERVICE_CATEGORIES +
-- SERVICES) se convierte en la ÚNICA fuente de precio, duración, IVA y
-- profesional requerido de cada línea de negocio: consulta, vacunación,
-- cirugía, laboratorio, peluquería, etc.
--
-- Los precios sembrados son una PROPUESTA de referencia en COP: la clínica
-- los ajusta desde Configuración → Servicios antes de operar.
-- El IVA por servicio (TAX_RATE) queda NULL = usa el IVA global de
-- CLINIC_SETTINGS ('invoice.tax_rate'); se valida con el contador antes de
-- fijar tarifas específicas.
--
-- Además, INVOICE_ITEMS gana SERVICE_ID (FK opcional) para saber qué línea
-- salió del catálogo; el precio de la factura sigue siendo una copia, de modo
-- que cambiar la lista de precios nunca reescribe facturas históricas.
-- ============================================================================

CREATE TABLE SERVICE_CATEGORIES (
    ID         D_PK NOT NULL PRIMARY KEY,
    -- CONSULTA, PREVENTIVO, CIRUGIA, LABORATORIO, PELUQUERIA, OTROS
    CODE       D_CODE NOT NULL UNIQUE,
    NAME       D_NAME NOT NULL,
    SORT_ORDER INTEGER DEFAULT 0 NOT NULL,
    IS_ACTIVE  BOOLEAN DEFAULT TRUE NOT NULL
);
CREATE GENERATOR GEN_SERVICE_CATEGORIES_ID;

CREATE TABLE SERVICES (
    ID               D_PK NOT NULL PRIMARY KEY,
    CODE             D_CODE NOT NULL UNIQUE,
    NAME             D_NAME NOT NULL,
    CATEGORY_ID      D_PK NOT NULL REFERENCES SERVICE_CATEGORIES (ID),
    DESCRIPTION      D_NOTES,
    -- Duración estimada en minutos (alimenta la agenda de la fase 3).
    DURATION_MIN     INTEGER DEFAULT 30 NOT NULL CHECK (DURATION_MIN > 0),
    -- Precio de lista en COP (la factura guarda su propia copia).
    PRICE            D_AMOUNT DEFAULT 0 NOT NULL CHECK (PRICE >= 0),
    -- NULL = usa el IVA global de CLINIC_SETTINGS ('invoice.tax_rate').
    TAX_RATE         D_PERCENT,
    REQUIRES_VET     BOOLEAN DEFAULT FALSE NOT NULL,
    REQUIRES_GROOMER BOOLEAN DEFAULT FALSE NOT NULL,
    -- NULL = aplica a todas las especies.
    SPECIES_ID       D_PK REFERENCES SPECIES (ID) ON DELETE SET NULL,
    SORT_ORDER       INTEGER DEFAULT 0 NOT NULL,
    IS_ACTIVE        BOOLEAN DEFAULT TRUE NOT NULL,
    CREATED_AT       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_SERVICES_ID;

CREATE INDEX IX_SERVICES_CATEGORY ON SERVICES (CATEGORY_ID);
CREATE INDEX IX_SERVICES_ACTIVE ON SERVICES (IS_ACTIVE);
CREATE INDEX IX_SERVICES_SPECIES ON SERVICES (SPECIES_ID);

-- Línea de factura vinculada a un servicio del catálogo (opcional: las
-- líneas manuales siguen existiendo sin SERVICE_ID).
ALTER TABLE INVOICE_ITEMS ADD SERVICE_ID D_PK REFERENCES SERVICES (ID) ON DELETE SET NULL;
CREATE INDEX IX_INVOICE_ITEMS_SERVICE ON INVOICE_ITEMS (SERVICE_ID);

SET TERM ^ ;

CREATE TRIGGER BI_SERVICE_CATEGORIES FOR SERVICE_CATEGORIES BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_SERVICE_CATEGORIES_ID, 1);
END^

CREATE TRIGGER BI_SERVICES FOR SERVICES BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_SERVICES_ID, 1);
END^

SET TERM ; ^

-- =========================== SEMILLA DE CATEGORÍAS ==========================
INSERT INTO SERVICE_CATEGORIES (ID, CODE, NAME, SORT_ORDER)
    VALUES (1, 'CONSULTA',    'Consultas',                  1);
INSERT INTO SERVICE_CATEGORIES (ID, CODE, NAME, SORT_ORDER)
    VALUES (2, 'PREVENTIVO',  'Medicina preventiva',        2);
INSERT INTO SERVICE_CATEGORIES (ID, CODE, NAME, SORT_ORDER)
    VALUES (3, 'CIRUGIA',     'Cirugía y procedimientos',   3);
INSERT INTO SERVICE_CATEGORIES (ID, CODE, NAME, SORT_ORDER)
    VALUES (4, 'LABORATORIO', 'Laboratorio',                4);
INSERT INTO SERVICE_CATEGORIES (ID, CODE, NAME, SORT_ORDER)
    VALUES (5, 'PELUQUERIA',  'Peluquería y estética',      5);
INSERT INTO SERVICE_CATEGORIES (ID, CODE, NAME, SORT_ORDER)
    VALUES (6, 'OTROS',       'Otros servicios',            6);

-- =========================== SEMILLA DE SERVICIOS ===========================
-- Precios de referencia en COP; la clínica los ajusta antes de operar.
-- ============================ Consultas =====================================
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_VET, SORT_ORDER)
    VALUES (1, 'CONS-GEN',  'Consulta general',            1, 30,  60000,  TRUE, 1);
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_VET, SORT_ORDER)
    VALUES (2, 'CONS-ESP',  'Consulta de especialidad',    1, 45,  90000,  TRUE, 2);
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_VET, SORT_ORDER)
    VALUES (3, 'CONS-REV',  'Revisión / control',          1, 20,  40000,  TRUE, 3);
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_VET, SORT_ORDER)
    VALUES (4, 'CONS-DOM',  'Consulta a domicilio',        1, 60,  80000,  TRUE, 4);
-- ========================= Medicina preventiva ==============================
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_VET, SORT_ORDER)
    VALUES (5, 'PREV-VAC',  'Vacunación',                  2, 20,  45000,  TRUE, 1);
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_VET, SORT_ORDER)
    VALUES (6, 'PREV-DES',  'Desparasitación',             2, 15,  25000,  TRUE, 2);
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_VET, SORT_ORDER)
    VALUES (7, 'PREV-CHIP', 'Implantación de microchip',   2, 20,  90000,  TRUE, 3);
-- ===================== Cirugía y procedimientos =============================
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_VET, SORT_ORDER)
    VALUES (8, 'CIR-MEN',   'Cirugía menor',               3, 60, 150000,  TRUE, 1);
-- ============================= Laboratorio ==================================
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_VET, SORT_ORDER)
    VALUES (9,  'LAB-BAS',  'Perfil básico de laboratorio',   4, 30,  60000, FALSE, 1);
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_VET, SORT_ORDER)
    VALUES (10, 'LAB-COM',  'Perfil completo de laboratorio', 4, 30, 120000, FALSE, 2);
-- ======================= Peluquería y estética ==============================
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_GROOMER, SORT_ORDER)
    VALUES (11, 'PEL-BANO',     'Baño',                     5,  60,  40000, TRUE, 1);
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_GROOMER, SORT_ORDER)
    VALUES (12, 'PEL-BANO-MED', 'Baño medicado',            5,  75,  60000, TRUE, 2);
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_GROOMER, SORT_ORDER)
    VALUES (13, 'PEL-RAZA',     'Peluquería de raza',       5, 120,  70000, TRUE, 3);
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_GROOMER, SORT_ORDER)
    VALUES (14, 'PEL-HIGI',     'Corte higiénico',          5,  45,  35000, TRUE, 4);
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_GROOMER, SORT_ORDER)
    VALUES (15, 'PEL-DESL',     'Deslanado',                5,  90,  65000, TRUE, 5);
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_GROOMER, SORT_ORDER)
    VALUES (16, 'PEL-SPA',      'Spa canino',               5,  90,  85000, TRUE, 6);
INSERT INTO SERVICES (ID, CODE, NAME, CATEGORY_ID, DURATION_MIN, PRICE, REQUIRES_GROOMER, SORT_ORDER)
    VALUES (17, 'PEL-DENT',     'Limpieza dental estética', 5,  45,  50000, TRUE, 7);

-- Los generadores quedan sincronizados con los IDs sembrados.
SET GENERATOR GEN_SERVICE_CATEGORIES_ID TO 6;
SET GENERATOR GEN_SERVICES_ID TO 17;
