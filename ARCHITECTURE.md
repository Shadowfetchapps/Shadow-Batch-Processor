# Architecture

`shadow_batch` plans a file list + pipeline, then runs each file independently.

- `files` validates drops (no folders, no empty files)
- `pipeline` previews destination names
- `ops` applies steps with structured FFmpeg / ImageMagick argv
- `runner` pauses between files, cancels process groups, and keeps going after a single failure
- `ui` is GTK4 / libadwaita

Outputs are unique copies. Temporary directories carry `.shadow-batch-temp` and only those are cleaned.
