# Authoring external WyrmGrid extensions

WyrmGrid extensions are files that can be installed, replaced, disabled,
updated, rolled back, and removed without rebuilding WyrmGrid. First-party
extensions use the same package and lifecycle as community extensions.

## Choose the boundary

| Artifact        | Function                                                    | Manifest              | Process boundary             |
| --------------- | ----------------------------------------------------------- | --------------------- | ---------------------------- |
| `.wyrmplugin`   | Consumes approved WyrmGrid facts and publishes bounded data | `plugin.json`         | Supervised Python process    |
| `.wyrmprovider` | Translates a simulator SDK into WyrmGrid Bridge telemetry   | `provider.json`       | Supervised native executable |
| `.wyrmaudio`    | Enumerates approved audio sources and emits bounded PCM     | `audio-provider.json` | Supervised native executable |
| `.wyrmcodec`    | Encodes selected transient PCM into bounded media packets   | `audio-codec.json`    | Supervised native executable |

Use an ordinary plugin unless native simulator, device, or codec integration is
actually required. Native provider packages run as the current operating-system
user; process separation is not an operating-system sandbox.

## Create a starting tree

Node.js 22 or newer is required. From a WyrmGrid source checkout:

```powershell
npm run extension:scaffold -- `
  --kind audio-codec `
  --directory ..\my-wyrmgrid-codec `
  --id org.example.my-codec `
  --name "My codec" `
  --author "Example Author"
```

`--kind` accepts `plugin`, `simulator-provider`, `audio-provider`, or
`audio-codec`. `--version` defaults to `0.1.0`. The command validates the
reverse-domain ID and semantic version and refuses to overwrite a non-empty
directory. It creates the correct manifest, a deny-by-default starting
capability set, packaging instructions, and ignore rules. Native scaffolds do
not invent a working executable: implement and test the applicable public
protocol before packaging.

## Package contents

Every format is a deterministic ZIP-compatible envelope with a reserved
`wyrmgrid-package.json`. That envelope binds:

- package kind, extension ID, extension version, and manifest path;
- an exact case-sensitive payload inventory;
- byte length and SHA-256 digest for every payload file; and
- bounded file count, paths, individual files, expanded contents, and archive.

The package must contain the manifest and every declared entry point. It must
not contain symlinks, traversal paths, encrypted ZIP entries, undeclared
content, duplicate or case-colliding paths, Windows device names, credentials,
personal data, or development secrets.

The scaffold README supplies the matching command. The four packagers are also
available directly:

```powershell
npm run plugin:package -- --source <directory> --output <file.wyrmplugin>
npm run provider:package -- --source <directory> --output <file.wyrmprovider>
npm run audio-provider:package -- --source <directory> --output <file.wyrmaudio>
npm run audio-codec:package -- --source <directory> --output <file.wyrmcodec>
```

Use `--include SOURCE=PACKAGE_PATH` to add a built executable outside the
manifest directory. On Windows, audio-provider and audio-codec manifests name
an extensionless entry-point stem while the package contains the matching
`.exe`. A simulator-provider manifest names its packaged executable exactly.
The packager refuses an existing output unless `--force` is deliberately used.

## Identity and compatibility

- Choose one stable reverse-domain ID and do not transfer it between unrelated
  projects.
- Use exact `X.Y.Z` versions. Never publish different bytes under an existing
  ID and version.
- Increment the package version when published contents change.
- Treat the package schema, extension protocol, application version, and
  database schema as separate compatibility axes.
- Declare only platforms, simulators, profiles, permissions, capabilities, and
  network origins that the packaged version implements.
- Keep all provider-specific raw data inside the provider. Publish only the
  stable WyrmGrid contract.

WyrmGrid rejects malformed or incompatible packages before extraction. An
installed version is immutable. Installing a newer version retains one
rollback version; disabling removes it from runtime discovery; removal uses a
recoverable managed tombstone. Missing or incompatible providers fail closed
without silently selecting a replacement.

## Audio codec requirements

An Audio Codec Provider receives raw signed 16-bit little-endian PCM only for
an explicitly selected source while recording. It receives no device label,
OnAir key, database key, media key, storage path, export destination, general
plugin capability, or optional-AI channel.

Protocol version 1 requires the bounded hello/manifest/ready handshake, explicit
track start and stop, ordered frame sequences, and exactly one matching encoded
packet per accepted PCM frame. Profile IDs are stable WyrmGrid roles; the
manifest binds them to the codec's media type, channels, sample rate, target
bitrate, and packet duration. Start with
[Audio Codec Provider protocol version 1](audio-codec-provider-protocol.md) and
use the first-party `codecs/opus` process as a concrete reference, not as an ABI
dependency.

## Test before sharing

At minimum:

1. Validate the manifest against its checked-in schema and test malformed,
   oversized, unsupported-version, wrong-identity, timeout, and shutdown cases.
2. Build and package each declared target from a clean checkout.
3. Inspect the package in WyrmGrid before accepting its native-code warning.
4. Exercise install, launch, disable, update, rollback, removal, application
   restart, and absent-dependency behaviour in a disposable profile.
5. Confirm a process crash cannot take down WyrmGrid or silently substitute a
   provider, source, codec, or fact.
6. Keep synthetic fixtures free of credentials, raw provider payloads, voices,
   device labels, personal data, and machine-specific paths.

The repository's deterministic fake audio package and first-party Opus package
also complete one synthetic packaged capture-to-codec-to-encrypted-playback
test on Windows. That proves the package and orchestration boundaries, not
live-device support, codec conformance, quality, publisher trust, or safety.

## Publishing status

Local package installation is implemented and does not depend on the future
WyrmGrid Aerie catalogue. Packages and publishers are not yet signed or
authenticated. Inspection establishes structural integrity and the exact bytes
accepted locally; it does not establish authorship, rights, intent, quality, or
safety. Do not recommend unreviewed executable packages until the remaining
signing, revocation, resource-limit, sandbox, moderation, legal, and
live-certification gates are complete.
