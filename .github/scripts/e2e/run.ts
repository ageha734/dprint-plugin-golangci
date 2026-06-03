/**
 * E2E test runner for dprint-plugin-golangci.
 *
 * Usage:
 *   deno run --allow-run --allow-read --allow-write --allow-env run.ts \
 *     <binary-path> <mode> <version> <scenario>
 *
 * Scenarios:
 *   lint-error   - detect unfixable lint issues (unused function)
 *   fix          - auto-fix gofmt violations via dprint fmt
 *   clean        - no lint errors, dprint check passes
 *   config-path  - custom configPath option works
 */

const COMPAT_MAP: Record<string, string> = {
  "2.5.0": "1.24",
  "2.4.0": "1.24",
  "2.3.0": "1.23",
  "2.2.0": "1.23",
  "2.1.0": "1.23",
  "2.0.0": "1.23",
  "1.64.8": "1.23",
  "1.64.0": "1.23",
  "1.63.4": "1.23",
  "1.62.2": "1.22",
  "1.61.0": "1.22",
  "1.60.3": "1.21",
  "1.59.1": "1.21",
  "1.58.2": "1.21",
  "1.57.2": "1.20",
  "1.56.2": "1.20",
  "1.55.2": "1.20",
};

type Scenario = "lint-error" | "fix" | "clean" | "config-path";

async function run(
  cmd: string[],
  opts?: { cwd?: string },
): Promise<{ code: number; stdout: string; stderr: string }> {
  const proc = new Deno.Command(cmd[0], {
    args: cmd.slice(1),
    cwd: opts?.cwd,
    stdout: "piped",
    stderr: "piped",
  });
  const output = await proc.output();
  return {
    code: output.code,
    stdout: new TextDecoder().decode(output.stdout),
    stderr: new TextDecoder().decode(output.stderr),
  };
}

async function sha256(path: string): Promise<string> {
  const { stdout } = await run(["sha256sum", path]);
  return stdout.split(" ")[0];
}

async function detectGoVersion(): Promise<string | null> {
  try {
    const { stdout } = await run(["go", "version"]);
    const match = stdout.match(/go(\d+\.\d+)/);
    return match ? match[1] : null;
  } catch {
    return null;
  }
}

async function detectGolangciVersion(): Promise<string | null> {
  try {
    const { stdout } = await run(["golangci-lint", "version"]);
    const match = stdout.match(/(\d+\.\d+\.\d+)/);
    return match ? match[1] : null;
  } catch {
    return null;
  }
}

function golangciConfig(
  version: string,
  opts: { linters?: string[]; formatters?: string[] },
): string {
  const major = version.split(".")[0];
  let config = major === "2" ? `version: "2"\n` : "";

  if (opts.linters && opts.linters.length > 0) {
    const list = opts.linters.map((l) => `    - ${l}`).join("\n");
    config += `linters:\n  enable:\n${list}\n`;
  }

  if (opts.formatters && opts.formatters.length > 0) {
    const list = opts.formatters.map((f) => `    - ${f}`).join("\n");
    if (major === "2") {
      config += `formatters:\n  enable:\n${list}\n`;
    } else {
      config += `linters:\n  enable:\n${list}\n`;
    }
  }

  return config;
}

function setupTestProject(
  dir: string,
  goVersion: string,
  golangciVersion: string,
  scenario: Scenario,
): void {
  Deno.mkdirSync(dir, { recursive: true });

  Deno.writeTextFileSync(`${dir}/go.mod`, `module e2e\n\ngo ${goVersion}\n`);

  switch (scenario) {
    case "lint-error":
      Deno.writeTextFileSync(
        `${dir}/main.go`,
        `package main\n\nimport "fmt"\n\nfunc unused() {}\n\nfunc main() {\n\tfmt.Println("hello")\n}\n`,
      );
      Deno.writeTextFileSync(
        `${dir}/.golangci.yml`,
        golangciConfig(golangciVersion, { linters: ["unused"] }),
      );
      break;

    case "fix":
      // gofmt violation: spaces instead of tabs
      Deno.writeTextFileSync(
        `${dir}/main.go`,
        `package main\n\nimport "fmt"\n\nfunc main() {\n  fmt.Println("hello")\n}\n`,
      );
      Deno.writeTextFileSync(
        `${dir}/.golangci.yml`,
        golangciConfig(golangciVersion, { formatters: ["gofmt"] }),
      );
      break;

    case "clean":
      Deno.writeTextFileSync(
        `${dir}/main.go`,
        `package main\n\nimport "fmt"\n\nfunc main() {\n\tfmt.Println("hello")\n}\n`,
      );
      Deno.writeTextFileSync(
        `${dir}/.golangci.yml`,
        golangciConfig(golangciVersion, { linters: ["unused"] }),
      );
      break;

    case "config-path":
      Deno.writeTextFileSync(
        `${dir}/main.go`,
        `package main\n\nimport "fmt"\n\nfunc unused() {}\n\nfunc main() {\n\tfmt.Println("hello")\n}\n`,
      );
      Deno.mkdirSync(`${dir}/config`, { recursive: true });
      Deno.writeTextFileSync(
        `${dir}/config/lint.yml`,
        golangciConfig(golangciVersion, { linters: ["unused"] }),
      );
      break;
  }
}

