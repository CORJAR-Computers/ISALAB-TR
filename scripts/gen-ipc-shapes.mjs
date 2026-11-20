// ============================================================================
// Forma de los comandos IPC: esquema por comando generado desde bindings.ts,
// verificado contra los tipos de Rust y usado por el mock de los E2E.
//
// Por qué: `check:bindings` y `check:e2e-mock` validan *nombres* de comandos,
// no su contenido. Un mock que devuelve `patient_id` donde la UI espera
// `patientId`, o un campo de Rust renombrado sin regenerar bindings, pasaban
// todos los controles y fallaban en la clínica. Aquí:
//   1. Se deriva de src/bindings.ts, con el compilador de TypeScript, la forma
//      de la respuesta y las claves del payload de cada comando → se escribe en
//      e2e/ipc-shapes.json (versionado).
//   2. Se comparan los `pub struct` de Rust con los tipos homónimos de
//      bindings.ts: un campo mal nombrado (serde rename / camelCase) falla.
//   3. En los E2E, el mock valida cada respuesta contra ese esquema, así que un
//      cambio de nombre en el mock rompe el test, no la instalación.
//
// Uso:  node scripts/gen-ipc-shapes.mjs          (verifica; CI)
//       node scripts/gen-ipc-shapes.mjs --write  (regenera el JSON)
// ============================================================================

import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const BINDINGS = "src/bindings.ts";
const shapesPath = join(root, "e2e/ipc-shapes.json");
const write = process.argv.includes("--write");

const program = ts.createProgram([join(root, BINDINGS)], {
  target: ts.ScriptTarget.ES2022,
  module: ts.ModuleKind.ESNext,
  moduleResolution: ts.ModuleResolutionKind.Bundler,
  strict: true,
  noEmit: true,
  skipLibCheck: true,
});
const checker = program.getTypeChecker();
const source = program.getSourceFile(join(root, BINDINGS));
if (!source) {
  console.error(`No se pudo leer ${BINDINGS}`);
  process.exit(1);
}

// ---------- 1. Forma por comando desde bindings.ts ----------

/** Nombre del comando dentro de un `__TAURI_INVOKE("...")`, esté o no envuelto. */
function invokeCallOf(node) {
  if (ts.isCallExpression(node)) {
    const text = node.expression.getText();
    if (text.endsWith("__TAURI_INVOKE")) return node;
  }
  let found = null;
  node.forEachChild((child) => {
    found ??= invokeCallOf(child);
  });
  return found;
}

const MAX_DEPTH = 8;

/**
 * Tipos con nombre declarados en bindings.ts: se guardan una sola vez y las
 * formas los referencian (`{ kind: "ref", name }`). Sin esto el JSON se
 * duplica por cada comando que devuelve el mismo tipo (387 KB).
 */
const definitions = {};

function namedTypeOf(type) {
  const symbol = type.aliasSymbol ?? type.symbol;
  const name = symbol?.escapedName ?? symbol?.name;
  if (!name || String(name).startsWith("__")) return null;
  const declarations = symbol?.declarations ?? [];
  if (declarations.length === 0) return null;
  // Solo tipos declarados en bindings.ts (no Array, Promise o tipos de lib).
  const fromBindings = declarations.every((node) => node.getSourceFile() === source);
  return fromBindings ? String(name) : null;
}

/** Convierte un tipo de TS en la forma mínima que el mock puede validar. */
function shapeOf(type, depth = 0) {
  if (!type || depth > MAX_DEPTH) return { kind: "unknown" };
  const name = namedTypeOf(type);
  if (name) {
    if (!(name in definitions)) {
      definitions[name] = { kind: "unknown" }; // evita recursión infinita
      definitions[name] = shapeBody(type, depth);
    }
    return { kind: "ref", name };
  }
  return shapeBody(type, depth);
}

