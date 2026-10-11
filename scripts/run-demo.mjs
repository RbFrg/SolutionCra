// Lance le serveur en mode démo sans dépendre du shell (Windows et Linux).
import { spawnSync } from "node:child_process";

const env = { ...process.env, APP_MODE: "demo" };
delete env.CRA_DB_PATH;
delete env.CRA_SECRET_KEY;

const r = spawnSync("cargo", ["run", "-p", "cra-server"], {
  env,
  stdio: "inherit",
  shell: process.platform === "win32",
});
process.exit(r.status ?? 1);
