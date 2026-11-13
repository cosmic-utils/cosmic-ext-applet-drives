# Flatpak handoff: drive detection and unmounting

Status notes for picking up the `gio-refactor` branch on a machine where Flatpak can be built and run.

## Goal

The applet must run as a Flatpak (`dev.cappsy.CosmicExtAppletDrives`). That is fixed. Inside the sandbox it needs to:

1. Detect removable USB drives and network mounts, and refresh the list when they are mounted or unmounted.
2. Unmount a drive from the popup and have the unmount actually succeed.

Outside Flatpak, the gio-based version works well. Inside Flatpak, drives are not detected reliably and unmounting does not work.

## State of the repository

- Branch `gio-refactor`, which has two commits on top of the last common ancestor with `main`, and has `main` merged in.
- The merge of `main` into `gio-refactor` was resolved. Conflicts were in `Cargo.toml`, `Cargo.lock`, `dev.cappsy.CosmicExtAppletDrives.json`, `src/app.rs`, and `src/lib.rs`. The merge should be committed before any new work starts. Check with `git status`; if it still shows `UU` entries, `git add` the five files and commit with the message in `.git/MERGE_MSG`.
- `cargo check --offline --locked` and `cargo fmt --check` pass.
- Nothing has been run inside a Flatpak yet. Runtime behaviour is untested.

## What the branch changed

### Detection (`src/lib.rs`)

`get_all_devices()` no longer reads `/proc/mounts` and udev. It now:

- Calls `gio::VolumeMonitor::get().mounts()` and drops shadowed mounts.
- Marks a mount as USB if `mount.drive()` exists and `drive.is_removable()` is true.
- Marks a mount as network if the root's `FILESYSTEM_REMOTE` attribute is true.
- Keeps only USB and network mounts. Internal disks are excluded.
- Stores each mount's enumeration index as `idx`. Unmount uses this index.

### Monitor thread (`src/monitor.rs`, new)

- Starts a thread with a GLib main loop.
- Connects `mount-added` and `mount-removed` on the volume monitor and sends `()` on a channel. The UI subscribes to this channel and refreshes the list.
- Receives `Cmd::Unmount(idx)`. It finds the mount with the same index and calls `eject_with_operation` if `can_eject()`, otherwise `unmount_with_operation` if `can_unmount()`.

### UI (`src/app.rs`)

- Keeps the mount list in the model. It is loaded at init and reloaded on `RefreshMounts`.
- A `Subscription::run_with` feeds monitor events into `RefreshMounts`. This replaced the removed `run_with_id` API in libcosmic.
- Opening a mount runs `cosmic-files` through `run_command`, which uses `flatpak-spawn --host` when built with the `flatpak` feature.
- The popup uses libcosmic's `surface` API from `main`.

### Manifest (`dev.cappsy.CosmicExtAppletDrives.json`)

`finish-args` now includes `--socket=session-bus`, `--share=network`, `--system-talk-name=org.freedesktop.UDisks2`, `--talk-name=org.freedesktop.NetworkManager`, `--talk-name=org.freedesktop.Flatpak`, `--filesystem=host`, and many `org.gtk.vfs.*` names.

### Dependencies

- Added `gio = "0.22.1"` and `tokio = { version = "1", features = ["process", "sync"] }`.
- `Cargo.lock` was regenerated with cargo against the merged manifest. Its new crates include `gio`, `glib`, `gobject-sys`, `system-deps`, and `toml 0.9`.

## Known problems to fix before the Flatpak build works

1. **`flatpak/cargo-sources.json` is stale.** It has no entries for `gio`, `glib`, `gio-sys`, `glib-sys`, `gobject-sys`, `glib-macros`, `system-deps`, or `cfg-expr`. An offline Flatpak build will fail. Regenerate it with `just flatpak-cargo-sources` after the lockfile is final, and check that it contains those crates.
2. **Unmount errors are ignored.** In `src/monitor.rs` both `eject_with_operation` and `unmount_with_operation` pass `|_| {}` as the callback. A failure produces no output anywhere, so a silent failure looks identical to success.
3. **The unmount index is only valid for the list the UI last rendered.** The index is recomputed in the monitor thread, so if the device list changes between render and click, the wrong drive could be targeted. Passing a stable identifier, such as the mount's UUID or root path, would be safer.
4. **Commit author identity is not configured on the machine where this was started.** Set `user.name` and `user.email` before committing.

## Leading hypotheses for the Flatpak failure

These are not verified. The first step is to test them.

