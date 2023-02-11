# Copyright 2004-2023 Tom Rothamel <pytom@bishoujo.us>
#
# Permission is hereby granted, free of charge, to any person
# obtaining a copy of this software and associated documentation files
# (the "Software"), to deal in the Software without restriction,
# including without limitation the rights to use, copy, modify, merge,
# publish, distribute, sublicense, and/or sell copies of the Software,
# and to permit persons to whom the Software is furnished to do so,
# subject to the following conditions:
#
# The above copyright notice and this permission notice shall be
# included in all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
# EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
# MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
# NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
# LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
# OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
# WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.


def letterlike(c):
    if "a" <= c <= "z":
        return 1

    if "A" <= c <= "Z":
        return 1

    if "0" <= c <= "9":
        return 1

    if "_" == c:
        return 1

    return 0


def match_logical_word(s, pos):
    start = pos
    len_s = len(s)
    c = s[pos]

    if c == " ":
        pos += 1

        while pos < len_s:
            if not (s[pos] == " "):
                break

            pos += 1

    elif letterlike(c):
        pos += 1

        while pos < len_s:
            if not letterlike(s[pos]):
                break

            pos += 1

    else:
        pos += 1

    word = s[start:pos]

    if (pos - start) >= 3 and (word[0] == "_") and (word[1] == "_"):
        magic = True
    else:
        magic = False

    return s[start:pos], magic, pos
