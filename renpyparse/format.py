import sys

import renpy
import renpy.parser as parser


def get_imspec(imspec):
    image_name, expression, tag, at_list, layer, zorder, behind = imspec
    if expression:
        code = f"expression {expression}"
    else:
        code = f"{' '.join(image_name)}"
    if tag:
        code += f" as {tag}"
    if at_list:
        code += f" at {', '.join(at_list)}"
    if layer:
        code += f" onlayer {layer}"
    if zorder:
        code += f" zorder {zorder}"
    if behind:
        code += f" behind {', '.join(behind)}"
    return code


def get_atl(atl, depth=0):
    if hasattr(atl, "statements"):
        code = ""
        for statement in atl.statements:
            code += get_atl(statement, depth + 1)
        return f"{'    ' * depth}block:\n{code}"
    ctx = renpy.atl.Context({})
    item = atl.compile(ctx)
    code = f"unknown: {item}"
    if isinstance(item, renpy.atl.Interpolation):
        merged_properties = f"\n{'    ' * depth}".join([f"{k} {v}" for k, v in item.properties])
        if item.warper == "instant":
            code = f"{merged_properties}"
        else:
            code = f"{item.warper} {item.duration} {merged_properties}"
    elif isinstance(item, renpy.atl.Child):
        if item.transition:
            code = f"{' '.join(item.child.name)} {item.transition}"
        else:
            code = f"{' '.join(item.child.name)}"
    assembled_code = f"{'    ' * depth}{code}\n"
    return assembled_code


def render_statement(stmt, depth=0, indent=4):
    code = ""
    level = " " * indent
    spaces = level * depth

    if isinstance(stmt, renpy.ast.Python):
        lines = stmt.code.source.strip().split("\n")
        if len(lines) == 1:
            code += f"{spaces}$ {lines[0]}"
        else:
            merged = f"\n{spaces}{level}".join(lines)
            code += f"\n{spaces}python:\n{spaces}{level}{merged}"
    elif isinstance(stmt, renpy.ast.Scene):
        code += f"{spaces}scene {get_imspec(stmt.imspec)}"
        if stmt.atl:
            code += f":\n{get_atl(stmt.atl, depth + 1)[:-1]}"
    elif isinstance(stmt, renpy.ast.With):
        if stmt.expr != "None":
            code += f"{spaces}with {stmt.expr}"
    elif isinstance(stmt, renpy.ast.Show):
        code += f"\n{spaces}show {get_imspec(stmt.imspec)}"
        if stmt.atl:
            code += f":\n{get_atl(stmt.atl, depth + 1)[:-1]}"
    elif isinstance(stmt, renpy.ast.Hide):
        print("Hide", stmt.imspec)
    elif isinstance(stmt, renpy.ast.Jump):
        print("Jump", stmt.target, stmt.expression)
    elif isinstance(stmt, renpy.ast.Say):
        code += f"\n{spaces}{stmt.get_code()}"
    elif isinstance(stmt, renpy.ast.Init):
        if (
            len(stmt.block) == 1
            and (isinstance(stmt.block[0], renpy.ast.Define))
            or isinstance(stmt.block[0], renpy.ast.Default)
        ):
            code += render_statement(stmt.block[0])
        else:
            code += f"\n{spaces}init:"
            for b in stmt.block:
                code += f"\n{spaces}{level}{render_statement(b)}"
    elif isinstance(stmt, renpy.ast.Label):
        if stmt.parameters:
            params = []
            found_default = False
            for k, v in stmt.parameters.parameters:
                if v is not None:
                    found_default = True
                if found_default:
                    params.append(f"{k}={v}")
                else:
                    params.append(f"{k}")
            code += f"\n{spaces}label {stmt.name}({', '.join(params)}):"
        else:
            code += f"\n{spaces}label {stmt.name}:"
        sub_code = []
        for b in stmt.block:
            sc = render_statement(b, depth + 1)
            if not sc:
                continue
            sub_code.append(sc)
        sub_code = f"\n{spaces}".join(sub_code)
        if not sub_code.startswith("\n"):
            sub_code = f"\n{sub_code}"
        code += sub_code
    elif isinstance(stmt, renpy.ast.Define):
        if stmt.store == "store":
            code += f"{spaces}define {stmt.varname} = {stmt.code.source}"
        else:
            code += f"{spaces}define {stmt.store}.{stmt.varname} = {stmt.code.source}"
    elif isinstance(stmt, renpy.ast.Default):
        if stmt.store == "store":
            code += f"{spaces}default {stmt.varname} = {stmt.code.source}"
        else:
            code += f"{spaces}default {stmt.store}.{stmt.varname} = {stmt.code.source}"
    elif isinstance(stmt, renpy.ast.Return):
        if stmt.expression:
            code += f"\n{spaces}return {stmt.expression}"
        else:
            code += f"\n{spaces}return"
    else:
        code += f"{spaces}Unsupported: {stmt}"

    return code


def main():
    statements, error = parser.parse(sys.argv[1])

    if error:
        for error in statements:
            print(error)
        return

    code = ""
    for stmt in statements:
        code += render_statement(stmt) + "\n"

    print(code)


if __name__ == "__main__":
    main()
