# renpyfmt

`renpyfmt` is a source code formatter for Ren'Py script.

Right now it only supports formatting blocks of Python code contained within Ren'Py script files, but the intent is to expand this to allow formatting of actual Ren'Py script as well.

The contained Python source code is formatted via [black](https://github.com/psf/black).

All Python related statements are supported:
- `$` single-line statements
- `python:` blocks
- `init python:` blocks
- `python early:` blocks
