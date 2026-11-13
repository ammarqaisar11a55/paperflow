# Privacy

PaperFlow is built so that your documents never leave your computer.

## What PaperFlow does not do

- It does not upload documents or send their contents anywhere.
- It does not connect to the internet. The snap has no network permission at all, and the
  interface's content security policy only allows the app's own files.
- It has no telemetry, analytics, crash reporting, update checks or advertising.
- It does not require or offer an account.
- It does not store passwords. A password is passed to qpdf through standard input for the one
  job that needs it, never on the command line (where other programs could see it), never in a
  file, and never in a log. The recent activity list does not include tool settings.
- It does not modify your original files.
- It does not run macros or scripts from documents. Word, PowerPoint, Excel and HTML files are
  converted by LibreOffice with macros disabled and linked internet content blocked, and HTML is
  never executed.

## What PaperFlow keeps on your computer

| Data | Contents | Where | How to remove |
| --- | --- | --- | --- |
| Settings | Theme, default folders and presets, favorite tools | `~/.config/io.github.ammarqaisar11a55.paperflow/settings.json` (snap: under `~/snap/paperflow/`) | Delete the file, or remove the snap |
| Recent activity | For each job: file names, the tool, the outcome, the time and where results were saved. Never document contents. At most 200 entries. | `~/.local/share/io.github.ammarqaisar11a55.paperflow/activity.json` | Settings › Privacy › Clear, or turn "Keep recent activity" off |
| Temporary files | Private copies of inputs and partial results while a job runs | `/tmp/paperflow-<uid>/jobs/<job>` (folder readable only by you) | Deleted automatically when each job ends; Settings › Privacy › Clear; also at start-up |
| Results | The files you asked for | The folder you chose | Delete them like any other file |

## Permissions in the snap

| Permission | Why |
| --- | --- |
| `home` | Open your documents and save results next to them or in a folder you choose. |
| `removable-media` | The same for USB drives and other removable storage. |
| `browser-support` | Required by WebKitGTK, which draws the interface, to sandbox its web process. |
| GNOME extension (`desktop`, `wayland`, `x11`, `opengl`, `gsettings`…) | Show a window, use the theme and fonts, and open file dialogs. |

There is no `network` permission.

## Telling for yourself

- Disconnect from the network: every tool keeps working.
- `snap connections paperflow` lists exactly what the snap can access.
- The source code is public; the only place the interface talks to the backend is
  `src/services/api.ts`, and the only external programs run are listed in
  `src-tauri/src/services/engines.rs`.
