import { describe, expect, test } from "bun:test";

describe("cli", () => {
  test("--help flag shows help", async () => {
    const proc = Bun.spawn(["bun", "run", "src/cli/index.ts", "--help"], {
      cwd: `${import.meta.dir}/..`,
      stdout: "pipe",
    });

    const output = await new Response(proc.stdout).text();
    expect(output).toContain("repo-lint");
    expect(output).toContain("USAGE");
    expect(output).toContain("--help");
  });

  test("--version flag shows version", async () => {
    const proc = Bun.spawn(["bun", "run", "src/cli/index.ts", "--version"], {
      cwd: `${import.meta.dir}/..`,
      stdout: "pipe",
    });

    const output = await new Response(proc.stdout).text();
    expect(output.trim()).toMatch(/^\d+\.\d+\.\d+$/);
  });

  test("check command runs", async () => {
    const proc = Bun.spawn(["bun", "run", "src/cli/index.ts", "check"], {
      cwd: `${import.meta.dir}/..`,
      stdout: "pipe",
      stderr: "pipe",
    });

    await proc.exited;
    const stdout = await new Response(proc.stdout).text();
    const stderr = await new Response(proc.stderr).text();

    expect(stdout.length + stderr.length).toBeGreaterThan(0);
  });

  test("inspect layout shows layout", async () => {
    const proc = Bun.spawn(["bun", "run", "src/cli/index.ts", "inspect", "layout"], {
      cwd: `${import.meta.dir}/..`,
      stdout: "pipe",
    });

    const output = await new Response(proc.stdout).text();
    expect(output).toContain("type");
  });

  test("--json outputs JSON format", async () => {
    const proc = Bun.spawn(["bun", "run", "src/cli/index.ts", "check", "--json"], {
      cwd: `${import.meta.dir}/..`,
      stdout: "pipe",
      stderr: "pipe",
    });

    await proc.exited;
    const stdout = await new Response(proc.stdout).text();

    if (stdout.trim()) {
      const parsed = JSON.parse(stdout) as { violations: unknown[]; summary: unknown };
      expect(parsed).toHaveProperty("violations");
      expect(parsed).toHaveProperty("summary");
    }
  });
});
