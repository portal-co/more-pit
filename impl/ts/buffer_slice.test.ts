import { execSync } from "node:child_process";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { sliceBuffer } from "../impl/ts/buffer_slice.js";

const repoRoot = join(fileURLToPath(new URL(".", import.meta.url)), "..");
const bufferPit = join(repoRoot, "pit/common/buffer.pit");
const rid = execSync(`cargo run -q -p pit-rid -- "${bufferPit}"`, {
  cwd: repoRoot,
  encoding: "utf8",
}).trim();

type BufferApi = Record<string, (...args: number[]) => unknown>;

describe("buffer_slice impl", () => {
  it("read write size against generated P type", () => {
    const data = new Uint8Array([42, 1, 2, 3]);
    const buf = sliceBuffer(data) as BufferApi;
    const read8 = buf[`P${rid}_read8`];
    const write8 = buf[`P${rid}_write8`];
    const size = buf[`P${rid}_size`];
    expect(read8(0)).toEqual([42]);
    write8(1, 9);
    expect(data[1]).toBe(9);
    expect(size()).toEqual([4]);
  });
});
