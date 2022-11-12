# -*- coding: utf-8 -*-
import click


@click.command()
@click.argument(
    "input_file",
    default="-",
    envvar="RPYFMT_IN",
    type=click.File("r"),
)
@click.argument(
    "output_file",
    default="-",
    envvar="RPYFMT_OUT",
    type=click.File("w"),
)
def cli(input_file, output_file):
    text = input_file.read()
    text_fmt = format(text)
    output_file.write(text_fmt)


if __name__ == "__main__":
    cli()