**H1. Removable drives are classified as non-removable inside the sandbox.**
`mount.drive()` gets its data from a volume-monitor backend, normally UDisks2 through GVfs. That backend is probably not loaded inside the sandbox. If so, `drive()` is `None` and USB drives are filtered out. Network mounts might still appear. If `mount-added` never fires, the list also won't refresh automatically.
- I believe `org.freedesktop.Platform` does not ship the GVfs/UDisks2 GIO modules. Confirm this on the target machine.

**H2. Unmount fails silently.**
UDisks2 unmount and eject go through polkit. The sandbox has no polkit authentication agent, so the request likely fails. Because the callback ignores the result (problem 2 above), nothing is reported.

**H3. Mount paths are not accessible from the sandbox.**
Network and FUSE mounts often live under GVfs paths such as `/run/user/<uid>/gvfs`. `--filesystem=host` covers `/run/media` but may not cover these paths.

## Step 1 on the test machine: measure

1. Build and install the Flatpak from the manifest:

   ```sh
   just flatpak-builder
   ```

   This needs `flatpak-builder` and the Flathub remote. The sources file must be current first (see problem 1).

2. Open a shell inside the sandbox:

   ```sh
   flatpak run --command=sh dev.cappsy.CosmicExtAppletDrives
   ```

3. Inside the sandbox, check what GIO can see:

   ```sh
   gio mount -l
   gio volume -l
   gio mount -l -d   # drives, if supported by this gio version
   gdbus introspect --system --dest org.freedesktop.UDisks2 \
     --object-path /org/freedesktop/UDisks2
   ls "$GIO_EXTRA_MODULES" "${GIO_MODULE_DIR:-}" 2>/dev/null
   echo "$XDG_RUNTIME_DIR"; ls "$XDG_RUNTIME_DIR/gvfs" 2>/dev/null
   ```

   Record the output. For H1, look for whether a USB drive appears with a drive attached.

4. Add temporary logging to find the cause:
   - In `get_all_devices()`, print each mount's name, root path, `drive()`, and `is_removable()` to stderr.
   - In `src/monitor.rs`, replace `|_| {}` with a closure that prints the `Result` on error.
   - Run the applet from a terminal with `flatpak run dev.cappsy.CosmicExtAppletDrives` and watch stderr while mounting and unmounting a USB stick.

5. Compare with the non-Flatpak build by running `cargo run --release` on the same machine with the same stick.

## Step 2: choose a direction

**Option A (recommended): detect with `/proc/mounts` and `/sys/block`; unmount on the host.**

- Detection: parse `/proc/self/mountinfo` or `/proc/mounts`. Decide removable via `/sys/block/<dev>/removable`, the bus via `/sys/block/<dev>/device` or udev properties, and network mounts via the filesystem type (`nfs`, `cifs`, `sshfs`, `fuse.*`).
- Refresh: watch UDisks2 `InterfacesAdded` and `InterfacesRemoved` on the system bus, which the manifest already allows, or poll every few seconds as a fallback.
- Unmount: run `flatpak-spawn --host udisksctl unmount -b <device>`, or `gio mount -u <path>` on the host. The host's polkit agent handles authentication. The existing `run_command` helper already uses `flatpak-spawn --host`.
- Pros: it resembles the original design, which worked in Flatpak; it needs no GIO backend; the permissions are already in the manifest.
- Cons: it parses kernel and system interfaces more directly, and the unmount path shells out.

**Option B: keep GIO and make the backend available.**

- Add GVfs or UDisks2 as Flatpak modules, or switch to a runtime that ships them.
- Pros: the code stays as it is.
- Cons: heavy, fragile across runtime versions, and not something I would start with.

**Option C: talk to UDisks2 directly over D-Bus, using a crate such as `zbus`.**

- Read `Block`, `Drive`, and `Filesystem` objects, and call `Filesystem.Unmount` and `Drive.Eject`.
- Pros: the cleanest long-term model.
- Cons: a new dependency and more code. Polkit still applies, so the same authentication question as Option A must be answered.

Decide after Step 1. If H1 is confirmed and the backend cannot be provided, Option A or C. If only H2 is confirmed, Option A's host-side unmount alone may be enough.

## Step 3: after the direction is chosen

1. Finish the code change on the branch.
2. Fix the Flatpak sources (`just flatpak-cargo-sources`).
3. Commit the merge and the fix.
4. Test in the sandbox with a USB stick and, if possible, an SMB or NFS share. Check mount, refresh, open, and unmount.
5. Remove the temporary logging, or keep it behind a debug flag.

## Useful commands

```sh
git status                      # confirm merge state
cargo check --offline --locked  # quick compile check
cargo fmt --check
just flatpak-cargo-sources      # regenerate flatpak/cargo-sources.json
just flatpak-builder            # build and install the flatpak locally
flatpak run --command=sh dev.cappsy.CosmicExtAppletDrives
```
