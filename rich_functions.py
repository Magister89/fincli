"""
Rich functions for prettier outputs
"""
from typing import Optional
from rich import print as printp
from rich.table import Table
import portfolio as prt
import ticker as tk

def prettier_portfolio(portfolio: prt.Portfolio, total: bool):
    """
    Prints table portfolio status
    """
    table = Table(show_header=True,
                  header_style="bold magenta", title_justify="center")
    table.add_column("Total" if total else "Ticker", justify="center")
    table.add_column("Value", justify="center")
    table.add_column("Yield", justify="center")
    prtf = portfolio.get_portfolio()
    if total:
        prtf = portfolio.get_total_portfolio_value()
    for element in prtf:
        ticker = element['ticker']
        price = element['price']
        yield_p = element['yield']
        ticker_t = f'[bold blue]{ticker}[/bold blue]'
        price_t = f'[bold white]{price}[/bold white]'
        conditional_yield_t = f'[bold green]▲ {yield_p}%[/bold green]' \
            if yield_p >= 0 else f'[bold red]▼ {yield_p}%[/bold red]'
        table.add_row(ticker_t, price_t, conditional_yield_t)
    printp(table)

def prettier_info(ticker: tk.Ticker, info: Optional[str], attribute: str):
    """
    Prints table ticker info
    """
    if info:
        table = Table(show_header=True, header_style="bold magenta", title_justify="left")
        table.add_column("Attribute", justify="left")
        table.add_column("Value", justify="left")
        for key, value in ticker.get_ticker_fast_info().items():
            table.add_row(f'[bold blue]{key}[/bold blue]', str(value))
        printp(table)
    else:
        printp(f"{attribute} for [bold green]{ticker.ticker}[/bold green] is [bold]{ticker.get_ticker_fast_info[attribute]}[/bold]")
