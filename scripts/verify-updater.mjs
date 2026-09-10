/**
 * Dry-run del auto-updater, de extremo a extremo, contra el feed publicado.
 *
 * Réplica de lo que hace @tauri-apps/plugin-updater en una instalación real:
 *   1. fetch del endpoint configurado en tauri.conf.json (anónimo)
 *   2. comparación de versión contra la del repositorio (package.json)
 *   3. texto que renderizaría el popup (UpdateDialog muestra `notes` tal cual)
 *   4. descarga del instalador anunciado por el feed
 *   5. verificación minisign con la pubkey embebida en tauri.conf.json —
 *      el algoritmo exacto de minisign-verify (la crate de Tauri):
 *        · firma del archivo = Ed25519 sobre blake2b-512(instalador) (modo "ED")
 *        · firma global      = Ed25519 sobre (sig_archivo || trusted_comment
 *          SIN el prefijo "trusted comment: " y sin salto de línea)
 *
 * Uso:  npm run verify-updater
 * Sale 0 si la cadena completa funciona (feed coherente + firma válida) y
 * 1 en cualquier fallo. Pensado para correrlo antes de publicar el draft.
 *
 * Sin secretos: la pubkey sale de src-tauri/tauri.conf.json, igual que en
 * producción. Compara el feed contra la versión de package.json, de modo
 * que en una versión ya publicada el paso 2 concluye "sin actualización"
 * (y aun así valida feed, descarga y firma).
 */
import { readFileSync } from "node:fs";
import crypto from "node:crypto";

const conf = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const pkg = JSON.parse(readFileSync("package.json", "utf8"));
const endpoint = conf.plugins.updater.endpoints[0];
const pubkeyB64 = conf.plugins.updater.pubkey;
const REPO_VERSION = pkg.version; // la versión que "tendría" esta build

let failures = 0;
const step = (n, t) => console.log(`\n== Paso ${n}: ${t}`);
const check = (label, ok, detail = "") => {
  console.log(`   ${ok ? "✅" : "❌"} ${label}${detail ? ` — ${detail}` : ""}`);
  if (!ok) failures += 1;
};

// 1. fetch del feed (anónimo, como el plugin)
step(1, `fetch ${endpoint}`);
let feed;
try {
  const res = await fetch(endpoint);
  check(`HTTP ${res.status}`, res.ok);
  feed = JSON.parse(await res.text());
  console.log(`   version=${feed.version} · notes=${feed.notes?.length ?? 0} chars · pub_date=${feed.pub_date}`);
} catch (e) {
  check("feed accesible", false, String(e));
  process.exit(1);
}

// 2. comparación semver contra la versión del repo (mayor.menor.parche)
step(2, `¿${feed.version} > ${REPO_VERSION}? (versión de package.json)`);
const cmp = (a, b) => {
  const [x, y] = [a, b].map((v) => v.split(".").map(Number));
  for (let i = 0; i < 3; i++) if ((x[i] ?? 0) !== (y[i] ?? 0)) return Math.sign((x[i] ?? 0) - (y[i] ?? 0));
  return 0;
};
const updateAvailable = cmp(feed.version, REPO_VERSION) > 0;
const same = feed.version === REPO_VERSION;
console.log(
  updateAvailable
    ? "   → hay actualización: el plugin devolvería Update y abriría el diálogo"
    : same
      ? "   → misma versión: el plugin devolvería null (silencio); se valida el resto igual"
      : "   → la versión del feed es MENOR que la del repo: ¡el feed está desactualizado!",
);
if (cmp(feed.version, REPO_VERSION) < 0) failures += 1;

