# GMHistory

Tool written in Rust to download and extract GameMaker IDEs and runtimes.

This tool can:

- Download _GameMaker: Studio 1_ IDE installers (archive.org)
- Download _GameMaker Studio 2_ IDE installers (partially archive.org)
- Download modern _GameMaker_ IDE installers
- Download modern (GMS2+) GameMaker _runtimes_
- Extract the directory structure of the IDE installer (just 7-Zip)
- Extract the password-protected `UpgradeZip` of _GameMaker: Studio 1_ which contains the "actual" IDE
- Extract the password-protected runtime ZIP archives

Of all **217** GameMaker IDE versions, this tool can download **164**.

Of all **1395** GameMaker runtimes, this tool can download **1288**.

## Installation

> [!NOTE]
> This tool is still a work-in-progress.
> Things might not work yet. Things might change.
> I will release pre-built binaries when it is stable enough.

1. Install the [Rust toolchain](https://rustup.rs)
2. Install 7-Zip (see below).
3. Clone this repo: `git clone https://github.com/BioTomateDE/GMHistory && cd GMHistory`
4. Compile: `cargo b -r`
5. Run the program: `./target/release/gmhistory`

To understand the program better, call it with `--help` on a subcommand, like `./gmhistory download-ides --help`.

### Getting 7-Zip

- Website: <https://7-zip.org>
- Debian/Ubuntu: `sudo apt install 7zip`
- Arch Linux: `sudo pacman -S 7zip`
- I don't care about other distros

If you are a poor soul still using Windows, then you'll also have to make sure that the `7z` program is in your PATH.
See <https://stackoverflow.com/questions/4822400> for example.

## IDEs

### HTML changelogs

These are the official IDE changelogs by YoYo Games.

- (changelogs 1.0.23+) 1.1.412 - 1.1.754: <https://web.archive.org/web/20130123092814/https://store.yoyogames.com/downloads/gm-studio/release-notes-studio.html#:~:text=Version%201.1.412>
- 1.1.867 - 1.4.1804: <https://web.archive.org/web/20180829163534/http://store.yoyogames.com/downloads/gm-studio/release-notes-studio.html#:~:text=Version%201.1.867>
- 1.4.9999 (latest revision): <https://store.yoyogames.com/downloads/gm-studio/release-notes-studio.html>
- (changelogs 2.0.1.33+) 2.1.4.295 - latest (2024.14.4.222): <https://gms.yoyogames.com/ReleaseNotes.html>

### 1.4.9999 revisions

There are multiple revisions of 1.4.9999.
For the same download link and page, the changelogs and IDE installer differ slightly for different archive.org snapshots.

> TODO: Test whether the IDE is actually different (apart from metadata changes)?

This program always downloads the oldest available 1.4.9999 revision.

| date       | changelog ID   | installer ID   |
| ---------- | -------------- | -------------- |
| 2018-08-28 | 20180830234512 | 20180902152118 |
| 2018-09-06 | 20180927020445 | 20180902152118 |
| 2018-10-04 | 20181225202637 | 20190327151917 |

- The **date** is the date specified in the header in the changelog HTML: `/html/body/h1[1]/a[1]`.

- The **changelog ID** is the "archive.org timestamp" of the changelog HTML:
  `https://web.archive.org/web/{TIMESTAMP}/https://store.yoyogames.com/downloads/gm-studio/release-notes-studio.html`

- The **installer ID** is the "archive.org timestamp" of the Windows IDE installer download:
  `https://web.archive.org/web/{TIMESTAMP}/http://store.yoyogames.com/downloads/gm-studio/GMStudio-Installer-1.4.9999.exe`

This installer timestamp is the same for 2018-08-28 and 2018-09-06;
archive.org did unfortunately not record a snapshot between those two 1.4.9999 revisions.

### Lost media

Some IDEs are not downloadable via [archive.org](https://web.archive.org) or the official website anymore:

- GM:S 1 has no downloads before 1.1.690
- GMS 2 has no downloads before 2.1.4.295
- There might be more 1.4.9999 revisions between the snapshots recorded by archive.org

If you have access to these IDEs, let me know!

Contact me via:

- Discord: `@biotomate.de` (User ID 1383525574508679228)
- Email: `bio@biotomate.dev`
- Github Issue/PR

## Runtimes

The `Zeus-Manifest.rss` file can be acquired at <https://gms.yoyogames.com/Zeus-Runtime.rss>.
This contains links to all the GameMaker runtimes this tool downloads.

There might actually be more manifests linking to other interesting GameMaker stuff. If you have those, let me know!

### Mac Mono Runtime

The `mac.mono` runtime will not be downloaded by this tool, as it is not GameMaker related.
You can get them yourself:

- GameMaker 2.1.4.200 - 2.2.0.258: <https://gms.yoyogames.com/MonoFramework-MDK-4.8.0.520.macos10.xamarin.universal.pkg>
- GameMaker 2.2.1.287 - 2.3.0.401: <https://gms.yoyogames.com/MonoFramework-MDK-5.10.1.57.macos10.xamarin.universal.pkg>
- GameMaker 2.3.1.406 - 2022.3.0.497: <https://gms.yoyogames.com/MonoFramework-MDK-5.10.1.57.macos10.xamarin.universal.pkg>

These GameMaker version ranges are complete; they range from the oldest RSS item containing `mac.mono` to the newest one containing it.

### Lost Media

Of all 1395 runtimes, 107 seem to be missing from the servers (or need some sort of authorization to access).
These are listed in [`missingruntimes.txt`](missingruntimes.txt).
Trying to download any of these runtimes results in HTTP 403 ("Forbidden").
Keep in mind that the servers respond with 403 to _any_ path that doesn't exist, even root: <https://gms.yoyogames.com>.
This probably(?) means that the download links were removed, but it _could_ also be a super duper
secret runtime that only cool people can access. Who knows.
