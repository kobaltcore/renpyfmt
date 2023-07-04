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

"""Functions that make the user's life easier."""


import contextlib
import time
from collections.abc import Iterable

import renpy

from .color import Color


def lookup_displayable_prefix(d):
    """
    Given `d`, a string given a displayable, returns the displayable it
    corresponds to or None if it does not correspond to one.
    """

    prefix, colon, arg = d.partition(":")

    if not colon:
        return None

    fn = renpy.config.displayable_prefix.get(prefix, None)
    if fn is None:
        return None

    return displayable(fn(arg))


def displayable_or_none(
    d, scope=None, dynamic=True
):  # type: (Any, dict|None, bool) -> renpy.display.core.Displayable|None
    if isinstance(d, renpy.display.core.Displayable):
        return d

    if d is None:
        return d

    if isinstance(d, str):
        if not d:
            raise Exception("An empty string cannot be used as a displayable.")
        elif ("[" in d) and renpy.config.dynamic_images and dynamic:
            return renpy.display.image.DynamicImage(d, scope=scope)

        rv = lookup_displayable_prefix(d)

        if rv is not None:
            return rv
        elif d[0] == "#":
            return renpy.store.Solid(d)
        elif "." in d:
            return renpy.store.Image(d)
        else:
            return renpy.store.ImageReference(tuple(d.split()))

    if isinstance(d, Color):
        return renpy.store.Solid(d)  # type: ignore

    if isinstance(d, list):
        return renpy.display.image.DynamicImage(d, scope=scope)  # type: ignore

    # We assume the user knows what he's doing in this case.
    if hasattr(d, "_duplicate"):
        return d

    if d is True or d is False:
        return d

    raise Exception(f"Not a displayable: {d!r}")


class ImageReference:
    """
    ImageReference objects are used to reference images by their name,
    which is a tuple of strings corresponding to the name used to define
    the image in an image statment.
    """

    nosave = ["target"]

    target = None
    old_transform = None
    param_target = None

    __version__ = 1

    def after_upgrade(self, version):
        if version < 1:
            if isinstance(self.param_target, renpy.display.transform.Transform):
                self.old_transform = self.param_target

    def __init__(self, name, **properties):
        """
        @param name: A tuple of strings, the name of the image. Or else
        a displayable, containing the image directly.
        """

        super().__init__(**properties)

        self.name = name
        self.target = None  # type: renpy.display.core.Displayable|None

    def _repr_info(self):
        return repr(self.name)

    def __hash__(self):
        return hash(self.name)

    def __eq__(self, o):
        if self is o:
            return True

        if not self._equals(o):
            return False

        if self.name != o.name:
            return False

        return True

    def _target(self):
        if self.target is None:
            self.find_target()

        return self.target._target()

    def find_target(self):
        name = self.name

        if isinstance(name, renpy.display.core.Displayable):
            self.target = name
            return True

        if not isinstance(name, tuple):
            name = tuple(name.split())

        def error(msg):
            self.target = renpy.text.text.Text(msg, color=(255, 0, 0, 255), xanchor=0, xpos=0, yanchor=0, ypos=0)

            if renpy.config.debug:
                raise Exception(msg)

        target = None  # typing

        args = []

        while name:
            target = images.get(name, None)

            if target is not None:
                break

            args.insert(0, name[-1])
            name = name[:-1]

        if not name:
            error("Image '%s' not found." % " ".join(self.name))
            return False

        if name and (self._args.name == name):
            error("Image '{}' refers to itself.".format(" ".join(name)))
            return False

        args += self._args.args

        try:
            a = self._args.copy(name=name, args=args)
            self.target = target._duplicate(a)

        except Exception as e:
            if renpy.config.raise_image_exceptions and (renpy.config.debug or renpy.config.developer):
                raise

            error(str(e))
            return False

        # Copy the old transform over.
        new_transform = self.target._target()

        if isinstance(new_transform, renpy.display.transform.Transform):
            if self.old_transform is not None:
                new_transform.take_state(self.old_transform)

            self.old_transform = new_transform

        else:
            self.old_transform = None

        return True

    _duplicatable = True

    def _duplicate(self, args):
        if args and args.args:
            args.extraneous()

        rv = self._copy(args)
        rv.target = None

        if isinstance(rv.name, renpy.display.core.Displayable):
            if rv.name._duplicatable:
                rv.name = rv.name._duplicate(args)

        rv.find_target()

        return rv

    def _unique(self):
        if self.target is None:
            self.find_target()

        self.target._unique()
        self._duplicatable = False

    def _in_current_store(self):
        if self.target is None:
            self.find_target()

        target = self.target._in_current_store()

        if target is self.target:
            return self

        rv = self._copy()
        rv.target = target
        return rv

    def _handles_event(self, event):
        if self.target is None:
            return False

        return self.target._handles_event(event)

    def _hide(self, st, at, kind):
        if self.target is None:
            self.find_target()

        return self.target._hide(st, at, kind)

    def set_transform_event(self, event):
        if self.target is None:
            self.find_target()

        return self.target.set_transform_event(event)

    def event(self, ev, x, y, st):
        if self.target is None:
            self.find_target()

        return self.target.event(ev, x, y, st)

    def render(self, width, height, st, at):
        if self.target is None:
            self.find_target()

        return wrap_render(self.target, width, height, st, at)

    def get_placement(self):
        if self.target is None:
            self.find_target()

        if not renpy.config.imagereference_respects_position:
            return self.target.get_placement()

        xpos, ypos, xanchor, yanchor, xoffset, yoffset, subpixel = self.target.get_placement()

        if xpos is None:
            xpos = self.style.xpos

        if ypos is None:
            ypos = self.style.ypos

        if xanchor is None:
            xanchor = self.style.xanchor

        if yanchor is None:
            yanchor = self.style.yanchor

        return xpos, ypos, xanchor, yanchor, xoffset, yoffset, subpixel

    def visit(self):
        if self.target is None:
            self.find_target()

        return [self.target]


