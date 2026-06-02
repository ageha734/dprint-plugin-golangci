const VERSION = Deno.readTextFileSync("Cargo.toml")
    .match(/^version = "(.+)"$/m)?.[1] ?? "0.0.0";

const PLUGIN_NAME = "dprint-plugin-golangci";
const REPO = "ageha734/dprint-plugin-golangci";

const PLATFORMS: Record<string, string> = {
    "darwin-x86_64": "x86_64-apple-darwin",
    "darwin-aarch64": "aarch64-apple-darwin",
    "linux-x86_64": "x86_64-unknown-linux-gnu",
    "linux-aarch64": "aarch64-unknown-linux-gnu",
    "windows-x86_64": "x86_64-pc-windows-msvc",
};

interface PlatformEntry {
    reference: string;
    checksum: string;
}

interface PluginJson {
    schemaVersion: number;
    kind: string;
    name: string;
    version: string;
    [platform: string]: unknown;
}

async function sha256(filePath: string): Promise<string> {
    const data = await Deno.readFile(filePath);
    const hash = await crypto.subtle.digest("SHA-256", data);
    return Array.from(new Uint8Array(hash))
        .map((b) => b.toString(16).padStart(2, "0"))
        .join("");
}

async function main() {
    const pluginJson: PluginJson = {
        schemaVersion: 2,
        kind: "process",
        name: PLUGIN_NAME,
        version: VERSION,
    };

    for (const [platform, target] of Object.entries(PLATFORMS)) {
        const zipName = `${PLUGIN_NAME}-${target}.zip`;
        const zipPath = `release/${zipName}`;

        try {
            const checksum = await sha256(zipPath);
            const url =
                `https://github.com/${REPO}/releases/download/v${VERSION}/${zipName}`;

            const entry: PlatformEntry = { reference: url, checksum };
            pluginJson[platform] = entry;
        } catch {
            console.error(`Warning: ${zipPath} not found, skipping ${platform}`);
        }
    }

    await Deno.writeTextFile(
        "plugin.json",
        JSON.stringify(pluginJson, null, 2) + "\n",
    );

    console.log(`Generated plugin.json for ${PLUGIN_NAME} v${VERSION}`);
    console.log(JSON.stringify(pluginJson, null, 2));
}

main();
