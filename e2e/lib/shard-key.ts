/**
 * Prints the input key of one CI shard: a hash over the git blob of every tracked file the shard
 * depends on (`inputsOf`). CI stores a pass under it, and a rerun with the same inputs skips the
 * shard. Runs before packages are installed: it imports only the shard table.
 *
 *   bun e2e/lib/shard-key.ts flows   (run from the repository root)
 */
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";

import { SHARDS, inputsOf, isShard } from "./shards.ts";

const shard = process.argv[2] ?? "";
if (!isShard(shard)) {
  process.stderr.write(`usage: shard-key.ts <${Object.keys(SHARDS).join("|")}>\n`);
  process.exit(2);
}
// `<mode> <blob> <stage>\t<path>`: the blob hash stands for the content, so nothing is read twice.
const entries = new Map(
  execFileSync("git", ["ls-files", "-s"], { encoding: "utf8" })
    .split("\n")
    .filter((line) => line !== "")
    .map((line) => {
      const [meta = "", path = ""] = line.split("\t");
      return [path, meta.split(" ")[1] ?? ""] as const;
    })
);
const hash = createHash("sha256");
for (const path of inputsOf(shard, [...entries.keys()]).sort()) hash.update(`${path} ${entries.get(path) ?? ""}\n`);
process.stdout.write(`${hash.digest("hex").slice(0, 32)}\n`);