def displayable(d, scope=None):  # type(d, dict|None=None) -> renpy.display.core.Displayable|None
    """
    :doc: udd_utility
    :name: renpy.displayable

    This takes `d`, which may be a displayable object or a string. If it's
    a string, it converts that string into a displayable using the usual
    rules.
    """
    if isinstance(d, str):
        if not d:
            raise Exception("An empty string cannot be used as a displayable.")
        elif ("[" in d) and renpy.config.dynamic_images:
            return renpy.display.image.DynamicImage(d, scope=scope)

        rv = lookup_displayable_prefix(d)

        if rv is not None:
            return rv
        elif d[0] == "#":
            return renpy.store.Solid(d)
        elif "." in d:
            return renpy.store.Image(d)
        else:
            return ImageReference(tuple(d.split()))

    if isinstance(d, renpy.display.core.Displayable):
        return d

    if isinstance(d, Color):
        return renpy.store.Solid(d)

    if isinstance(d, list):
        return renpy.display.image.DynamicImage(d, scope=scope)

    # We assume the user knows what he's doing in this case.
    if hasattr(d, "_duplicate"):
        return d

    if d is True or d is False:
        return d

    raise Exception(f"Not a displayable: {d!r}")


def dynamic_image(
    d, scope=None, prefix=None, search=None
):  # type: (Any, dict|None, str|None, list|None) -> renpy.display.core.Displayable|None
    """
    Substitutes a scope into `d`, then returns a displayable.

    If `prefix` is given, and a prefix has been given a prefix search is
    performed until a file is found. (Only a file can be used in this case.)
    """

    if not isinstance(d, list):
        d = [d]

    def find(name):
        if renpy.exports.image_exists(name):
            return True

        if renpy.loader.loadable(name):
            return True

        if lookup_displayable_prefix(name):
            return True

        if (len(d) == 1) and (renpy.config.missing_image_callback is not None):
            if renpy.config.missing_image_callback(name):
                return True

    for i in d:
        if not isinstance(i, str):
            continue

        if (prefix is not None) and ("[prefix_" in i):
            if scope:
                scope = dict(scope)
            else:
                scope = {}

            for p in renpy.styledata.stylesets.prefix_search[prefix]:  # @UndefinedVariable
                scope["prefix_"] = p

                rv = renpy.substitutions.substitute(i, scope=scope, force=True, translate=False)[0]

                if find(rv):
                    return displayable_or_none(rv)

                if search is not None:
                    search.append(rv)

        else:
            rv = renpy.substitutions.substitute(i, scope=scope, force=True, translate=False)[0]

            if find(rv):
                return displayable_or_none(rv)

            if search is not None:
                search.append(rv)

    rv = d[-1]

    if find(rv):
        return displayable_or_none(rv, dynamic=False)

    return None


def predict(d):
    d = renpy.easy.displayable_or_none(d)

    if d is not None:
        renpy.display.predict.displayable(d)


@contextlib.contextmanager
def timed(name):
    start = time.time()
    yield
    print("{}: {:.2f} ms".format(name, (time.time() - start) * 1000.0))


def split_properties(properties, *prefixes):
    """
    :doc: other

    Splits up `properties` into multiple dictionaries, one per `prefix`. This
    function checks each key in properties against each prefix, in turn.
    When a prefix matches, the prefix is stripped from the key, and the
    resulting key is mapped to the value in the corresponding dictionary.

    If no prefix matches, an exception is thrown. (The empty string, "",
    can be used as the last prefix to create a catch-all dictionary.)

    For example, this splits properties beginning with text from
    those that do not::

        text_properties, button_properties = renpy.split_properties(properties, "text_", "")
    """

    rv = []

    for _i in prefixes:
        rv.append({})

    if not properties:
        return rv

    prefix_d = list(zip(prefixes, rv))

    for k, v in properties.items():
        for prefix, d in prefix_d:
            if k.startswith(prefix):
                d[k[len(prefix) :]] = v
                break
        else:
            raise Exception(f"Property {k} begins with an unknown prefix.")

    return rv


def to_list(value, copy=False):
    """
    If the value is an iterable, turns it into a list, otherwise wraps it into one.
    If a list is provided and `copy` is True, a new list will be returned.
    """
    if isinstance(value, list):
        return list(value) if copy else value

    if not isinstance(value, str) and isinstance(value, Iterable):
        return list(value)

    return [value]


def to_tuple(value):
    """
    Same as to_list, but with tuples.
    """
    if isinstance(value, tuple):
        return value

    if not isinstance(value, str) and isinstance(value, Iterable):
        return tuple(value)

    return (value,)


def run_callbacks(cb, *args, **kwargs):
    """
    Runs a callback or list of callbacks that do not expect results
    """

    if cb is None:
        return None

    if isinstance(cb, (list, tuple)):
        rv = None

        for i in cb:
            new_rv = run_callbacks(i, *args, **kwargs)

            if new_rv is not None:
                rv = new_rv

        return rv

    return cb(*args, **kwargs)
