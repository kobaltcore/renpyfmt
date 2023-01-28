import copy
import re
from collections import defaultdict

import black

from .common import dedent, indent


def code_format(source):
    source = [line.rstrip() for line in source.splitlines()]

    reformatted = {}

    for line_num, line in enumerate(source):
        m = re.match(r"(\s+)\$\s*(.*)", line)
        if not m:
            continue
        leading_spaces, code = m.groups()
        src_indent = len(leading_spaces)
        code_fmt = black.format_str(code, mode=black.Mode(line_length=1000)).split("\n")
        new_code = ""
        for i, line in enumerate(code_fmt):
            spaces = " " * src_indent
            if i == 0:
                new_code += f"{spaces}$ {line}\n"
                continue
            new_code += f"{spaces}{line}\n"
        new_code = re.sub(r"\s+$", "", new_code)
        reformatted[(line_num, line_num)] = new_code

    is_python = False
    current_group = None
    python_block_types = {}
    python_block_ranges = {}
    current_group_line_num = 0
    python_blocks = defaultdict(list)
    for line_num, line in enumerate(source):
        m = re.match(r"(\s*)(.*)", line)
        if not m:
            continue
        leading_spaces, code = m.groups()
        code = code.strip()
        src_indent = len(leading_spaces)

        if src_indent == 0 and code:
            python_block_ranges[current_group_line_num] = (
                current_group_line_num,
                line_num - 1,
            )
            current_group = code.rstrip(":")
            current_group_line_num = line_num
            if current_group == "python early":
                is_python = True
            elif current_group == "python":
                is_python = True
            elif m := re.match(r"init\s+(?:-|\+)?\d+\s+python", current_group):
                is_python = True
            else:
                is_python = False
            python_block_types[current_group_line_num] = current_group

        if current_group and is_python and line_num != current_group_line_num:
            python_blocks[current_group_line_num].append(line)

    if current_group_line_num:
        python_block_ranges[current_group_line_num] = (current_group_line_num, line_num)

    for line_num, block in python_blocks.items():
        block, margin = dedent("\n".join(block))
        block_fmt = black.format_str(block, mode=black.FileMode())
        start, end = python_block_ranges[line_num]
        reformatted[(start, end)] = f"{python_block_types[line_num]}:\n" + indent(block_fmt, margin)

    code_fmt = copy.deepcopy(source)
    for (start, end), code in sorted(reformatted.items(), key=lambda x: x[0][0], reverse=True):
        if end == -1:
            code_fmt = [code]
            continue
        del code_fmt[start : end + 1]
        code_fmt.insert(start, code)

    code_fmt = "\n".join(code_fmt).strip() + "\n"

    return code_fmt
