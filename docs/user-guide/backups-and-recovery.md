# Encrypted backups and recovery

WyrmGrid encrypts its local database automatically. The database key is created
for this installation and protected by the operating system; you do not need to
choose or remember it. Do not copy `wyrmgrid.db` as a backup: that file remains
bound to the original operating-system credential entry.

Use **Settings → Encrypted data & backups** when you need to move or preserve
WyrmGrid data.

See [Remembered accounts and credentials](accounts-and-credentials.md) for the
separate Windows credential and provider-metadata boundary.

## Create a portable backup

1. Choose **Create portable backup**, then select a new `.wyrmbackup` file.
2. Enter and confirm a dedicated password of at least 12 characters.
3. Store that password in a trusted password manager before relying on the
   backup.
4. Keep the resulting file somewhere appropriate for your own recovery plan.

WyrmGrid never overwrites an existing backup and never stores or recovers the
backup password. A portable backup is encrypted but contains the complete local
WyrmGrid database: Hoard observations, retained simulator recordings, local
authorisation history, legal choices, display preferences, and imported theme
and language manifests. Treat it as sensitive operational history.

The OnAir API key is not included because an optionally remembered key belongs
to the operating-system credential store, outside the database. The encrypted
backup does include the saved OnAir Company ID, automatic-connect choice, and
remembered SimBrief Pilot ID or username. After moving systems or reinstalling
Windows, enter the OnAir API key again before reconnecting. Browser-webview
local storage and command-line launch options are also outside backup format
version 1.

## Restore or move to another system

1. Install and launch the same or a newer compatible WyrmGrid release on the
   destination system.
2. Open **Settings → Encrypted data & backups** and choose the portable backup.
3. Enter its password and confirm that the destination's local database will
   be replaced.
4. Select **Validate and prepare restore**.
5. Restart WyrmGrid to activate the validated data.

The current database remains active until restart. During startup WyrmGrid
retains it as a rollback copy, validates the replacement under the destination
device's new key, and removes the rollback only after the replacement opens
successfully. A wrong password, damaged file, unsupported format, or future
database schema fails without replacing current local data.

This is the supported flow for a new computer, operating-system reinstall, or
recovery after losing the device credential. Create the portable backup before
removing the old installation; WyrmGrid cannot recover an encrypted database
after both its device key and every usable portable backup are lost.

## Install a newer version on this device

1. Close WyrmGrid and stop any running simulator provider.
2. Optionally create a portable backup, especially before installing a
   prerelease.
3. Run the newer WyrmGrid setup. Do not uninstall the existing version first.
4. Let setup replace the application in its existing per-user location.
5. Launch WyrmGrid normally. It reuses the encrypted database in application
   data and the device-local key held by the operating-system credential store.

A normal setup upgrade does not remove application data. Do not delete the
WyrmGrid application-data directory as part of an update. Older installers are
blocked from replacing a newer installed version.

## Where Windows keeps local data

The persistent application-data directory is:

`%APPDATA%\io.github.phobosdthorga.onairwyrmgrid`

This directory is not a cache. It contains the encrypted database and may also
contain temporary pending or rollback databases during a restore. The matching
database key is held separately by Windows Credential Manager, so copying this
directory alone is not a supported backup or migration method.

Developer builds may also use `%LOCALAPPDATA%\WyrmGrid\cargo-target`. That
directory and repository-local `target\debug` directories contain disposable
compiler output only. They may be deleted while WyrmGrid and development tools
are closed, although the next SQLCipher/OpenSSL build will be considerably
slower.

## Deletion and ordinary file backups

Portable backups remain wherever you place them until you or the storage
provider deletes them. WyrmGrid does not track, rotate, upload, or erase those
copies. Filesystems, synchronisation services, snapshots, and deleted-file
recovery may retain older copies after apparent deletion.

Ordinary whole-system backups can preserve WyrmGrid if they restore both the
application data and the operating-system credential store consistently, but
that behaviour depends on the backup product. A WyrmGrid portable backup is the
explicit cross-system recovery contract.