async function setupDprintConfig(
  testDir: string,
  binaryPath: string,
  dprintConfigVersion: string,
  scenario: Scenario,
): Promise<void> {
  await run(["dprint", "clear-cache"]).catch(() => {});
  await run(["zip", "-j", "plugin-binary.zip", binaryPath], { cwd: testDir });

  const zipChecksum = await sha256(`${testDir}/plugin-binary.zip`);

  const pluginJson = JSON.stringify({
    schemaVersion: 2,
    kind: "process",
    name: "dprint-plugin-golangci",
    version: "0.1.0",
    "linux-x86_64": { reference: `${testDir}/plugin-binary.zip`, checksum: zipChecksum },
    "linux-aarch64": { reference: `${testDir}/plugin-binary.zip`, checksum: zipChecksum },
    "darwin-x86_64": { reference: `${testDir}/plugin-binary.zip`, checksum: zipChecksum },
    "darwin-aarch64": { reference: `${testDir}/plugin-binary.zip`, checksum: zipChecksum },
  }, null, 2);
  Deno.writeTextFileSync(`${testDir}/plugin.json`, pluginJson);

  const pluginChecksum = await sha256(`${testDir}/plugin.json`);

  const golangciConfig: Record<string, unknown> = {};
  if (dprintConfigVersion) {
    golangciConfig.version = dprintConfigVersion;
  }
  if (scenario === "config-path") {
    golangciConfig.configPath = "config/lint.yml";
  }
  golangciConfig.fix = scenario === "fix";

  const dprintJson = JSON.stringify({
    golangci: golangciConfig,
    plugins: [`${testDir}/plugin.json@${pluginChecksum}`],
  }, null, 2);
  Deno.writeTextFileSync(`${testDir}/dprint.json`, dprintJson);
}

async function runScenario(
  testDir: string,
  mode: string,
  scenario: Scenario,
  goVersion: string,
): Promise<boolean> {
  switch (scenario) {
    case "lint-error": {
      const { stdout, stderr } = await run(
        ["dprint", "check", "--", "main.go"],
        { cwd: testDir },
      );
      const output = stdout + stderr;
      console.log(output);
      if (output.includes("unused")) {
        console.log(`✓ ${mode}/${scenario}: passed (go ${goVersion})`);
        return true;
      }
      console.log(`✗ ${mode}/${scenario}: expected 'unused' lint error not found`);
      return false;
    }

    case "fix": {
      const original = Deno.readTextFileSync(`${testDir}/main.go`);
      await run(["dprint", "fmt", "--", "main.go"], { cwd: testDir });
      const fixed = Deno.readTextFileSync(`${testDir}/main.go`);
      if (fixed !== original && fixed.includes("\t")) {
        console.log(`✓ ${mode}/${scenario}: file was fixed (go ${goVersion})`);
        return true;
      }
      console.log(`✗ ${mode}/${scenario}: file was not fixed`);
      return false;
    }

    case "clean": {
      const result = await run(
        ["dprint", "check", "--", "main.go"],
        { cwd: testDir },
      );
      if (result.code === 0) {
        console.log(`✓ ${mode}/${scenario}: passed (go ${goVersion})`);
        return true;
      }
      console.log(result.stdout + result.stderr);
      console.log(`✗ ${mode}/${scenario}: expected clean pass`);
      return false;
    }

    case "config-path": {
      const { stdout, stderr } = await run(
        ["dprint", "check", "--", "main.go"],
        { cwd: testDir },
      );
      const output = stdout + stderr;
      console.log(output);
      if (output.includes("unused")) {
        console.log(`✓ ${mode}/${scenario}: passed (go ${goVersion})`);
        return true;
      }
      console.log(`✗ ${mode}/${scenario}: custom config not picked up`);
      return false;
    }
  }
}

async function main() {
  const [binaryPath, mode, version, scenario] = Deno.args;

  if (!binaryPath || !mode) {
    console.error(
      "Usage: run.ts <binary-path> <mode> [version] [scenario]",
    );
    Deno.exit(1);
  }

  const resolvedBinary = await Deno.realPath(binaryPath);
  const scenarioName = (scenario || "lint-error") as Scenario;
  const e2eDir = "/tmp/e2e-dprint-golangci";

  let golangciVersion: string;
  let dprintConfigVersion = "";

  if (mode === "installed") {
    const detected = await detectGolangciVersion();
    if (!detected) {
      console.log("=== SKIP (installed mode): golangci-lint not found ===");
      Deno.exit(0);
    }
    golangciVersion = detected;
    console.log(`=== Testing [${scenarioName}] with installed golangci-lint v${golangciVersion} ===`);
  } else if (mode === "auto-install") {
    if (!version) {
      console.error("Error: version is required for auto-install mode");
      Deno.exit(1);
    }
    golangciVersion = version;
    dprintConfigVersion = version;
    console.log(`=== Testing [${scenarioName}] auto-install of golangci-lint v${golangciVersion} ===`);
  } else {
    console.error(`Unknown mode: ${mode}`);
    Deno.exit(1);
  }

  const goVersion = await detectGoVersion();
  if (!goVersion) {
    console.error("✗ Go not found in PATH");
    Deno.exit(1);
  }

  const requiredGo = COMPAT_MAP[golangciVersion] || "1.22";
  const [goMajor, goMinor] = goVersion.split(".").map(Number);
  const [reqMajor, reqMinor] = requiredGo.split(".").map(Number);
  if (goMajor < reqMajor || (goMajor === reqMajor && goMinor < reqMinor)) {
    console.log(`=== SKIP: Go ${goVersion} < required ${requiredGo} ===`);
    Deno.exit(0);
  }

  const testDir = `${e2eDir}/${mode}/${scenarioName}`;
  try {
    Deno.removeSync(testDir, { recursive: true });
  } catch { /* ignore */ }

  setupTestProject(testDir, goVersion, golangciVersion, scenarioName);
  await setupDprintConfig(testDir, resolvedBinary, dprintConfigVersion, scenarioName);

  const passed = await runScenario(testDir, mode, scenarioName, goVersion);
  Deno.exit(passed ? 0 : 1);
}

main();
