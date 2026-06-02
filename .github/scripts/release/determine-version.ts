/**
 * Determine the next version based on Conventional Commits since the last tag.
 *
 * Usage:
 *   deno run --allow-run determine-version.ts
 *
 * Output (JSON to stdout):
 *   { "skip": true } if no releasable commits
 *   { "skip": false, "version": "0.2.0-alpha.20260603", "tag": "v0.2.0-alpha.20260603", "bump": "minor" }
 */

type Bump = "major" | "minor" | "patch" | "none";

async function run(cmd: string[]): Promise<string> {
  const proc = new Deno.Command(cmd[0], {
    args: cmd.slice(1),
    stdout: "piped",
    stderr: "piped",
  });
  const { stdout } = await proc.output();
  return new TextDecoder().decode(stdout).trim();
}

async function getLatestTag(): Promise<string | null> {
  try {
    return await run(["git", "describe", "--tags", "--abbrev=0"]);
  } catch {
    return null;
  }
}

async function getCommitMessages(range: string): Promise<string[]> {
  const output = await run(["git", "log", "--pretty=format:%s", range]);
  return output ? output.split("\n") : [];
}

function determineBump(messages: string[]): Bump {
  let bump: Bump = "none";

  for (const msg of messages) {
    if (/^[a-z]+!:/.test(msg) || /BREAKING CHANGE/i.test(msg)) {
      return "major";
    }
    if (/^feat(\(.+\))?:/.test(msg) && bump !== "major") {
      bump = "minor";
    }
    if (/^fix(\(.+\))?:/.test(msg) && bump === "none") {
      bump = "patch";
    }
  }

  return bump;
}

function bumpVersion(
  current: string,
  bump: Exclude<Bump, "none">,
): string {
  const [major, minor, patch] = current.split(".").map(Number);
  switch (bump) {
    case "major":
      return `${major + 1}.0.0`;
    case "minor":
      return `${major}.${minor + 1}.0`;
    case "patch":
      return `${major}.${minor}.${patch + 1}`;
  }
}

async function main() {
  const latestTag = await getLatestTag();

  let range: string;
  let currentVersion: string;

  if (!latestTag) {
    range = "HEAD";
    currentVersion = "0.0.0";
  } else {
    range = `${latestTag}..HEAD`;
    currentVersion = latestTag.replace(/^v/, "").replace(/-.*$/, "");
  }

  const messages = await getCommitMessages(range);
  const bump = determineBump(messages);
  const outputFile = Deno.env.get("GITHUB_OUTPUT");

  if (bump === "none") {
    console.log("No releasable commits found, skipping.");
    if (outputFile) {
      Deno.writeTextFileSync(outputFile, "skip=true\n", { append: true });
    }
    return;
  }

  const nextVersion = bumpVersion(currentVersion, bump);
  const date = new Date().toISOString().slice(0, 10).replace(/-/g, "");
  const preVersion = `${nextVersion}-alpha.${date}`;
  const tag = `v${preVersion}`;

  console.log(`Next version: ${preVersion} (bump: ${bump})`);

  if (outputFile) {
    const outputs = [
      "skip=false",
      `version=${preVersion}`,
      `tag=${tag}`,
    ].join("\n") + "\n";
    Deno.writeTextFileSync(outputFile, outputs, { append: true });
  }
}

main();
