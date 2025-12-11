#!/usr/bin/env python3
"""
Finance CLI for checking stocks/funds info and your portfolio status
"""
from typing import Optional
import typer
from typing_extensions import Annotated
import rich_functions as rf
import portfolio as prt
import ticker as tk

PORTFOLIO_FILE_NAME = "portfolio.json"
app = typer.Typer()


def validate_option(ctx: typer.Context, arg: str, opt: str):
    """
    Generic option validation with specified argument
    """
    if ctx.params.get(arg) is not None and ctx.params.get(opt) is not None:
        raise typer.BadParameter(
            f"--{opt} option in not valid with argument {arg}")


@app.callback()
def main(ctx: typer.Context):
    """
    Cache callback
    """
    ctx.obj = {}


@app.command()
def ticker(ctx: typer.Context, tkr: str,
           info: Annotated[Optional[str], typer.Argument(
               help="Prints Info")] = None,
           attribute: Annotated[Optional[str], typer.Option("--attribute", "-a",
                                                            help="Attribute value")] = None):
    """
    Stocks/Funds info
    """
    validate_option(ctx, "info", "attribute")
    tkr_loaded = tk.Ticker(tkr)
    rf.prettier_info(tkr_loaded, info, attribute)


@app.command()
def portfolio(ctx: typer.Context, total: Annotated[bool,
                                                   typer.Option("--total", "-t",
                                                                help="Prints total value and yield",
                                                                is_flag=True)] = False,
              file: Annotated[str, typer.Option("--file", "-f",
                                                          help="File Path")] = PORTFOLIO_FILE_NAME,
              cache: Annotated[bool, typer.Option("--cache", help="Cache renewal", is_flag=True)] = False):
    """
    Portfolio Status
    """
    prt_loaded = prt.Portfolio(file)
    if not cache:
        rf.prettier_portfolio(prt_loaded, total)


if __name__ == "__main__":
    app()