function shapeBody(type, depth) {
  if (type.isUnion?.()) {
    const parts = type.types
      .filter((part) => !(part.flags & ts.TypeFlags.Undefined))
      .map((part) => shapeOf(part, depth + 1));
    const unique = [];
    for (const part of parts) {
      if (!unique.some((u) => JSON.stringify(u) === JSON.stringify(part))) {
        unique.push(part);
      }
    }
    if (unique.length === 0) return { kind: "unknown" };
    if (unique.length === 1) return unique[0];
    return { kind: "union", of: unique };
  }
  if (checker.isArrayType(type)) {
    const [item] = checker.getTypeArguments(type);
    return { kind: "array", items: shapeOf(item, depth + 1) };
  }
  if (type.flags & ts.TypeFlags.String) return { kind: "string" };
  if (type.flags & ts.TypeFlags.Number) return { kind: "number" };
  if (type.flags & ts.TypeFlags.Boolean) return { kind: "boolean" };
  if (type.flags & (ts.TypeFlags.Null | ts.TypeFlags.Void | ts.TypeFlags.Undefined)) {
    return { kind: "null" };
  }
  if (type.getStringIndexType()) return { kind: "record" };
  const props = type.getProperties?.() ?? [];
  if (props.length === 0) return { kind: "object", fields: {} };
  if (!(type.flags & ts.TypeFlags.Object)) return { kind: "unknown" };

  const fields = {};
  for (const prop of props) {
    const optional = (prop.flags & ts.SymbolFlags.Optional) !== 0;
    const propType = checker.getTypeOfSymbolAtLocation(prop, source);
    const field = { shape: shapeOf(propType, depth + 1) };
    if (optional) field.optional = true;
    fields[prop.name] = field;
  }
  return { kind: "object", fields };
}

/** Tipo "await" de la llamada (invoke devuelve Promise<T>). */
function resultTypeOf(call) {
  const type = checker.getTypeAtLocation(call);
  const awaited = checker.getAwaitedType?.(type) ?? type;
  return awaited;
}

