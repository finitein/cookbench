import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";

const hostLine = execFileSync("rustc", ["-vV"], { encoding: "utf8" })
  .split("\n")
  .find((line) => line.startsWith("host: "));
const target = process.env.COOKBENCH_TARGET
  ?? process.env.TAURI_ENV_TARGET_TRIPLE
  ?? hostLine?.slice("host: ".length);

if (!target || !/^[A-Za-z0-9_.-]+$/.test(target)) {
  throw new Error("Cookbench could not determine a safe Rust target triple");
}

execFileSync("cargo", [
  "build",
  "--locked",
  "--release",
  "--target",
  target,
  "-p",
  "cookbench-bridge",
  "-p",
  "cookbench-hook",
], { stdio: "inherit" });

const extension = target.includes("windows") ? ".exe" : "";
const outputDirectory = join("src-tauri", "binaries");
mkdirSync(outputDirectory, { recursive: true });

for (const name of ["cookbench-bridge", "cookbench-hook"]) {
  const source = join("target", target, "release", `${name}${extension}`);
  const destination = join(outputDirectory, `${name}-${target}${extension}`);
  copyFileSync(source, destination);
  console.log(`prepared ${destination}`);
}
