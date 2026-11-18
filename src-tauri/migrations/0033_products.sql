-- ============================================================================
-- ISALAB · Migración 0033 — Productos e inventario (fase 6)
--
-- Módulo de comercio de la clínica: alimentos, medicamentos, accesorios e
-- insumos que se venden en recepción y que además consume la propia clínica
-- durante las estancias (comida del hospedaje, insumos de limpieza).
--
-- PRODUCTS es el catálogo: precio de costo, precio de venta, IVA, unidad de
-- medida y stock mínimo (para alertas de reposición).
--
-- STOCK_MOVEMENTS es el libro de movimientos: cada línea suma (entrada) o
-- resta (salida) del stock del producto. El stock actual NUNCA se almacena
-- en PRODUCTS: es la suma de los movimientos, de modo que el histórico siempre
-- cuadra con el inventario (trazabilidad de tipo libro de auxilios).
--
--   ENTRADA     compra o donación        (+)
--   SALIDA      venta al cliente         (-)
--   CONSUMO     uso interno de la clínica (-)
--   DEVOLUCION  devolución del cliente   (+)
--   AJUSTE      corrección de inventario (+/-)
-- ============================================================================

CREATE TABLE PRODUCTS (
    ID          D_PK NOT NULL PRIMARY KEY,
    -- Código interno (SKU) único, p. ej. ALIM-001.
    CODE        D_CODE NOT NULL UNIQUE,
    NAME        D_NAME NOT NULL,
    -- ALIMENTO | MEDICAMENTO | ACCESORIO | INSUMO
    CATEGORY    VARCHAR(30) CHARACTER SET UTF8 DEFAULT 'INSUMO' NOT NULL
                CHECK (CATEGORY IN ('ALIMENTO', 'MEDICAMENTO', 'ACCESORIO', 'INSUMO')),
    -- UNIDAD | KG | LT | PAQUETE | CAJA | BOLSA
    UNIT        VARCHAR(20) CHARACTER SET UTF8 DEFAULT 'UNIDAD' NOT NULL,
    COST_PRICE  D_AMOUNT DEFAULT 0 NOT NULL,
    SALE_PRICE  D_AMOUNT DEFAULT 0 NOT NULL,
    -- IVA del producto; NULL = usa el IVA global de la clínica.
    TAX_RATE    D_PERCENT,
    -- Stock mínimo: por debajo de este valor el producto aparece en alertas.
    MIN_STOCK   INTEGER DEFAULT 0 NOT NULL,
    ACTIVE      BOOLEAN DEFAULT TRUE NOT NULL,
    NOTES       D_NOTES,
    CREATED_AT  TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT  TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_PRODUCTS_ID;

CREATE TABLE STOCK_MOVEMENTS (
    ID          D_PK NOT NULL PRIMARY KEY,
    PRODUCT_ID  D_PK NOT NULL REFERENCES PRODUCTS (ID) ON DELETE CASCADE,
    -- Positivo = entrada al inventario; negativo = salida.
    QUANTITY    INTEGER NOT NULL CHECK (QUANTITY <> 0),
    -- ENTRADA | SALIDA | CONSUMO | DEVOLUCION | AJUSTE
    KIND        VARCHAR(20) CHARACTER SET UTF8 DEFAULT 'ENTRADA' NOT NULL
                CHECK (KIND IN ('ENTRADA', 'SALIDA', 'CONSUMO', 'DEVOLUCION', 'AJUSTE')),
    -- Costo unitario del movimiento (para valorar el inventario).
    UNIT_COST   D_AMOUNT,
    REASON      D_NOTES,
    -- Origen del movimiento (opcional): estancia o visita que consumió.
    STAY_ID     D_PK REFERENCES STAYS (ID) ON DELETE SET NULL,
    VISIT_ID    D_PK REFERENCES VISITS (ID) ON DELETE SET NULL,
    CREATED_BY  D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    CREATED_AT  TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_STOCK_MOVEMENTS_ID;

CREATE INDEX IX_PRODUCTS_CATEGORY ON PRODUCTS (CATEGORY, ACTIVE);
CREATE INDEX IX_STOCK_MOVEMENTS_PRODUCT ON STOCK_MOVEMENTS (PRODUCT_ID, CREATED_AT);
CREATE INDEX IX_STOCK_MOVEMENTS_STAY ON STOCK_MOVEMENTS (STAY_ID);

SET TERM ^ ;

CREATE TRIGGER BI_PRODUCTS FOR PRODUCTS BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_PRODUCTS_ID, 1);
END^

CREATE TRIGGER BI_STOCK_MOVEMENTS FOR STOCK_MOVEMENTS BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_STOCK_MOVEMENTS_ID, 1);
END^

SET TERM ; ^
