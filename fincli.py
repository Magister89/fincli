#!/usr/bin/env python3
"""
Finance CLI for checking stocks/funds info and your portfolio status
"""
from typing import Optional
import typer
from typing_extensions import Annotated
import finance_functions as ff

app = typer.Typer()

@app.command()
def ticker(tkr: str, info: Annotated[Optional[str], typer.Argument(help= "Prints Info")] = None,
           attribute: str = typer.Option("lastPrice", "--attribute", "-a", help="Attribute value")):
    """
    Stocks/Funds info
    """
    ff.get_ticker_info_price(tkr, info, attribute)

@app.command()
def portfolio(total: bool = typer.Option(False, "--total", "-t", help= "Prints total value and yield", is_flag=True),
              file: str = typer.Option("portfolio.json", "--file", "-f", help="File Path")):
    """
    Portfolio Status
    """
    ff.portfolio_print(file, total)


if __name__ == "__main__":
    app()
