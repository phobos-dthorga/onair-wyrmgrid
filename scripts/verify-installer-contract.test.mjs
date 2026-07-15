import assert from "node:assert/strict";
import test from "node:test";

import {
  installerIdentity,
  verifyInstallerConfiguration,
} from "./verify-installer-contract.mjs";

function validConfiguration() {
  return {
    productName: installerIdentity.productName,
    identifier: installerIdentity.identifier,
    bundle: {
      targets: ["nsis"],
      windows: {
        allowDowngrades: false,
        nsis: { installMode: installerIdentity.installMode },
      },
    },
  };
}

test("accepts the stable per-user NSIS upgrade identity", () => {
  const configuration = validConfiguration();
  assert.equal(verifyInstallerConfiguration(configuration), configuration);
});

test("rejects identity drift that would orphan an existing installation", () => {
  const configuration = validConfiguration();
  configuration.identifier = "io.github.example.replacement";

  assert.throws(
    () => verifyInstallerConfiguration(configuration),
    /identifier must remain 'io\.github\.phobosdthorga\.onairwyrmgrid'/,
  );
});

test("rejects downgrade permission or a changed installation scope", () => {
  const configuration = validConfiguration();
  configuration.bundle.windows.allowDowngrades = true;
  configuration.bundle.windows.nsis.installMode = "perMachine";

  assert.throws(
    () => verifyInstallerConfiguration(configuration),
    /allowDowngrades must be false[\s\S]*installMode must remain 'currentUser'/,
  );
});