const commands = {};
const typeFields = {};
for (const statement of source.statements) {
  if (!ts.isVariableStatement(statement)) continue;
  const declaration = statement.declarationList.declarations[0];
  if (!declaration || declaration.name.getText() !== "commands") continue;
  const table = declaration.initializer;
  if (!ts.isObjectLiteralExpression(table)) continue;

  for (const property of table.properties) {
    if (!ts.isPropertyAssignment(property)) continue;
    const call = invokeCallOf(property.initializer);
    if (!call) continue;
    const [commandArg, payloadArg] = call.arguments;
    if (!commandArg || !ts.isStringLiteral(commandArg)) continue;
    const command = commandArg.text;
    const argKeys =
      payloadArg && ts.isObjectLiteralExpression(payloadArg)
        ? payloadArg.properties
            .map((prop) => (prop.name ? prop.name.getText().replace(/['"]/g, "") : null))
            .filter(Boolean)
            .sort()
        : [];
    commands[command] = {
      result: shapeOf(resultTypeOf(call)),
      args: argKeys,
    };
  }

  // Tipos exportados de bindings.ts (para compararlos con los structs de Rust).
  for (const inner of source.statements) {
    if (!ts.isTypeAliasDeclaration(inner)) continue;
    const type = checker.getTypeAtLocation(inner.name);
    if (!(type.flags & ts.TypeFlags.Object) || type.getStringIndexType()) continue;
    typeFields[inner.name.getText()] = type
      .getProperties()
      .map((prop) => prop.name)
      .sort();
  }
}

const sortedCommands = Object.fromEntries(
  Object.keys(commands)
    .sort()
    .map((key) => [key, commands[key]]),
);
const sortedTypes = Object.fromEntries(
  Object.keys(typeFields)
    .sort()
    .map((key) => [key, typeFields[key]]),
);
const sortedDefinitions = Object.fromEntries(
  Object.keys(definitions)
    .sort()
    .map((key) => [key, definitions[key]]),
);
const generated = {
  comment:
    "Generado por scripts/gen-ipc-shapes.mjs desde src/bindings.ts. No editar a mano: `npm run gen:ipc-shapes`.",
  commands: sortedCommands,
  types: sortedDefinitions,
};

// ---------- 2. Rust: `pub struct` con serde ↔ bindings.ts ----------

function rustFiles(dir, acc = []) {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) rustFiles(path, acc);
    else if (path.endsWith(".rs")) acc.push(path);
  }
  return acc;
}

const camel = (name) =>
  name.replace(/_([a-z0-9])/g, (_, char) => char.toUpperCase());

/** Structs con sus campos ya normalizados a como los ve el frontend. */
function parseRustStructs(files) {
  const structs = {};
  for (const file of files) {
    const text = readFileSync(file, "utf-8");
    const lines = text.split(/\r?\n/);
    for (let index = 0; index < lines.length; index += 1) {
      const match = lines[index].match(/^pub struct (\w+)[\s\S]*?\{$/);
      if (!match) continue;
      const name = match[1];
      // `rename_all` se busca en los atributos inmediatamente anteriores.
      let renameAll = null;
      for (let back = index - 1; back >= Math.max(0, index - 6); back -= 1) {
        const attr = lines[back].match(/rename_all\s*=\s*"(\w+)"/);
        if (attr) renameAll = attr[1];
        if (/^\s*pub struct/.test(lines[back])) break;
      }
      const fields = [];
      let pendingSkip = false;
      let pendingRename = null;
      for (let cursor = index + 1; cursor < lines.length; cursor += 1) {
        const line = lines[cursor];
        if (/^\}/.test(line)) break;
        if (/#\[serde\([^)]*skip/.test(line)) pendingSkip = true;
        const rename = line.match(/#\[serde\([^)]*rename\s*=\s*"([^"]+)"/);
        if (rename) pendingRename = rename[1];
        const field = line.match(/^\s{2,}pub (\w+)\s*:\s*(.+?),?\s*$/);
        if (!field) continue;
        const [, rawName] = field;
        if (pendingSkip) {
          pendingSkip = false;
          pendingRename = null;
          continue;
        }
        const wire = pendingRename ?? (renameAll === "camelCase" ? camel(rawName) : rawName);
        fields.push(wire);
        pendingRename = null;
      }
      if (fields.length > 0) structs[name] = fields.sort();
    }
  }
  return structs;
}

const rust = parseRustStructs(rustFiles(join(root, "src-tauri/src")));
const rustProblems = [];
for (const [structName, rustFields] of Object.entries(rust)) {
  const bindingFields = sortedTypes[structName];
  if (!bindingFields) continue; // struct interno o no expuesto al frontend
  const missing = bindingFields.filter((field) => !rustFields.includes(field));
  const extra = rustFields.filter((field) => !bindingFields.includes(field));
  if (missing.length > 0 || extra.length > 0) {
    rustProblems.push(
      `${structName}: bindings.ts y Rust no coinciden.` +
        (missing.length ? `\n      falta en Rust: ${missing.join(", ")}` : "") +
        (extra.length ? `\n      sobra en Rust: ${extra.join(", ")}` : ""),
    );
  }
}

// ---------- 3. Comparar con el JSON versionado ----------

let onDisk = null;
try {
  onDisk = readFileSync(shapesPath, "utf-8");
} catch {
  onDisk = null;
}
const rendered = `${JSON.stringify(generated, null, 1)}\n`;
const stale = onDisk !== rendered;

const problems = [...rustProblems];
if (stale) {
  problems.push(
    "e2e/ipc-shapes.json está desactualizado respecto a src/bindings.ts.\n" +
      "      Regenera con `npm run gen:ipc-shapes`.",
  );
}

if (write) {
  writeFileSync(shapesPath, rendered);
  console.log(
    `Formas IPC escritas: ${relative(root, shapesPath)} ` +
      `(${Object.keys(sortedCommands).length} comandos, ${Object.keys(sortedTypes).length} tipos).`,
  );
  if (rustProblems.length > 0) {
    console.error("\nAvisos de Rust (revisa antes de commitear):\n" + rustProblems.join("\n"));
  }
  process.exit(0);
}

if (problems.length > 0) {
  console.error("Forma de los comandos IPC desincronizada.\n");
  console.error(problems.map((problem) => `  - ${problem}`).join("\n\n"));
  process.exit(1);
}

console.log(
  `Formas IPC OK: ${Object.keys(sortedCommands).length} comandos, ` +
    `${Object.keys(sortedTypes).length} tipos, ${Object.keys(rust).length} structs de Rust.`,
);
