# Removable Drives Applet (for the COSMIC™ Desktop)

An applet for the COSMIC™ desktop to manage your drives. This is in very early stages but is functional as it stands.

### What we have

- List of all mounted drives (removable USB and internal drives mounted by the user)
- Click the drive to open the mount in COSMIC Files
- Click the unmount button to unmount the drive before removable
  
### Possible future additions

- Network mounts
- Possibly notifications for unmounting, errors, and a bossy warning about removing a drive before unmounting (with the option to disable, of course)
- More robust unmounting process (currently it's just running the umount command and hoping the user has permission to do so)
- Use your default file browser when opening the drive, rather than forcing COSMIC Files

## Flatpak installation

By far the best way to install the applet is through the official COSMIC™ Flatpak repository. Firstly, ensure you have Flatpak itself installed. You then should be able to search for and install Logo Menu from the COSMIC™ Store, under the Applets category. Alternatively, you can ensure you have the correct repo enabled and install through the command line.

```sh
flatpak remote-add --if-not-exists --user cosmic https://apt.pop-os.org/cosmic/cosmic.flatpakrepo
flatpak install dev.cappsy.CosmicExtAppletDrives
```

## Arch User Repository installation

The applet can be installed directly from [the AUR](https://aur.archlinux.org/packages/cosmic-ext-applet-drives-git), and this will get you very latest code and not be tied to tagged releases. You will need `base-devel` and `git` if you don't have them already.

```sh
sudo pacman -S base-devel git
git clone https://aur.archlinux.org/cosmic-ext-applet-drives-git.git
cd cosmic-ext-applet-drives-git && makepkg -si
```

## Manual installation

You're going to need to make sure you have the ability to compile Rust binaries, along with `git` and `just`

```sh
git clone https://github.com/cappsyco/cosmic-ext-applet-drives && cd cosmic-ext-applet-drives
just build-release
sudo just install
```

## Credit & thanks
* [System76 and their COSMIC desktop environment](https://system76.com/cosmic/)
* [COSMIC Utilities](https://github.com/cosmic-utils/) - Organization containing third party utilities for COSMIC™
