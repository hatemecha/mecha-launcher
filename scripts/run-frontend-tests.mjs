import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";
import { createRequire } from "node:module";
import ts from "typescript";

const workspaceRequire = createRequire(import.meta.url);
const moduleCache = new Map();

function resolveModulePath(specifier, parentFile) {
  const parentDir = path.dirname(parentFile);
  const candidates = [
    path.resolve(parentDir, specifier),
    path.resolve(parentDir, `${specifier}.ts`),
    path.resolve(parentDir, `${specifier}.js`)
  ];

  for (const candidate of candidates) {
    if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
      return candidate;
    }
  }

  throw new Error(`Unable to resolve ${specifier} from ${parentFile}`);
}

function loadTsModule(modulePath) {
  const absolutePath = path.resolve(modulePath);
  if (moduleCache.has(absolutePath)) {
    return moduleCache.get(absolutePath).exports;
  }

  const source = fs.readFileSync(absolutePath, "utf8");
  const compiled = ts.transpileModule(source, {
    fileName: absolutePath,
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
      moduleResolution: ts.ModuleResolutionKind.NodeJs,
      esModuleInterop: true
    }
  });

  const module = { exports: {} };
  moduleCache.set(absolutePath, module);

  const localRequire = (specifier) => {
    if (specifier.startsWith(".")) {
      const resolved = resolveModulePath(specifier, absolutePath);
      if (resolved.endsWith(".ts")) {
        return loadTsModule(resolved);
      }
      return workspaceRequire(resolved);
    }

    return workspaceRequire(specifier);
  };

  const wrapper = vm.runInThisContext(
    `(function (exports, require, module, __filename, __dirname) { ${compiled.outputText}\n})`,
    { filename: absolutePath }
  );

  wrapper(module.exports, localRequire, module, absolutePath, path.dirname(absolutePath));
  return module.exports;
}

const {
  buildCatalogItems,
  filterCatalogItems,
  findCatalogItemForInstalledVersion,
  recommendedJavaMajorForMinecraft
} = loadTsModule("./src/lib/catalog.ts");
const {
  formatLauncherStatusMessage,
  formatOptifineInstallMessage,
  formatVanillaInstallMessage
} = loadTsModule("./src/lib/launcher-messages.ts");

let failures = 0;

function runTest(name, testFn) {
  try {
    testFn();
    console.log(`PASS ${name}`);
  } catch (error) {
    failures += 1;
    console.error(`FAIL ${name}`);
    console.error(error);
  }
}

runTest("recommendedJavaMajorForMinecraft maps supported release ranges", () => {
  assert.equal(recommendedJavaMajorForMinecraft("1.16.5"), 8);
  assert.equal(recommendedJavaMajorForMinecraft("1.17.1"), 16);
  assert.equal(recommendedJavaMajorForMinecraft("1.20.4"), 17);
  assert.equal(recommendedJavaMajorForMinecraft("1.21.1"), 21);
  assert.equal(recommendedJavaMajorForMinecraft("snapshot"), null);
});

runTest("buildCatalogItems keeps unknown local installs as local entries", () => {
  const installedVersions = [
    {
      id: "1.20.4-custom-pack",
      folderName: "1.20.4-custom-pack",
      jarPath: "versions/1.20.4-custom-pack/1.20.4-custom-pack.jar",
      manifestPath: "versions/1.20.4-custom-pack/1.20.4-custom-pack.json",
      javaMajorVersion: 17
    }
  ];
  const releases = [{ id: "1.20.4" }];
  const optifineOptions = [];

  const items = buildCatalogItems(installedVersions, releases, optifineOptions);
  const localItem = items.find((item) => item.id === "1.20.4-custom-pack");

  assert.ok(localItem);
  assert.equal(localItem.kind, "local");
  assert.equal(localItem.installed, true);
  assert.equal(localItem.requiredJavaMajor, 17);
});

runTest("filterCatalogItems ranks popular items by score before tie-breakers", () => {
  const items = [
    {
      key: "vanilla:1.20.4",
      id: "1.20.4",
      kind: "vanilla",
      title: "Minecraft 1.20.4",
      subtitle: "Vanilla",
      installed: false
    },
    {
      key: "optifine:1.20.4-hd-u-i7",
      id: "1.20.4-hd-u-i7",
      kind: "optifine",
      title: "Minecraft 1.20.4",
      subtitle: "OptiFine",
      installed: true,
      installedVersionId: "1.20.4-OptiFine_HD_U_I7"
    }
  ];

  const filtered = filterCatalogItems(items, "popular", new Set(), {
    "vanilla:1.20.4": 1,
    "optifine:1.20.4-hd-u-i7": 8
  });

  assert.deepEqual(
    filtered.map((item) => item.key),
    ["optifine:1.20.4-hd-u-i7", "vanilla:1.20.4"]
  );
});

runTest("findCatalogItemForInstalledVersion returns the matching installed catalog item", () => {
  const items = [
    {
      key: "vanilla:1.20.4",
      id: "1.20.4",
      kind: "vanilla",
      title: "Minecraft 1.20.4",
      subtitle: "Vanilla",
      installed: true,
      installedVersionId: "1.20.4"
    },
    {
      key: "local:beta-1.7.3",
      id: "beta-1.7.3",
      kind: "local",
      title: "Minecraft beta-1.7.3",
      subtitle: "Local",
      installed: true,
      installedVersionId: "beta-1.7.3"
    }
  ];

  const selectedItem = findCatalogItemForInstalledVersion(items, "beta-1.7.3");

  assert.equal(selectedItem?.key, "local:beta-1.7.3");
});

runTest("formatLauncherStatusMessage localizes launcher states", () => {
  assert.equal(
    formatLauncherStatusMessage(
      "es",
      { launchId: "launch-1", state: "launching", message: null },
      "1.20.4"
    ),
    "Preparando el lanzamiento de 1.20.4."
  );

  assert.equal(
    formatLauncherStatusMessage(
      "en",
      { launchId: "launch-1", state: "running", message: null },
      "1.20.4"
    ),
    "1.20.4 is running."
  );
});

runTest("formatOptifineInstallMessage maps known stages and preserves progress", () => {
  assert.equal(
    formatOptifineInstallMessage("en", {
      optionId: "1.20.4-HD_U_I7",
      stage: "libraries",
      message: "backend message",
      current: 3,
      total: 10
    }),
    "Downloading libraries 3/10"
  );

  assert.equal(
    formatOptifineInstallMessage("es", {
      optionId: "1.20.4-HD_U_I7",
      stage: "done",
      message: "backend message",
      current: 1,
      total: 1
    }),
    "1.20.4-HD_U_I7 quedó listo para jugar."
  );
});

runTest("formatVanillaInstallMessage falls back to backend messages for unknown stages", () => {
  assert.equal(
    formatVanillaInstallMessage("en", {
      versionId: "1.20.4",
      stage: "custom-stage",
      message: "Custom backend detail",
      current: null,
      total: null
    }),
    "Custom backend detail"
  );
});

if (failures > 0) {
  process.exitCode = 1;
} else {
  console.log("All frontend checks passed.");
}
