import re
from importlib.util import MAGIC_NUMBER as MAGIC

# Change this to force a recompile when required.
MAGIC += b"_v3.1"

bytecode_version = 1

record_pycode = True

all_pycode = []

all_pyexpr = []

# A map from line loc (elided filename, line) to the Line object representing
# that line.
lines = {}

# The set of files that have been loaded.
files = set()


class Line:
    """
    Represents a logical line in a file.
    """

    def __init__(self, filename, number, start):
        filename = filename.replace("\\", "/")

        # The full path to the file with the line in it.
        self.filename = filename

        # The line number.
        self.number = number

        # The offset inside the file at which the line starts.
        self.start = start

        # The offset inside the file at which the line ends.
        self.end = start

        # The offset inside the lime where the line delimiter ends.
        self.end_delim = start

        # The text of the line.
        self.text = ""

        # The full text, including any comments or delimiters.
        self.full_text = ""

    def __repr__(self):
        return f"<Line {self.filename}:{self.number} {self.text!r}>"


sentinels = {}


class Sentinel:
    """
    This is used to represent a sentinel object. There will be exactly one
    sentinel object with a name existing in the system at any time.
    """

    def __new__(cls, name):
        rv = sentinels.get(name, None)

        if rv is None:
            rv = object.__new__(cls)
            sentinels[name] = rv

        return rv

    def __init__(self, name):
        self.name = name

    def __reduce__(self):
        return (Sentinel, (self.name,))


def encode_say_string(s):
    """
    Encodes a string in the format used by Ren'Py say statements.
    """

    s = s.replace("\\", "\\\\")
    s = s.replace("\n", "\\n")
    s = s.replace('"', '\\"')
    s = re.sub(r"(?<= ) ", "\\ ", s)

    return '"' + s + '"'
