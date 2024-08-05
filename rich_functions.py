"""
Rich functions for prettier outputs
"""
from typing import Optional
from rich import print as printp
from rich.table import Table
import portfolio as prt
import ticker as tk
DEFAULT_ATTRIBUTE = "previousClose"

def prettier_portfolio(portfolio: prt.Portfolio, total: bool):
    """
    Prints table portfolio status
    """
    table = Table(show_header=True,
                  header_style="bold magenta", title_justify="center")
    table.add_column("Total" if total else "Ticker", justify="center")
    table.add_column("Value", justify="center")
    table.add_column("P&L", justify="center")
    prtf = portfolio.get_portfolio()
    if total:
        prtf = portfolio.get_total_portfolio_value()
    for element in prtf:
        ticker = element['ticker']
        price = element['price']
        profit_loss = element['p&l']
        ticker_t = f'[bold blue]{ticker}[/bold blue]'
        price_t = f'[bold white]{price}[/bold white]'
        conditional_profit_loss_t = f'[bold green]▲ {profit_loss}%[/bold green]' \
            if profit_loss >= 0 else f'[bold red]▼ {profit_loss}%[/bold red]'
        table.add_row(ticker_t, price_t, conditional_profit_loss_t)
    printp(table)

def prettier_info(ticker: tk.Ticker, info: Optional[str], attribute: Optional[str]):
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
        attr_str = DEFAULT_ATTRIBUTE if attribute is not None else attribute
        attr = str(ticker.get_ticker_fast_info()[attr_str])
        printp(f"{attribute} for [bold green]{ticker.ticker}[/bold green] is [bold]{attr}[/bold]")
