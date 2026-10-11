// Hook Claude Code (PostToolUse) : retour du compilateur après chaque édition.
// Sort en code 2 en cas d'échec pour que Claude reçoive l'erreur (7.6).
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, join } from "node:path";

const entree = JSON.parse(readFileSync(0, "utf8") || "{}");
const f = (entree.tool_input?.file_path ?? "").replaceAll("\\", "/");

let cmd = null;
if (f.endsWith(".rs") || f.endsWith("/Cargo.toml")) {
  cmd = ["cargo", ["check", "--workspace", "--quiet", "--message-format", "short"]];
} else if (/\/web\/src\/.*\.tsx?$/.test(f)) {
  cmd = ["npm", ["--prefix", "web", "run", "typecheck", "--silent"]];
}
if (cmd) {
  // rustup installe cargo dans ~/.cargo/bin, absent du PATH d'un processus
  // démarré avant l'installation.
  const env = { ...process.env };
  const cle = Object.keys(env).find((k) => k.toLowerCase() === "path") ?? "PATH";
  env[cle] = [join(homedir(), ".cargo", "bin"), env[cle] ?? ""].join(delimiter);
  const r = spawnSync(cmd[0], cmd[1], {
    encoding: "utf8",
    env,
    shell: process.platform === "win32",
  });
  if (r.status !== 0) {
    console.error((r.stdout ?? "") + (r.stderr ?? ""));
    process.exit(2);
  }
}
