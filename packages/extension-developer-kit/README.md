# WyrmGrid Extension Developer Kit v1

EDK v1 creates, validates, packages, and locally tests independently
installable WyrmGrid extensions without a WyrmGrid source checkout.

## Install

Until the package has a reviewed public npm release, build its tarball from a
WyrmGrid checkout:

```powershell
npm pack ./packages/extension-developer-kit
npm install --global ./wyrmgrid-extension-developer-kit-1.0.0.tgz
```

After publication, the intended command is:

```powershell
npm install --global @wyrmgrid/extension-developer-kit
```

WyrmGrid desktop distributions also carry the same unpacked, installable npm
package. Open **Forge → Open developer kit**, then install that directory with
`npm install --global "<opened directory>"`. WyrmGrid does not silently install
the command or a Node.js runtime.

Node.js 22.12 or newer is required.

The EDK JavaScript package is platform-neutral and is bundled identically on
Windows, Linux, and macOS. Scaffolding, validation, packaging, schema export,
and archive inspection use the same implementation on each platform. Runtime
conformance can naturally launch only an extension built for the current
supported host; native provider artifacts remain platform-specific.

The package also carries WyrmGrid's zero-dependency Python plugin SDK. Creating
an ordinary plugin copies the compatible SDK into
`src/wyrmgrid_sdk/__init__.py`, so the generated project can be packaged and
shared without referring back to a WyrmGrid checkout.

## First extension

```powershell
wyrmgrid-extension new `
  --kind plugin `
  --directory ./my-first-plugin `
  --id org.example.my-first-plugin `
  --name "My first plugin" `
  --author "Example Author"

cd ./my-first-plugin
wyrmgrid-extension validate --source .
wyrmgrid-extension test --source . --skip-runtime
wyrmgrid-extension package --source . --output ./dist/my-first-plugin.wyrmplugin
```

Remove `--skip-runtime` once the SDK or native executable required by the
scaffold is installed and implemented. Runtime testing launches the extension
with a privacy-reduced environment, performs the versioned startup and
shutdown handshake, applies framing and timeout bounds, and never grants OnAir
credentials, simulator mutation, audio permission, storage paths, or network
authority.

## Commands

- `new` creates a no-overwrite starting tree for `plugin`,
  `simulator-provider`, `audio-provider`, or `audio-codec`.
- `validate` checks a source manifest or completed package against EDK v1.
- `package` creates a deterministic exact-inventory WyrmGrid artifact.
- `test` validates, packages twice, proves byte reproducibility, inspects the
  archive, and optionally exercises runtime startup and shutdown.
- `schemas` lists or copies the exact public schemas shipped with EDK v1.

Scaffolds include `.wyrmignore`. Each non-comment line is an exact
project-relative file or directory path; directories end in `/`. Globs,
negation, absolute paths, and traversal are rejected. The packager always
excludes its current output, `.wyrmignore`, and common version-control metadata.
Review the resulting exact inventory because ignore rules are not a secret
scanner.

Pass `--report FILE` to `validate`, `package`, or `test` for a bounded JSON
compatibility report. Reports contain extension identity, compatibility
versions, stable check results, issue codes, and package SHA-256 only. They do
not include environment variables, credentials, process output, absolute
source paths, or extension payloads.

Native packages remain unsigned in package format v1. Passing EDK checks proves
contract compatibility and local byte integrity—not publisher identity,
rights, safety, audio quality, simulator correctness, or live support.
