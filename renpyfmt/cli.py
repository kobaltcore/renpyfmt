import rich_click as click

from .code_format import code_format


@click.command()
@click.argument(
    "input_file",
    default="-",
    envvar="RPYFMT_IN",
    type=click.File("r", encoding="utf-8"),
)
@click.argument(
    "output_file",
    default="-",
    envvar="RPYFMT_OUT",
    type=click.File("w", encoding="utf-8"),
)
def cli(input_file, output_file):
    text = input_file.read()
    text_fmt = code_format(text)
    output_file.write(text_fmt)


if __name__ == "__main__":
    cli()