// 3. lo que renderiza UpdateDialog.tsx (body = notes, textual, whitespace-pre-wrap)
step(3, "popup que vería el usuario (UpdateDialog)");
console.log("   ┌─ Nueva versión disponible");
console.log(`   │  ISALAB v${feed.version} ya está lista para instalar.`);
console.log("   │  ┌ body (= notes del feed) ─────────────");
for (const line of feed.notes.split("\n")) console.log(`   │  │ ${line}`);
console.log("   │  └──────────────────────────────────────");
console.log("   └─ [Más tarde] [Actualizar ahora]");
check("notes no vacías", typeof feed.notes === "string" && feed.notes.length > 0);

// 4. descarga del instalador anunciado
const platform = feed.platforms["windows-x86_64"];
if (!platform) {
  check("plataforma windows-x86_64 en el feed", false);
  process.exit(1);
}
const { signature, url } = platform;
step(4, `download ${url}`);
let installer;
try {
  const dl = await fetch(url);
  check(`HTTP ${dl.status}`, dl.ok);
  installer = Buffer.from(await dl.arrayBuffer());
  console.log(`   ${(installer.length / 1024 / 1024).toFixed(1)} MB`);
} catch (e) {
  check("instalador descargable", false, String(e));
  process.exit(1);
}

// 5. verificación minisign — algoritmo EXACTO de minisign-verify
step(5, "verificar firma minisign con la pubkey de tauri.conf.json");
const NL = String.fromCharCode(10);

// PublicKey::from_base64: la config guarda base64 del archivo .pub
// (comentario + base64 del blob: algo(2) + keyid(8) + clave Ed25519(32))
const pubText = Buffer.from(pubkeyB64, "base64").toString("utf8");
check("pubkey con formato minisign", pubText.startsWith("untrusted comment:"));
const pubRaw = Buffer.from(pubText.split(NL)[1], "base64");
const pubKeyid = pubRaw.subarray(2, 10);
const pubKey = pubRaw.subarray(10, 42);
// SPKI DER para Ed25519: prefijo fijo + 32 bytes de clave
const keyObject = crypto.createPublicKey({
  key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), pubKey]),
  format: "der",
  type: "spki",
});

// Signature::decode: línea 2 = base64( algo(2) + keyid(8) + sig(64) ) = 74 bytes,
// línea 3 = trusted comment, línea 4 = base64( sig_global(64) )
const sigText = Buffer.from(signature, "base64").toString("utf8").replace(/\r\n/g, NL);
const lines = sigText.split(NL);
const bundle = Buffer.from(lines[1], "base64");
const sigAlgo = bundle.subarray(0, 2).toString(); // "ED" = prehased (blake2b-512)
const sigKeyid = bundle.subarray(2, 10);
const fileSig = bundle.subarray(10, 74);
const globalSig = Buffer.from(lines[3], "base64");
const trustedCommentFull = lines[2];
const trustedComment = trustedCommentFull.slice("trusted comment: ".length);

check("keyid de la firma == keyid de la pubkey", pubKeyid.equals(sigKeyid));
check("modo prehased (ED)", sigAlgo === "ED", `algo=${sigAlgo}`);

const hashed = crypto.createHash("blake2b512").update(installer).digest();
const okFile = crypto.verify(null, hashed, keyObject, fileSig);
const globalMsg = Buffer.concat([fileSig, Buffer.from(trustedComment, "utf8")]);
const okGlobal = crypto.verify(null, globalMsg, keyObject, globalSig);
check("firma del instalador", okFile, "Ed25519 sobre blake2b-512(instalador)");
check("firma global", okGlobal, "Ed25519 sobre sig_archivo || trusted_comment");
console.log(`   ${trustedCommentFull}`);

const feedTargetsThisInstaller = trustedCommentFull.includes(
  `file:ISALAB_${feed.version}_x64-setup.exe`,
);
check("el trusted comment nombra el instalador de la versión del feed", feedTargetsThisInstaller);

console.log(
  `\n${failures === 0 ? "✅ VERIFICACIÓN COMPLETA: feed coherente y firma válida" : `❌ ${failures} chequeo(s) fallaron`}`,
);
process.exit(failures === 0 ? 0 : 1);
