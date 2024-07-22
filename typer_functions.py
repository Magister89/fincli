"""
Rich functions for prettier outputs
"""
from typing import Optional
from rich import print as printp
from rich.table import Table

def prettier_portfolio(portfolio: list, total: bool):
    """
    Prints table portfolio status
    """
    table = Table(show_header=True,
                  header_style="bold magenta", title_justify="center")
    table.add_column("Total" if total else "Ticker", justify="center")
    table.add_column("Value", justify="center")
    table.add_column("Yield", justify="center")
    for element in portfolio:
        ticker = element['ticker']
        price = element['price']
        yield_p = element['yield']
        ticker_t = f'[bold blue]{ticker}[/bold blue]'
        price_t = f'[bold white]{price}[/bold white]'
        conditional_yield_t = f'[bold green]▲ {yield_p}%[/bold green]' \
            if yield_p >= 0 else f'[bold red]▼ {yield_p}%[/bold red]'
        table.add_row(ticker_t, price_t, conditional_yield_t)
    printp(table)

def prettier_info(info: dict):
    """
    Prints table ticker info
    """
    table = Table(show_header=True, header_style="bold magenta", title_justify="left")
    table.add_column("Attribute", justify="left")
    table.add_column("Value", justify="left")

    for key, value in info.items():
        table.add_row(f'[bold blue]{key}[/bold blue]', str(value))

    printp(table)
